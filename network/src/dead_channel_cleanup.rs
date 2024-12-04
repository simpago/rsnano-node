use crate::{ChannelId, NetworkInfo};
use rsnano_nullable_clock::SteadyClock;
use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

pub trait DeadChannelCleanupStep: Send {
    fn clean_up_dead_channels(&self, dead_channel_ids: &[ChannelId]);
}

// Removes dead channels and all their related queue entries
pub struct DeadChannelCleanup {
    clock: Arc<SteadyClock>,
    network: Arc<RwLock<NetworkInfo>>,
    cleanup_cutoff: Duration,
    cleanup_steps: Vec<Box<dyn DeadChannelCleanupStep>>,
}

impl DeadChannelCleanup {
    pub fn new(
        clock: Arc<SteadyClock>,
        network: Arc<RwLock<NetworkInfo>>,
        cleanup_cutoff: Duration,
    ) -> Self {
        Self {
            clock,
            network,
            cleanup_cutoff,
            cleanup_steps: Vec::new(),
        }
    }

    pub fn add_step(&mut self, step: impl DeadChannelCleanupStep + 'static) {
        self.cleanup_steps.push(Box::new(step));
    }

    pub fn clean_up(&self) {
        let removed_channels = self
            .network
            .write()
            .unwrap()
            .purge(self.clock.now(), self.cleanup_cutoff);

        let channel_ids: Vec<_> = removed_channels.iter().map(|c| c.channel_id()).collect();

        for step in &self.cleanup_steps {
            step.clean_up_dead_channels(&channel_ids);
        }
    }
}
