use super::OnlineReps;
use rsnano_core::Account;
use rsnano_network::{ChannelId, DeadChannelCleanupStep};
use std::sync::{Arc, Mutex};
use tracing::info;

/// Removes reps with dead channels
pub struct OnlineRepsCleanup(Arc<Mutex<OnlineReps>>);

impl OnlineRepsCleanup {
    pub fn new(reps: Arc<Mutex<OnlineReps>>) -> Self {
        Self(reps)
    }
}

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
