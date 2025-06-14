use super::*;

pub use named::NamedResource;

mod named;

#[derive(Clone, Debug, PartialEq)]
pub enum ResourceArg {
    Resource(Resource),
    NamedResource(NamedResource),
}

impl ResourceArg {
    pub fn from_strings(resources: &[String]) -> Result<Vec<Self>, InvalidResourceSpec> {
        // Two possible formats
        // 1. resource/name - in which case all the items should be the same
        // 2. resource[,resource,..] [name] [..]
        if resources.iter().any(|resource| resource.contains('/')) {
            resources.iter().map(Self::named_resource).collect()
        } else {
            let (resource, names) = resources.split_first().ok_or(InvalidResourceSpec)?;
            let resources = resource.split(",").map(Resource::from).collect::<Vec<_>>();
            let resources = if names.is_empty() {
                // Just resources, no names
                resources.into_iter().map(ResourceArg::Resource).collect()
            } else {
                resources
                    .into_iter()
                    .flat_map(|resource| {
                        names
                            .iter()
                            .map(move |name| NamedResource::with_resource(resource.clone(), name))
                    })
                    .map(Self::NamedResource)
                    .collect()
            };
            Ok(resources)
        }
    }

    fn named_resource(text: impl AsRef<str>) -> Result<Self, InvalidResourceSpec> {
        text.as_ref()
            .split_once("/")
            .ok_or(InvalidResourceSpec)
            .map(|(resource, name)| NamedResource::new(resource, name))
            .map(Self::NamedResource)
    }

    pub async fn _get(&self, kubectl: &Kubectl) -> kube::Result<Vec<api::DynamicObject>> {
        match self {
            Self::Resource(resource) => resource._get(kubectl).await,
            Self::NamedResource(named_resource) => named_resource._get(kubectl).await,
        }
    }

    pub async fn get(&self, kubectl: &Kubectl) -> kube::Result<Box<dyn Show>> {
        match self {
            Self::Resource(resource) => resource.list(kubectl).await,
            Self::NamedResource(named_resource) => {
                named_resource
                    .resource()
                    .get(kubectl, named_resource.name())
                    .await
            }
        }
    }

    pub fn resource(&self) -> &Resource {
        match self {
            Self::Resource(resource) => resource,
            Self::NamedResource(named_resource) => named_resource.resource(),
        }
    }

    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Resource(_resource) => None,
            Self::NamedResource(named_resource) => Some(named_resource.name()),
        }
    }
}

impl std::str::FromStr for ResourceArg {
    type Err = InvalidResourceSpec;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.contains("/") {
            Self::named_resource(text)
        } else {
            Ok(Self::Resource(Resource::from(text)))
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Resource {
    Pods,
    Nodes,
    ConfigMaps,
    ComponentStatuses,
    Other(String),
}

impl Resource {
    pub fn well_known(text: &str) -> Option<Self> {
        match text {
            "po" | "pod" | "pods" => Some(Self::Pods),
            "no" | "node" | "nodes" => Some(Self::Nodes),
            "cm" | "configmap" | "configmaps" => Some(Self::ConfigMaps),
            "cs" | "componentstatus" | "componentstatuses" => Some(Self::ComponentStatuses),
            _ => None,
        }
    }

    async fn _get(&self, kubectl: &Kubectl) -> kube::Result<Vec<api::DynamicObject>> {
        let lp = kubectl.list_params();
        let items = self.api(kubectl).await?.list(&lp).await?.items;
        Ok(items)
    }

    async fn list(&self, kubectl: &Kubectl) -> kube::Result<Box<dyn Show>> {
        let lp = kubectl.list_params();
        match self {
            Self::Pods => {
                let list = kubectl.pods()?.list(&lp).await?;
                Ok(Box::new(list))
            }
            Self::Nodes => {
                let list = kubectl.nodes()?.list(&lp).await?;
                Ok(Box::new(list))
            }
            Self::ConfigMaps => {
                let list = kubectl.configmaps()?.list(&lp).await?;
                Ok(Box::new(list))
            }
            Self::ComponentStatuses => {
                let list = kubectl.componentstatuses()?.list(&lp).await?;
                Ok(Box::new(list))
            }
            Self::Other(name) => {
                todo!("list not implemented yet for {name}")
            }
        }
    }

    async fn get(&self, kubectl: &Kubectl, name: &str) -> kube::Result<Box<dyn Show>> {
        match self {
            Self::Pods => {
                let obj = kubectl.pods()?.get(name).await?;
                Ok(Box::new(obj))
            }
            Self::Nodes => {
                let obj = kubectl.nodes()?.get(name).await?;
                Ok(Box::new(obj))
            }
            Self::ConfigMaps => {
                let obj = kubectl.configmaps()?.get(name).await?;
                Ok(Box::new(obj))
            }
            Self::ComponentStatuses => {
                let obj = kubectl.componentstatuses()?.get(name).await?;
                Ok(Box::new(obj))
            }
            Self::Other(name) => {
                todo!("get not implemented yet for {name}")
            }
        }
    }

    pub async fn api_resource(&self, kubectl: &Kubectl) -> kube::Result<Option<api::ApiResource>> {
        match self {
            Self::Pods => Ok(Some(Self::erase::<corev1::Pod>())),
            Self::Nodes => Ok(Some(Self::erase::<corev1::Node>())),
            Self::ConfigMaps => Ok(Some(Self::erase::<corev1::ConfigMap>())),
            Self::ComponentStatuses => Ok(Some(Self::erase::<corev1::ComponentStatus>())),
            Self::Other(name) => self.dynamic_api_resource(kubectl, name).await,
        }
    }

    async fn api(&self, kubectl: &Kubectl) -> kube::Result<api::Api<api::DynamicObject>> {
        let ar = self
            .api_resource(kubectl)
            .await?
            .ok_or(kube::Error::LinesCodecMaxLineLengthExceeded)?;
        let api = kubectl.dynamic_api(&ar);
        Ok(api)
    }

    async fn dynamic_api_resource(
        &self,
        kubectl: &Kubectl,
        name: &str,
    ) -> kube::Result<Option<api::ApiResource>> {
        let ar = kubectl
            .server_api_resources()
            .await?
            .into_iter()
            .find_map(|arl| arl.kube_api_resource(name));
        Ok(ar)
    }

    fn erase<K>() -> api::ApiResource
    where
        K: kube::Resource,
        <K as kube::Resource>::DynamicType: Default,
    {
        api::ApiResource::erase::<K>(&<K as kube::Resource>::DynamicType::default())
    }

    fn other(other: impl ToString) -> Self {
        Self::Other(other.to_string())
    }
}

impl From<String> for Resource {
    fn from(text: String) -> Self {
        Self::well_known(&text).unwrap_or_else(|| Self::other(text))
    }
}

impl From<&str> for Resource {
    fn from(text: &str) -> Self {
        Self::well_known(text).unwrap_or_else(|| Self::other(text))
    }
}

#[derive(Debug, thiserror::Error)]
#[error(
    "there is no need to specify a resource type as a separate argument when passing arguments in resource/name form (e.g. 'kubectl get resource/<resource_name>' instead of 'kubectl get resource resource/<resource_name>')"
)]
pub struct InvalidResourceSpec;

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &[&str]) -> Result<Vec<ResourceArg>, InvalidResourceSpec> {
        let resources = s.iter().map(ToString::to_string).collect::<Vec<_>>();
        ResourceArg::from_strings(&resources)
    }

    #[test]
    fn one_resource() {
        let resources = args(&["pod"]).unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0], ResourceArg::Resource(Resource::Pods));
    }

    #[test]
    fn many_resources() {
        let resources = args(&["pod,node"]).unwrap();
        assert_eq!(resources.len(), 2);

        let ResourceArg::Resource(ref pod) = resources[0] else {
            panic!("expecting NamedResource, found something else");
        };
        let ResourceArg::Resource(ref node) = resources[1] else {
            panic!("expecting NamedResource, found something else");
        };

        assert_eq!(pod, &Resource::Pods);
        assert_eq!(node, &Resource::Nodes);
    }

    #[test]
    fn resource_and_name() {
        let resources = args(&["pod", "bazooka"]).unwrap();
        assert_eq!(resources.len(), 1);
        let ResourceArg::NamedResource(ref pod) = resources[0] else {
            panic!("expecting NamedResource, found something else");
        };
        assert_eq!(pod.resource(), &Resource::Pods);
        assert_eq!(pod.name(), "bazooka");
    }

    #[test]
    fn resource_and_many_name() {
        let resources = args(&["pod", "bazooka", "darbooka"]).unwrap();
        assert_eq!(resources.len(), 2);
        let ResourceArg::NamedResource(ref pod1) = resources[0] else {
            panic!("expecting NamedResource, found something else");
        };
        let ResourceArg::NamedResource(ref pod2) = resources[1] else {
            panic!("expecting NamedResource, found something else");
        };
        assert_eq!(pod1.resource(), &Resource::Pods);
        assert_eq!(pod1.name(), "bazooka");
        assert_eq!(pod2.resource(), &Resource::Pods);
        assert_eq!(pod2.name(), "darbooka");
    }

    #[test]
    fn one_named_resource() {
        let resources = args(&["pod/bazooka"]).unwrap();
        assert_eq!(resources.len(), 1);
        let ResourceArg::NamedResource(ref pod) = resources[0] else {
            panic!("expecting NamedResource, found something else");
        };

        assert_eq!(pod.resource(), &Resource::Pods);
        assert_eq!(pod.name(), "bazooka");
    }

    #[test]
    fn many_named_resources() {
        let resources = args(&["pod/bazooka", "node/elephant"]).unwrap();
        assert_eq!(resources.len(), 2);

        let ResourceArg::NamedResource(ref pod) = resources[0] else {
            panic!("expecting NamedResource, found something else");
        };
        let ResourceArg::NamedResource(ref node) = resources[1] else {
            panic!("expecting NamedResource, found something else");
        };

        assert_eq!(pod.resource(), &Resource::Pods);
        assert_eq!(pod.name(), "bazooka");
        assert_eq!(node.resource(), &Resource::Nodes);
        assert_eq!(node.name(), "elephant");
    }

    #[test]
    fn invalid_mix() {
        let _err = args(&["pod/bazooka", "node"]).unwrap_err();
    }
}
