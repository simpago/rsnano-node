use crate::command_handler::RpcCommandHandler;
use rsnano_core::{BlockDetails, DifficultyV1};
use rsnano_rpc_messages::{WorkValidateArgs, WorkValidateResponse};

impl RpcCommandHandler {
    pub(crate) fn work_validate(&self, args: WorkValidateArgs) -> WorkValidateResponse {
        let default_difficulty = self.node.network_params.work.threshold_base();

        let difficulty = if let Some(multiplier) = args.multiplier {
            DifficultyV1::from_multiplier(multiplier.inner(), default_difficulty)
        } else {
            default_difficulty
        };

        /* Transition to epoch_2 difficulty levels breaks previous behavior.
         * When difficulty is not given, the default difficulty to validate changes when the first epoch_2 block is seen, breaking previous behavior.
         * For this reason, when difficulty is not given, the "valid" field is no longer included in the response to break loudly any client expecting it.
         * Instead, use the new fields:
         * * valid_all: the work is valid at the current highest difficulty threshold
         * * valid_receive: the work is valid for a receive block in an epoch_2 upgraded account
         */

        let result_difficulty = self
            .node
            .network_params
            .work
            .difficulty(&args.hash.into(), args.work.unwrap_or_default().into());

        let valid = if args.difficulty.is_some() {
            if result_difficulty >= difficulty {
                Some("1".to_owned())
            } else {
                Some("0".to_owned())
            }
        } else {
            None
        };

        let valid_all = if result_difficulty >= default_difficulty {
            "1".to_owned()
        } else {
            "0".to_owned()
        };

        let receive_difficulty = self.node.network_params.work.threshold(&BlockDetails::new(
            rsnano_core::Epoch::Epoch2,
            false,
            true,
            false,
        ));
        let valid_receive = if result_difficulty >= receive_difficulty {
            "1".to_owned()
        } else {
            "0".to_owned()
        };

        let result_multiplier = DifficultyV1::to_multiplier(result_difficulty, default_difficulty);

        WorkValidateResponse {
            valid,
            valid_all,
            valid_receive,
            difficulty: result_difficulty.into(),
            multiplier: result_multiplier.into(),
        }
    }
}
