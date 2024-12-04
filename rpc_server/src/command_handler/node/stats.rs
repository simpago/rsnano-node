use crate::command_handler::RpcCommandHandler;
use rsnano_core::utils::ContainerInfo;
use rsnano_node::stats::StatsJsonWriterV2;
use rsnano_rpc_messages::{StatsArgs, StatsType, SuccessResponse};

impl RpcCommandHandler {
    pub(crate) fn stats(&self, args: StatsArgs) -> anyhow::Result<serde_json::Value> {
        let mut sink = StatsJsonWriterV2::new();
        match args.stats_type {
            StatsType::Counters => {
                self.node.stats.log_counters(&mut sink).unwrap();
                sink.add(
                    "stat_duration_seconds",
                    self.node.stats.last_reset().as_secs(),
                );
                Ok(sink.finish())
            }
            StatsType::Samples => {
                self.node.stats.log_samples(&mut sink).unwrap();
                sink.add(
                    "stat_duration_seconds",
                    self.node.stats.last_reset().as_secs(),
                );
                Ok(sink.finish())
            }
            StatsType::Database => Ok(serde_json::to_value(self.node.store.memory_stats()?)?),
            StatsType::Objects => Ok(ContainerInfo::builder()
                .node("node", self.node.container_info())
                .finish()
                .into_json()),
        }
    }

    pub(crate) fn stats_clear(&self) -> SuccessResponse {
        self.node.stats.clear();
        SuccessResponse::new()
    }
}
