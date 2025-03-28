use super::*;

impl Show for corev1::ConfigMap {
    fn header(&self, output: &Output) -> Vec<String> {
        let header = match output {
            Output::Normal => ["NAMESPACE", "NAME", "DATA", "AGE"].as_slice(),
            Output::Wide => ["NAMESPACE", "NAME", "DATA", "AGE"].as_slice(),
            _ => todo!("{output:?}"),
        };
        header.iter().map(ToString::to_string).collect()
    }

    fn data(&self, params: &ShowParams, output: &Output) -> Vec<String> {
        let namespace = self.namespace().unwrap_or_default();
        let name = name(self, params);
        let data = self
            .data
            .as_ref()
            .map(|data| data.len())
            .unwrap_or_default();
        let binary_data = self
            .binary_data
            .as_ref()
            .map(|data| data.len())
            .unwrap_or_default();
        let data = format!("{}", data + binary_data);
        let age = self.creation_timestamp().map(age).unwrap_or_default();
        match output {
            Output::Normal => vec![namespace, name, data, age],
            Output::Wide => vec![namespace, name, data, age],
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
