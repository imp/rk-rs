use super::*;

impl Show for corev1::Node {
    fn header(&self, output: &OutputFormat) -> Vec<String> {
        let header = match output {
            OutputFormat::Normal => ["NAMESPACE", "NAME"].as_slice(),
            OutputFormat::Wide => ["NAMESPACE", "NAME", "AGE"].as_slice(),
            _ => todo!("{output:?}"),
        };
        header.iter().map(ToString::to_string).collect()
    }

    fn data(&self, params: &ShowParams, output: &OutputFormat) -> Vec<String> {
        let namespace = self.namespace().unwrap_or_default();
        let name = name(self, params);
        let age = self.creation_timestamp().map(age).unwrap_or_default();
        match output {
            OutputFormat::Normal => vec![namespace, name],
            OutputFormat::Wide => vec![namespace, name, age],
            _ => todo!("{output:?}"),
        }
    }

    fn yaml(&self) -> String {
        todo!()
    }

    fn json(&self) -> String {
        todo!()
    }

    fn name(&self) -> String {
        todo!()
    }
}
