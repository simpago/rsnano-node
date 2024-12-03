use crate::{RpcBool, RpcCommand, RpcU16, RpcU32, RpcU64, RpcU8};
use rsnano_core::{to_hex_string, BlockHash, NodeId, Signature};
use rsnano_messages::TelemetryData;
use serde::{ser::SerializeMap, Deserialize, Deserializer, Serialize, Serializer};
use std::net::{Ipv6Addr, SocketAddrV6};

impl RpcCommand {
    pub fn telemetry(args: TelemetryArgs) -> Self {
        Self::Telemetry(args)
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct TelemetryArgs {
    pub raw: Option<RpcBool>,
    pub address: Option<Ipv6Addr>,
    pub port: Option<RpcU16>,
}

impl TelemetryArgs {
    pub fn new() -> TelemetryArgs {
        TelemetryArgs {
            raw: None,
            address: None,
            port: None,
        }
    }

    pub fn builder() -> TelemetryArgsBuilder {
        TelemetryArgsBuilder {
            args: TelemetryArgs::new(),
        }
    }
}

pub struct TelemetryArgsBuilder {
    args: TelemetryArgs,
}

impl TelemetryArgsBuilder {
    pub fn raw(mut self) -> Self {
        self.args.raw = Some(true.into());
        self
    }

    pub fn remote_addr(mut self, addr: SocketAddrV6) -> Self {
        self.args.address = Some(addr.ip().clone());
        self.args.port = Some(addr.port().into());
        self
    }

    pub fn build(self) -> TelemetryArgs {
        self.args
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct TelemetryDto {
    pub block_count: RpcU64,
    pub cemented_count: RpcU64,
    pub unchecked_count: RpcU64,
    pub account_count: RpcU64,
    pub bandwidth_cap: RpcU64,
    pub uptime: RpcU64,
    pub peer_count: RpcU32,
    pub protocol_version: RpcU8,
    pub genesis_block: BlockHash,
    pub major_version: RpcU8,
    pub minor_version: RpcU8,
    pub patch_version: RpcU8,
    pub pre_release_version: RpcU8,
    pub maker: RpcU8,
    pub timestamp: RpcU64,
    pub active_difficulty: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<Signature>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<NodeId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<Ipv6Addr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<RpcU16>,
}

impl From<TelemetryData> for TelemetryDto {
    fn from(data: TelemetryData) -> Self {
        Self {
            block_count: data.block_count.into(),
            cemented_count: data.cemented_count.into(),
            unchecked_count: data.unchecked_count.into(),
            account_count: data.account_count.into(),
            bandwidth_cap: data.bandwidth_cap.into(),
            uptime: data.uptime.into(),
            peer_count: data.peer_count.into(),
            protocol_version: data.protocol_version.into(),
            genesis_block: data.genesis_block.into(),
            major_version: data.major_version.into(),
            minor_version: data.minor_version.into(),
            patch_version: data.patch_version.into(),
            pre_release_version: data.pre_release_version.into(),
            maker: data.maker.into(),
            timestamp: data
                .timestamp
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .into(),
            active_difficulty: to_hex_string(data.active_difficulty),
            signature: Some(data.signature),
            node_id: Some(data.node_id),
            address: None,
            port: None,
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct TelemetryResponse {
    pub metrics: Vec<TelemetryDto>,
}

impl Serialize for TelemetryResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.metrics.len() == 1 {
            self.metrics[0].serialize(serializer)
        } else {
            let mut map = serializer.serialize_map(Some(1))?;
            map.serialize_entry("metrics", &self.metrics)?;
            map.end()
        }
    }
}

impl<'de> Deserialize<'de> for TelemetryResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum TelemetryDtoHelper {
            Single(TelemetryDto),
            Multiple { metrics: Vec<TelemetryDto> },
        }

        let helper = TelemetryDtoHelper::deserialize(deserializer)?;
        match helper {
            TelemetryDtoHelper::Single(data) => Ok(TelemetryResponse {
                metrics: vec![data],
            }),
            TelemetryDtoHelper::Multiple { metrics } => Ok(TelemetryResponse { metrics }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsnano_core::{to_hex_string, BlockHash, Signature};
    use rsnano_messages::TelemetryData;
    use std::time::UNIX_EPOCH;

    fn create_test_telemetry_data() -> TelemetryData {
        TelemetryData {
            signature: Signature::new(),
            node_id: NodeId::ZERO,
            block_count: 1000,
            cemented_count: 900,
            unchecked_count: 100,
            account_count: 500,
            bandwidth_cap: 1024,
            uptime: 3600,
            peer_count: 10,
            protocol_version: 18,
            genesis_block: BlockHash::zero(),
            major_version: 23,
            minor_version: 3,
            patch_version: 0,
            pre_release_version: 0,
            maker: 3,
            timestamp: UNIX_EPOCH + std::time::Duration::from_secs(1623456789),
            active_difficulty: 0xFFFFFFFFFFFFFFFF,
            unknown_data: vec![],
        }
    }

    #[test]
    fn test_telemetry_dto_serialize_single() {
        let data = create_test_telemetry_data();
        let dto = TelemetryDto::from(data);
        let dtos = TelemetryResponse {
            metrics: vec![dto.clone()],
        };

        let serialized = serde_json::to_string(&dtos).unwrap();
        let deserialized: TelemetryResponse = serde_json::from_str(&serialized).unwrap();

        assert_eq!(dtos.metrics, deserialized.metrics);
    }

    #[test]
    fn test_telemetry_dto_serialize_multiple() {
        let data1 = create_test_telemetry_data();
        let mut data2 = data1.clone();
        data2.block_count = 2000;

        let dto1 = TelemetryDto::from(data1);
        let dto2 = TelemetryDto::from(data2);
        let dtos = TelemetryResponse {
            metrics: vec![dto1.clone(), dto2.clone()],
        };

        let serialized = serde_json::to_string(&dtos).unwrap();
        let deserialized: TelemetryResponse = serde_json::from_str(&serialized).unwrap();

        assert_eq!(dtos.metrics, deserialized.metrics);
    }

    #[test]
    fn test_telemetry_dto_deserialize_single() {
        let data = create_test_telemetry_data();
        let dto = TelemetryDto::from(data);
        let json = serde_json::to_string(&dto).unwrap();

        let deserialized: TelemetryResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.metrics.len(), 1);
        assert_eq!(dto, deserialized.metrics[0]);
    }

    #[test]
    fn test_telemetry_dto_deserialize_multiple() {
        let data1 = create_test_telemetry_data();
        let mut data2 = data1.clone();
        data2.block_count = 2000;

        let dto1 = TelemetryDto::from(data1);
        let dto2 = TelemetryDto::from(data2);

        let json = format!(
            r#"{{"metrics":[{},{}]}}"#,
            serde_json::to_string(&dto1).unwrap(),
            serde_json::to_string(&dto2).unwrap()
        );

        let deserialized: TelemetryResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.metrics.len(), 2);
        assert_eq!(dto1, deserialized.metrics[0]);
        assert_eq!(dto2, deserialized.metrics[1]);
    }

    #[test]
    fn test_telemetry_data_to_dto_conversion() {
        let data = create_test_telemetry_data();
        let dto: TelemetryDto = data.clone().into();

        assert_eq!(dto.block_count, data.block_count.into());
        assert_eq!(dto.cemented_count, data.cemented_count.into());
        assert_eq!(dto.unchecked_count, data.unchecked_count.into());
        assert_eq!(dto.account_count, data.account_count.into());
        assert_eq!(dto.bandwidth_cap, data.bandwidth_cap.into());
        assert_eq!(dto.uptime, data.uptime.into());
        assert_eq!(dto.peer_count, data.peer_count.into());
        assert_eq!(dto.protocol_version, data.protocol_version.into());
        assert_eq!(dto.genesis_block, data.genesis_block.into());
        assert_eq!(dto.major_version, data.major_version.into());
        assert_eq!(dto.minor_version, data.minor_version.into());
        assert_eq!(dto.patch_version, data.patch_version.into());
        assert_eq!(dto.pre_release_version, data.pre_release_version.into());
        assert_eq!(dto.maker, data.maker.into());
        assert_eq!(dto.timestamp, 1623456789.into());
        assert_eq!(dto.active_difficulty, to_hex_string(data.active_difficulty));
        assert_eq!(dto.signature, Some(data.signature));
        assert_eq!(dto.node_id, Some(data.node_id));
        assert_eq!(dto.address, None);
        assert_eq!(dto.port, None);
    }
}
