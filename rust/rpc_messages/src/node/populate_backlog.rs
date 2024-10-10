use crate::RpcCommand;

impl RpcCommand {
    pub fn populate_backlog() -> Self {
        Self::PopulateBacklog
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_populate_backlog() {
        let command = RpcCommand::populate_backlog();
        let serialized = serde_json::to_string(&command).unwrap();
        assert_eq!(serialized, r#"{"action":"populate_backlog"}"#);
    }

    #[test]
    fn deserialize_populate_backlog() {
        let json = r#"{"action":"populate_backlog"}"#;
        let deserialized: RpcCommand = serde_json::from_str(json).unwrap();
        assert!(matches!(deserialized, RpcCommand::PopulateBacklog));
    }
}
