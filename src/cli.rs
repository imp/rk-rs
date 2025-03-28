use clap::{Args, Parser, Subcommand};

use super::*;

pub use command::Command;

mod command;

#[derive(Debug, Parser)]
pub struct Cli {
    #[arg(short, long, value_enum, global = true)]
    pub output: Option<OutputArg>,

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

    #[command(subcommand)]
    pub command: Command,
}

impl Cli {
    pub fn new() -> Self {
        Self::parse()
    }

    pub async fn exec(self, kubectl: Kubectl) -> kube::Result<()> {
        let output = self.output.unwrap_or_default();
        let namespace = Namespace::new(self.all_namespaces, self.namespace);
        let kubectl = kubectl.namespace(namespace);
        match self.command {
            Command::ApiResources(api_resources) => api_resources.exec(&kubectl, output).await,
            Command::ApiVersions => kubectl.api_versions().await,
            Command::Get { resources } => Self::get(&kubectl, resources, output).await,
        }
    }

    async fn get(kubectl: &Kubectl, resources: Vec<String>, output: OutputArg) -> kube::Result<()> {
        println!("{resources:?}");
        let resources = ResourceArg::from_strings(resources)
            .map_err(|_err| kube::Error::LinesCodecMaxLineLengthExceeded)?;
        println!("{resources:?}");
        let namespace = kubectl.show_namespace();
        let full_name = resources.len() > 1;
        for resource in resources {
            let data = resource.get(kubectl).await?;
            println!("{}", data.output(namespace, full_name, &output));
        }
        Ok(())
    }
}

impl Default for Cli {
    fn default() -> Self {
        Self::new()
    }
}
