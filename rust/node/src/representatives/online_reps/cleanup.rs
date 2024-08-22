use super::OnlineReps;
use crate::transport::{ChannelId, DeadChannelCleanupStep, DeadChannelCleanupTarget};
use rsnano_core::Account;
use std::sync::{Arc, Mutex};
use tracing::info;

impl DeadChannelCleanupTarget for Arc<Mutex<OnlineReps>> {
    fn dead_channel_cleanup_step(&self) -> Box<dyn DeadChannelCleanupStep> {
        Box::new(OnlineRepsCleanup(self.clone()))
    }
}

/// Removes reps with dead channels
pub(crate) struct OnlineRepsCleanup(Arc<Mutex<OnlineReps>>);

impl DeadChannelCleanupStep for OnlineRepsCleanup {
    fn clean_up_dead_channels(&self, dead_channel_ids: &[ChannelId]) {
        let mut online_reps = self.0.lock().unwrap();
        for channel_id in dead_channel_ids {
            let removed_reps = online_reps.remove_peer(*channel_id);
            for rep in removed_reps {
                info!(
                    "Evicting representative {} with dead channel",
                    Account::from(rep).encode_account(),
                );
            }
        }
    }
}
