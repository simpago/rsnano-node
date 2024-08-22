use super::{ChannelId, NetworkInfo};
use rsnano_nullable_clock::SteadyClock;
use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

pub(crate) trait DeadChannelCleanupTarget {
    fn dead_channel_cleanup_step(&self) -> Box<dyn DeadChannelCleanupStep>;
}

pub(crate) trait DeadChannelCleanupStep: Send {
    fn clean_up_dead_channels(&self, dead_channel_ids: &[ChannelId]);
}

// Removes dead channels and all their related queue entries
pub(crate) struct DeadChannelCleanup {
    clock: Arc<SteadyClock>,
    network: Arc<RwLock<NetworkInfo>>,
    cleanup_cutoff: Duration,
    cleanup_steps: Vec<Box<dyn DeadChannelCleanupStep>>,
}

impl DeadChannelCleanup {
    pub(crate) fn new(
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

    pub(crate) fn add(&mut self, target: &impl DeadChannelCleanupTarget) {
        self.add_step(target.dead_channel_cleanup_step());
    }

    pub(crate) fn add_step(&mut self, step: Box<dyn DeadChannelCleanupStep>) {
        self.cleanup_steps.push(step);
    }

    pub(crate) fn clean_up(&self) {
        let channel_ids = self
            .network
            .write()
            .unwrap()
            .purge(self.clock.now(), self.cleanup_cutoff);
        for step in &self.cleanup_steps {
            step.clean_up_dead_channels(&channel_ids);
        }
    }
}
