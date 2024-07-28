use super::{RepTier, RepTiers};
use crate::{
    config::VoteProcessorConfig,
    stats::{DetailType, StatType, Stats},
    transport::{ChannelEnum, FairQueue, Origin},
};
use rsnano_core::{
    utils::{ContainerInfo, ContainerInfoComponent},
    Vote, VoteSource,
};
use std::{
    collections::VecDeque,
    mem::size_of,
    sync::{Arc, Condvar, Mutex},
};

pub struct VoteProcessorQueue {
    data: Mutex<VoteProcessorQueueData>,
    condition: Condvar,
    pub config: VoteProcessorConfig,
    stats: Arc<Stats>,
    rep_tiers: Arc<RepTiers>,
}

impl VoteProcessorQueue {
    pub fn new(config: VoteProcessorConfig, stats: Arc<Stats>, rep_tiers: Arc<RepTiers>) -> Self {
        let conf = config.clone();
        Self {
            data: Mutex::new(VoteProcessorQueueData {
                stopped: false,
                queue: FairQueue::new(
                    Box::new(move |origin| match origin.source {
                        RepTier::Tier1 | RepTier::Tier2 | RepTier::Tier3 => conf.max_pr_queue,
                        RepTier::None => conf.max_non_pr_queue,
                    }),
                    Box::new(move |origin| match origin.source {
                        RepTier::Tier3 => conf.pr_priority * conf.pr_priority * conf.pr_priority,
                        RepTier::Tier2 => conf.pr_priority * conf.pr_priority,
                        RepTier::Tier1 => conf.pr_priority,
                        RepTier::None => 1,
                    }),
                ),
            }),
            condition: Condvar::new(),
            config,
            stats,
            rep_tiers,
        }
    }

    pub fn len(&self) -> usize {
        self.data.lock().unwrap().queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.lock().unwrap().queue.is_empty()
    }

    /// Queue vote for processing. @returns true if the vote was queued
    pub fn vote(&self, vote: Arc<Vote>, channel: &Arc<ChannelEnum>, source: VoteSource) -> bool {
        let tier = self.rep_tiers.tier(&vote.voting_account);

        let added = {
            let mut guard = self.data.lock().unwrap();
            guard
                .queue
                .push((vote, source), Origin::new(tier, Arc::clone(channel)))
        };

        if added {
            self.stats.inc(StatType::VoteProcessor, DetailType::Process);
            self.stats.inc(StatType::VoteProcessorTier, tier.into());
            self.condition.notify_one();
        } else {
            self.stats
                .inc(StatType::VoteProcessor, DetailType::Overfill);
            self.stats.inc(StatType::VoteProcessorOverfill, tier.into());
        }

        added
    }

    pub fn wait_for_votes(
        &self,
        max_batch_size: usize,
    ) -> VecDeque<((Arc<Vote>, VoteSource), Origin<RepTier>)> {
        let mut guard = self.data.lock().unwrap();
        loop {
            if guard.stopped {
                return VecDeque::new();
            }

            if !guard.queue.is_empty() {
                return guard.queue.next_batch(max_batch_size);
            } else {
                guard = self.condition.wait(guard).unwrap();
            }
        }
    }

    pub fn clear(&self) {
        {
            let mut guard = self.data.lock().unwrap();
            guard.queue.clear();
        }
        self.condition.notify_all();
    }

    pub fn stop(&self) {
        {
            let mut guard = self.data.lock().unwrap();
            guard.stopped = true;
        }
        self.condition.notify_all();
    }

    pub fn collect_container_info(&self, name: impl Into<String>) -> ContainerInfoComponent {
        let guard = self.data.lock().unwrap();
        ContainerInfoComponent::Composite(
            name.into(),
            vec![
                ContainerInfoComponent::Leaf(ContainerInfo {
                    name: "votes".to_string(),
                    count: guard.queue.len(),
                    sizeof_element: size_of::<(Arc<Vote>, Arc<ChannelEnum>)>(),
                }),
                guard.queue.collect_container_info("queue"),
            ],
        )
    }
}

struct VoteProcessorQueueData {
    stopped: bool,
    queue: FairQueue<(Arc<Vote>, VoteSource), RepTier>,
}
