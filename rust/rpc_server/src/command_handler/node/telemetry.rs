use crate::command_handler::RpcCommandHandler;
use anyhow::anyhow;
use rsnano_rpc_messages::{TelemetryArgs, TelemetryDto, TelemetryDtos};
use std::net::SocketAddrV6;

impl RpcCommandHandler {
    pub(crate) fn telemetry(&self, args: TelemetryArgs) -> anyhow::Result<TelemetryDtos> {
        if let (Some(address), Some(port)) = (args.address, args.port) {
            let endpoint = SocketAddrV6::new(address, port, 0, 0);

            if address.is_loopback() && port == self.node.network.port() {
                Ok(TelemetryDtos {
                    metrics: vec![self.node.telemetry.local_telemetry().into()],
                })
            } else {
                match self.node.telemetry.get_telemetry(&endpoint.into()) {
                    Some(data) => Ok(TelemetryDtos {
                        metrics: vec![data.into()],
                    }),
                    None => Err(anyhow!("Peer not found")),
                }
            }
        } else {
            let output_raw = args.raw.unwrap_or(false);

            if output_raw {
                let all_telemetries = self.node.telemetry.get_all_telemetries();
                let metrics: Vec<TelemetryDto> = all_telemetries
                    .iter()
                    .map(|(endpoint, telemetry)| {
                        let mut dto: TelemetryDto = telemetry.clone().into();
                        dto.address = Some(*endpoint.ip());
                        dto.port = Some(endpoint.port());
                        dto
                    })
                    .collect();

                Ok(TelemetryDtos { metrics })
            } else {
                Ok(TelemetryDtos {
                    metrics: vec![self.node.telemetry.local_telemetry().into()],
                })
            }
        }
    }
}