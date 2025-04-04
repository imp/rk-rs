use super::*;

pub struct Kubectl {
    client: kube::Client,
    namespace: Namespace,
    debug: bool,
}

impl Kubectl {
    pub async fn new(debug: bool) -> kube::Result<Self> {
        let namespace = default();
        kube::Client::try_default().await.map(|client| Self {
            client,
            namespace,
            debug,
        })
    }

    pub fn namespace(self, namespace: Namespace) -> Self {
        Self { namespace, ..self }
    }

    pub fn show_namespace(&self) -> bool {
        matches!(self.namespace, Namespace::All)
    }

    pub async fn get_core_api_resources(&self) -> kube::Result<Vec<metav1::APIResourceList>> {
        let versions = self.client.list_core_api_versions().await?.versions;
        let mut list = Vec::with_capacity(versions.len());
        for version in versions {
            let arl = self.client.list_core_api_resources(&version).await?;
            list.push(arl)
        }

        Ok(list)
    }

    pub async fn get_api_resources(&self) -> kube::Result<Vec<metav1::APIResourceList>> {
        let groups = self.client.list_api_groups().await?.groups;
        let mut list = Vec::new();
        for group in groups {
            let apiversion = group
                .preferred_version
                .as_ref()
                .or_else(|| group.versions.first());
            if let Some(apiversion) = apiversion {
                let arl = self
                    .client
                    .list_api_group_resources(&apiversion.group_version)
                    .await?;
                list.push(arl);
            } else {
                continue;
            }
        }

        Ok(list)
    }

    pub async fn api_versions(&self) -> kube::Result<()> {
        let core = self.list_core_api_versions().await?;
        let groups = self.list_api_groups().await?;
        core.versions
            .into_iter()
            .for_each(|version| println!("{version}"));
        groups
            .groups
            .iter()
            .flat_map(|group| group.versions.iter())
            .for_each(|version| println!("{}", version.group_version));
        Ok(())
    }

    pub fn dynamic_api(&self, resource: &api::ApiResource) -> api::Api<api::DynamicObject> {
        println!("{resource:?}");
        let client = self.client.clone();
        match &self.namespace {
            Namespace::All => api::Api::all_with(client, resource),
            Namespace::Default => api::Api::default_namespaced_with(client, resource),
            Namespace::Namespace(ns) => api::Api::namespaced_with(client, ns, resource),
        }
    }

    pub async fn get(&self, resource: Vec<Resource>, output: OutputFormat) -> kube::Result<()> {
        println!("Getting {resource:?} [{output:?}]");
        Ok(())
    }

    pub fn list_params(&self) -> api::ListParams {
        self.client.list_params()
    }

    pub fn pods(&self) -> api::Api<corev1::Pod> {
        self.namespaced_api()
    }

    pub fn configmaps(&self) -> api::Api<corev1::ConfigMap> {
        self.namespaced_api()
    }

    pub fn nodes(&self) -> api::Api<corev1::Node> {
        self.cluster_api()
    }

    fn cluster_api<K>(&self) -> api::Api<K>
    where
        K: kube::Resource<Scope = k8s::openapi::ClusterResourceScope>,
        <K as kube::Resource>::DynamicType: Default,
    {
        self.client.api()
    }

    fn namespaced_api<K>(&self) -> api::Api<K>
    where
        K: kube::Resource<Scope = k8s::openapi::NamespaceResourceScope>,
        <K as kube::Resource>::DynamicType: Default,
    {
        match &self.namespace {
            Namespace::All => self.client.api(),
            Namespace::Default => self.client.default_namespaced_api(),
            Namespace::Namespace(namespace) => self.client.namespaced_api(namespace),
        }
    }
}

impl std::fmt::Debug for Kubectl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Kubectl")
            .field("client", &"kube::Client")
            .field("namespace", &self.namespace)
            .field("debug", &self.debug)
            .finish()
    }
}

impl std::ops::Deref for Kubectl {
    type Target = kube::Client;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}
