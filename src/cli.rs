use clap::{Args, Parser, Subcommand};

use super::*;

pub use command::ApiResource;
pub use command::ApiResources;
pub use command::Auth;
pub use command::Command;
pub use command::Config;
pub use command::Get;
pub use command::Node;

mod command;

#[derive(Debug, Parser)]
pub struct Cli {
    #[arg(short, long, value_enum, global = true)]
    pub output: Option<OutputFormat>,

    // #[arg(flatten, global = true)]
    #[command(flatten)]
    pub options: GlobalOptions,

    /// Debug on/off
    #[arg(short, long, global = true)]
    pub debug: bool,

    /// If present, list the requested object(s) across all namespaces.
    /// Namespace in current context is ignored even if specified with --namespace.
    #[arg(short = 'A', long, global = true)]
    pub all_namespaces: bool,

    /// If present, the namespace scope for this CLI request
    #[arg(short = 'n', long, global = true)]
    pub namespace: Option<String>,

    /// The name of the kubeconfig cluster to use
    #[arg(long, global = true)]
    pub cluster: Option<String>,

    /// The name of the kubeconfig context to use
    #[arg(long, global = true)]
    pub context: Option<String>,

    /// The name of the kubeconfig user to use
    #[arg(long, global = true)]
    pub user: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

impl Cli {
    pub fn new() -> Self {
        Self::parse()
    }

    pub fn inspect(self) -> Self {
        tracing::debug!(cli = ?self, "rk");
        self
    }

    pub async fn exec(self) -> kube::Result<()> {
        let kubectl = self.kubectl().await?;
        self.command.exec(&kubectl).await
    }

    async fn kubectl(&self) -> kube::Result<Kubectl> {
        let namespace: Namespace = self.namespace();
        let output = self.output.unwrap_or_default();
        let kubectl = Kubectl::new(
            self.context.as_deref(),
            self.cluster.as_deref(),
            self.user.as_deref(),
            self.debug,
            &self.options,
        )
        .await
        .inspect(|kubectl| tracing::info!(?kubectl))?
        .with_namespace(namespace)
        .with_output(output);

        Ok(kubectl)
    }

    fn namespace(&self) -> Namespace {
        Namespace::new(self.all_namespaces, self.namespace.as_deref())
    }
}

impl Default for Cli {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Args)]
pub struct GlobalOptions {
    /// Default cache directory
    #[arg(long, global = true)]
    pub cache_dir: Option<PathBuf>,

    /// Username to impersonate for the operation. User could be a regular user or a service account in a namespace.
    #[arg(long = "as", global = true)]
    pub as_user: Option<String>,

    /// Group to impersonate for the operation, this flag can be repeated to specify multiple groups.
    #[arg(long, global = true)]
    pub as_group: Option<Vec<String>>,

    /// UID to impersonate for the operation.
    #[arg(long, global = true)]
    pub as_uid: Option<String>,
}

impl GlobalOptions {
    pub fn cache_dir(&self) -> PathBuf {
        self.cache_dir
            .clone()
            .or_else(|| etcetera::home_dir().ok())
            .unwrap_or_default()
            .join(".kube")
            .join("cache")
    }

    pub fn discovery_cache_for_config(
        &self,
        config: &kube::Config,
    ) -> Result<PathBuf, kube::config::KubeconfigError> {
        config
            .cluster_url
            .host()
            .map(|server| self.cache_dir().join("discovery").join(server))
            .ok_or(kube::config::KubeconfigError::MissingClusterUrl)
    }
}

#[expect(dead_code)]
fn metadata(name: impl ToString) -> metav1::ObjectMeta {
    let name = Some(name.to_string());
    metav1::ObjectMeta {
        name,
        // annotations: todo!(),
        // creation_timestamp: todo!(),
        // deletion_grace_period_seconds: todo!(),
        // deletion_timestamp: todo!(),
        // finalizers: todo!(),
        // generate_name: todo!(),
        // generation: todo!(),
        // labels: todo!(),
        // managed_fields: todo!(),
        // namespace: todo!(),
        // owner_references: todo!(),
        // resource_version: todo!(),
        // self_link: todo!(),
        // uid: todo!(),
        ..default()
    }
}
