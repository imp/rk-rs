use std::collections::BTreeSet;

use futures_util::stream::{self, StreamExt, TryStreamExt};

use super::*;

pub use cache::Cache;
pub use features::Feature;

mod cache;
mod features;
mod info;
mod kubeconfig;
mod version;

#[derive(Debug)]
pub struct Kubectl {
    config: kube::Config,
    kubeconfig: kube::config::Kubeconfig,
    cache: Cache,
    namespace: Namespace,
    output: OutputFormat,
    debug: bool,
    options: GlobalOptions,
}

impl Kubectl {
    pub fn _client(&self) -> kube::Result<kube::Client> {
        kube::Client::try_from(self.config.clone())
    }

    pub fn client(&self) -> kube::Result<kube::Client> {
        kube::Client::try_from(self.config.clone())
    }

    pub async fn new(
        context: Option<&str>,
        cluster: Option<&str>,
        user: Option<&str>,
        debug: bool,
        options: &GlobalOptions,
    ) -> kube::Result<Self> {
        let options = options.clone();
        let namespace = default();
        let output = default();
        let cache = cache::Cache::default();
        Self::kubeconfig(context, cluster, user, debug)
            .await
            .inspect_err(|err| tracing::error!(%err, "from_kubeconfig"))
            .map(|(config, kubeconfig)| Self {
                config,
                kubeconfig,
                cache,
                namespace,
                output,
                debug,
                options,
            })
            .and_then(Self::try_load_cache)
            .map_err(|_| kube::Error::LinesCodecMaxLineLengthExceeded)
    }

    fn cache_path(&self) -> Result<PathBuf, kube::config::KubeconfigError> {
        self.options.discovery_cache_for_config(&self.config)
    }

    fn try_load_cache(self) -> Result<Self, kube::config::KubeconfigError> {
        let path = self.cache_path()?;
        let cache = self.cache.try_load(path);
        if self.debug {
            println!("Loading cache took {:?}", cache.took());
        }
        Ok(Self { cache, ..self })
    }

    pub fn with_namespace(self, namespace: Namespace) -> Self {
        Self { namespace, ..self }
    }

    pub fn with_output(self, output: OutputFormat) -> Self {
        Self { output, ..self }
    }

    pub fn namespace(&self) -> &Namespace {
        &self.namespace
    }

    pub fn output(&self) -> &OutputFormat {
        &self.output
    }

    pub fn show_namespace(&self) -> bool {
        matches!(self.namespace, Namespace::All)
    }

    pub async fn server_api_resources(&self) -> kube::Result<Vec<metav1::APIResourceList>> {
        if let Some(resources) = self.cache.api_resources() {
            // resources.sort_by_key(|arl| arl.resources[0].group.as_deref());
            Ok(resources)
        } else {
            self.get_server_api_resources().await
        }
    }

    pub async fn server_api_groups(&self) -> kube::Result<metav1::APIGroupList> {
        if let Some(groups) = self.cache.api_groups() {
            Ok(groups)
        } else {
            self.get_server_api_groups().await
        }
    }

    pub async fn server_preferred_resources(&self) -> kube::Result<Vec<metav1::APIResourceList>> {
        let ag = self.server_api_groups().await?;
        let preferred_versions = ag
            .groups
            .into_iter()
            .map(|mut group| {
                group
                    .preferred_version
                    .unwrap_or_else(|| group.versions.remove(0))
            })
            .map(|gv| gv.group_version)
            .collect::<BTreeSet<_>>();
        let resources = self
            .server_api_resources()
            .await?
            .into_iter()
            .filter(|arl| preferred_versions.contains(&arl.group_version))
            .collect();
        Ok(resources)
    }

    async fn get_server_api_groups(&self) -> kube::Result<metav1::APIGroupList> {
        let client = self.client()?;
        let core = client.list_core_api_versions().await?;
        let name = kube::discovery::ApiGroup::CORE_GROUP.to_string();

        let versions = core
            .versions
            .into_iter()
            .map(|version| metav1::GroupVersionForDiscovery {
                group_version: format!("{name}{version}"),
                version,
            })
            .collect::<Vec<_>>();

        let core = metav1::APIGroup {
            name,
            preferred_version: Some(versions[0].clone()),
            server_address_by_client_cidrs: Some(core.server_address_by_client_cidrs),
            versions,
        };

        let mut groups = client.list_api_groups().await?;
        groups.groups.insert(0, core);
        Ok(groups)
    }

    async fn get_server_api_resources(&self) -> kube::Result<Vec<metav1::APIResourceList>> {
        let client = self.client()?;
        let core = client.list_core_api_versions().await?;
        let core = stream::iter(&core.versions)
            .then(|version| client.list_core_api_resources(version))
            .try_collect::<Vec<_>>()
            .await?;

        let groups = client.list_api_groups().await?.groups;
        let apiversions = groups.iter().filter_map(|group| {
            group
                .preferred_version
                .as_ref()
                .or_else(|| group.versions.first())
        });
        let groups = stream::iter(apiversions)
            .then(|apiversion| client.list_api_group_resources(&apiversion.group_version))
            .try_collect::<Vec<_>>()
            .await?;

        Ok(core.into_iter().chain(groups).collect())
    }

    pub async fn api_versions(&self) -> kube::Result<()> {
        self.server_api_groups()
            .await?
            .groups
            .iter()
            .flat_map(|group| group.versions.iter())
            .for_each(|version| println!("{}", version.group_version));
        Ok(())
    }

    pub fn dynamic_api(&self, resource: &api::ApiResource) -> api::Api<api::DynamicObject> {
        println!("{resource:?}");
        let client = self.client().unwrap();
        match &self.namespace {
            Namespace::All => api::Api::all_with(client, resource),
            Namespace::Default => api::Api::default_namespaced_with(client, resource),
            Namespace::Namespace(ns) => api::Api::namespaced_with(client, ns, resource),
        }
    }

    pub async fn raw(&self, name: &str) -> kube::Result<String> {
        let gp = self.get_params();
        let request = api::Request::new("")
            .get(name, &gp)
            .map_err(kube::Error::BuildRequest)?;
        self.client()?.request_text(request).await
    }

    pub async fn get(&self, resource: Vec<Resource>, output: OutputFormat) -> kube::Result<()> {
        println!("Getting {resource:?} [{output:?}]");
        Ok(())
    }

    pub fn get_params(&self) -> api::GetParams {
        api::GetParams::default()
    }

    pub fn list_params(&self) -> api::ListParams {
        api::ListParams::default()
    }

    pub fn post_params(&self) -> api::PostParams {
        api::PostParams::default()
    }

    pub fn pods(&self) -> kube::Result<api::Api<corev1::Pod>> {
        self.namespaced_api()
    }

    pub fn configmaps(&self) -> kube::Result<api::Api<corev1::ConfigMap>> {
        self.namespaced_api()
    }

    pub fn componentstatuses(&self) -> kube::Result<api::Api<corev1::ComponentStatus>> {
        self.cluster_api()
    }

    pub fn nodes(&self) -> kube::Result<api::Api<corev1::Node>> {
        self.cluster_api()
    }

    pub fn selfsubjectaccessreviews(
        &self,
    ) -> kube::Result<api::Api<authorizationv1::SelfSubjectAccessReview>> {
        self.cluster_api()
    }

    pub fn selfsubjectrulesreviews(
        &self,
    ) -> kube::Result<api::Api<authorizationv1::SelfSubjectRulesReview>> {
        self.cluster_api()
    }

    pub fn selfsubjectreviews(
        &self,
    ) -> kube::Result<api::Api<authenticationv1::SelfSubjectReview>> {
        self.cluster_api()
    }

    pub fn inspect<K>(&self, k: &K)
    where
        K: serde::Serialize,
    {
        if self.debug {
            let k = yaml::to_string(k).unwrap_or_default();
            println!("{k}");
        }
    }

    pub fn inspect_err(&self, err: &kube::Error) {
        if self.debug {
            println!("{err:?}");
        }
    }

    fn cluster_api<K>(&self) -> kube::Result<api::Api<K>>
    where
        K: kube::Resource<Scope = k8s::openapi::ClusterResourceScope>,
        <K as kube::Resource>::DynamicType: Default,
    {
        self.client().map(|client| client.api())
    }

    fn namespaced_api<K>(&self) -> kube::Result<api::Api<K>>
    where
        K: kube::Resource<Scope = k8s::openapi::NamespaceResourceScope>,
        <K as kube::Resource>::DynamicType: Default,
    {
        let client = self.client()?;
        let api = match &self.namespace {
            Namespace::All => client.api(),
            Namespace::Default => client.default_namespaced_api(),
            Namespace::Namespace(namespace) => client.namespaced_api(namespace),
        };
        Ok(api)
    }
}
