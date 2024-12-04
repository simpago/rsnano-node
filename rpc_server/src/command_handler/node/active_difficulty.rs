use crate::command_handler::RpcCommandHandler;
use rsnano_core::DifficultyV1;
use rsnano_rpc_messages::ActiveDifficultyResponse;

impl RpcCommandHandler {
    pub(crate) fn active_difficulty(&self) -> ActiveDifficultyResponse {
        let multiplier_active = 1.0;
        let default_difficutly = self.node.network_params.work.threshold_base();

        let default_receive_difficulty = self.node.network_params.work.epoch_2_receive;
        let receive_current_denormalized = self.node.network_params.work.denormalized_multiplier(
            multiplier_active,
            self.node.network_params.work.epoch_2_receive,
        );

        ActiveDifficultyResponse {
            deprecated: "1".to_owned(),
            network_minimum: default_difficutly.into(),
            network_receive_minimum: default_receive_difficulty.into(),
            network_current: DifficultyV1::from_multiplier(multiplier_active, default_difficutly)
                .into(),
            network_receive_current: DifficultyV1::from_multiplier(
                receive_current_denormalized,
                default_receive_difficulty,
            )
            .into(),
            multiplier: 1.0.into(),
            difficulty_trend: Some(1.0.into()),
        }
    }
}
