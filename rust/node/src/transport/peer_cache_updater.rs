use super::{ChannelEnum, Network};
use crate::{
    stats::{DetailType, StatType, Stats},
    utils::{CancellationToken, Runnable},
};
use rsnano_core::utils::SystemTimeFactory;
use rsnano_ledger::Ledger;
use rsnano_store_lmdb::LmdbWriteTransaction;
use std::{net::SocketAddrV6, sync::Arc, time::Duration};
use tracing::debug;

/// Writes a snapshot of the current peers to the database,
/// so that we can reconnect to them when the node is restarted
pub struct PeerCacheUpdater {
    network: Arc<Network>,
    ledger: Arc<Ledger>,
    time_factory: SystemTimeFactory,
    stats: Arc<Stats>,
    erase_cutoff: Duration,
}

impl PeerCacheUpdater {
    pub fn new(
        network: Arc<Network>,
        ledger: Arc<Ledger>,
        time_factory: SystemTimeFactory,
        stats: Arc<Stats>,
        erase_cutoff: Duration,
    ) -> Self {
        Self {
            network,
            ledger,
            time_factory,
            stats,
            erase_cutoff,
        }
    }

    fn save_peers(&self, tx: &mut LmdbWriteTransaction) {
        let live_peers = self.network.list_channels(0);
        for peer in live_peers {
            self.save_peer(tx, &peer);
        }
    }

    fn save_peer(&self, tx: &mut LmdbWriteTransaction, channel: &ChannelEnum) {
        let Some(endpoint) = channel.peering_endpoint() else {
            return;
        };
        let exists = self.ledger.store.peer.exists(tx, endpoint);

        self.ledger
            .store
            .peer
            .put(tx, endpoint, self.time_factory.now());

        if !exists {
            self.stats.inc(StatType::PeerHistory, DetailType::Inserted);
            debug!("Saved new peer: {}", endpoint);
        } else {
            self.stats.inc(StatType::PeerHistory, DetailType::Updated);
        }
    }

    fn delete_old_peers(&self, tx: &mut LmdbWriteTransaction) {
        for peer in self.get_old_peers(tx) {
            self.ledger.store.peer.del(tx, peer)
        }
    }

    fn get_old_peers(&self, tx: &LmdbWriteTransaction) -> Vec<SocketAddrV6> {
        let cutoff = self.time_factory.now() - self.erase_cutoff;
        let now = self.time_factory.now();
        self.ledger
            .store
            .peer
            .iter(tx)
            .filter_map(|(peer, time)| {
                if time < cutoff || time > now {
                    Some(peer)
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Runnable for PeerCacheUpdater {
    fn run(&mut self, _cancel_token: &CancellationToken) {
        self.stats.inc(StatType::PeerHistory, DetailType::Loop);
        let mut tx = self.ledger.rw_txn();
        self.save_peers(&mut tx);
        self.delete_old_peers(&mut tx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::Direction;
    use rsnano_core::utils::{
        new_test_timestamp, TEST_ENDPOINT_1, TEST_ENDPOINT_2, TEST_ENDPOINT_3,
    };
    use std::{net::SocketAddrV6, time::SystemTime};
    use tracing_test::traced_test;

    #[test]
    fn no_peers() {
        let open_channels = Vec::new();
        let already_stored = Vec::new();
        let (written, _, _) = run_peer_history(new_test_timestamp(), open_channels, already_stored);
        assert_eq!(written, Vec::new());
    }

    #[test]
    fn write_one_peer() {
        let now = new_test_timestamp();
        let endpoint = TEST_ENDPOINT_1;
        let open_channels = vec![endpoint];
        let already_stored = Vec::new();

        let (written, _, _) = run_peer_history(now, open_channels, already_stored);

        assert_eq!(written, vec![(endpoint, now)]);
    }

    #[test]
    fn write_multiple_peers() {
        let now = new_test_timestamp();
        let open_channels = vec![TEST_ENDPOINT_1, TEST_ENDPOINT_2, TEST_ENDPOINT_3];
        let already_stored = Vec::new();

        let (written, deleted, _) = run_peer_history(now, open_channels, already_stored);

        assert_eq!(
            written,
            vec![
                (TEST_ENDPOINT_1, now),
                (TEST_ENDPOINT_2, now),
                (TEST_ENDPOINT_3, now)
            ]
        );
        assert_eq!(deleted, Vec::new());
    }

    #[test]
    fn update_peer() {
        let endpoint = TEST_ENDPOINT_1;
        let now = new_test_timestamp();
        let open_channels = vec![endpoint];
        let already_stored = vec![(endpoint, now)];

        let (written, deleted, _) = run_peer_history(now, open_channels, already_stored);

        assert_eq!(written, vec![(endpoint, now)]);
        assert_eq!(deleted, Vec::new());
    }

    #[test]
    #[traced_test]
    fn log_when_new_peer_saved() {
        let open_channels = vec![TEST_ENDPOINT_1];
        let already_stored = Vec::new();

        run_peer_history(new_test_timestamp(), open_channels, already_stored);

        assert!(logs_contain("Saved new peer: [::ffff:10:0:0:1]:1111"));
    }

    #[test]
    #[traced_test]
    fn dont_log_when_peer_updated() {
        let endpoint = TEST_ENDPOINT_1;
        let now = new_test_timestamp();
        let open_channels = vec![endpoint];
        let already_stored = vec![(endpoint, now)];

        run_peer_history(now, open_channels, already_stored);

        logs_assert(|lines| {
            if lines.is_empty() {
                Ok(())
            } else {
                Err("log was written".to_string())
            }
        });
    }

    #[test]
    fn inc_stats_when_peer_inserted() {
        let endpoint = TEST_ENDPOINT_1;
        let open_channels = vec![endpoint];
        let already_stored = Vec::new();

        let (_, _, stats) = run_peer_history(new_test_timestamp(), open_channels, already_stored);
        assert_eq!(
            stats.count(StatType::PeerHistory, DetailType::Inserted, Direction::In),
            1
        );
        assert_eq!(
            stats.count(StatType::PeerHistory, DetailType::Updated, Direction::In),
            0
        );
    }

    #[test]
    fn inc_stats_when_peer_updated() {
        let endpoint = TEST_ENDPOINT_1;
        let open_channels = vec![endpoint];
        let already_stored = vec![(endpoint, new_test_timestamp())];

        let (_, _, stats) = run_peer_history(new_test_timestamp(), open_channels, already_stored);
        assert_eq!(
            stats.count(StatType::PeerHistory, DetailType::Inserted, Direction::In),
            0
        );
        assert_eq!(
            stats.count(StatType::PeerHistory, DetailType::Updated, Direction::In),
            1
        );
    }

    #[test]
    fn erase_entries_older_than_cutoff() {
        let open_channels = Vec::new();
        let endpoint = TEST_ENDPOINT_1;
        let now = new_test_timestamp();
        let already_stored = vec![(endpoint, now - Duration::from_secs(60 * 61))];

        let (written, deleted, _) =
            run_peer_history(new_test_timestamp(), open_channels, already_stored);

        assert_eq!(written, Vec::new());
        assert_eq!(deleted, vec![endpoint]);
    }

    #[test]
    fn erase_entries_newer_than_now() {
        let open_channels = Vec::new();
        let endpoint = TEST_ENDPOINT_1;
        let now = new_test_timestamp();
        let already_stored = vec![(endpoint, now + Duration::from_secs(60 * 61))];

        let (written, deleted, _) =
            run_peer_history(new_test_timestamp(), open_channels, already_stored);

        assert_eq!(written, Vec::new());
        assert_eq!(deleted, vec![endpoint]);
    }

    #[test]
    fn inc_loop_stats() {
        let open_channels = Vec::new();
        let already_stored = Vec::new();

        let (_, _, stats) = run_peer_history(new_test_timestamp(), open_channels, already_stored);

        assert_eq!(
            stats.count(StatType::PeerHistory, DetailType::Loop, Direction::In),
            1
        );
    }

    fn run_peer_history(
        now: SystemTime,
        open_channels: Vec<SocketAddrV6>,
        already_stored: Vec<(SocketAddrV6, SystemTime)>,
    ) -> (
        Vec<(SocketAddrV6, SystemTime)>,
        Vec<SocketAddrV6>,
        Arc<Stats>,
    ) {
        let network = Arc::new(Network::new_null());
        for endpoint in open_channels {
            network.insert_fake(endpoint);
        }
        let ledger = Arc::new(Ledger::new_null_builder().peers(already_stored).finish());
        let time_factory = SystemTimeFactory::new_null_with(now);
        let stats = Arc::new(Stats::default());
        let put_tracker = ledger.store.peer.track_puts();
        let delete_tracker = ledger.store.peer.track_deletions();
        let erase_cutoff = Duration::from_secs(60 * 60);
        let mut peer_history = PeerCacheUpdater::new(
            network,
            ledger,
            time_factory,
            Arc::clone(&stats),
            erase_cutoff,
        );

        peer_history.run(&CancellationToken::new());

        (put_tracker.output(), delete_tracker.output(), stats)
    }
}
