use super::{Network, NetworkExt, PeerConnector, PeerConnectorExt, SynCookies};
use crate::{
    config::{NodeConfig, NodeFlags},
    representatives::OnlineReps,
    stats::{DetailType, Direction, StatType, Stats},
    NetworkParams,
};
use rsnano_messages::{Keepalive, Message};
use std::{
    net::{Ipv6Addr, SocketAddrV6},
    sync::{Arc, Condvar, Mutex},
    thread::JoinHandle,
    time::{Duration, SystemTime},
};
use tracing::info;

pub struct NetworkThreads {
    cleanup_thread: Option<JoinHandle<()>>,
    keepalive_thread: Option<JoinHandle<()>>,
    reachout_thread: Option<JoinHandle<()>>,
    stopped: Arc<(Condvar, Mutex<bool>)>,
    network: Arc<Network>,
    online_reps: Arc<Mutex<OnlineReps>>,
    peer_connector: Arc<PeerConnector>,
    flags: NodeFlags,
    network_params: NetworkParams,
    stats: Arc<Stats>,
    syn_cookies: Arc<SynCookies>,
    keepalive_factory: Arc<KeepaliveFactory>,
}

impl NetworkThreads {
    pub fn new(
        network: Arc<Network>,
        peer_connector: Arc<PeerConnector>,
        flags: NodeFlags,
        network_params: NetworkParams,
        stats: Arc<Stats>,
        syn_cookies: Arc<SynCookies>,
        keepalive_factory: Arc<KeepaliveFactory>,
        online_reps: Arc<Mutex<OnlineReps>>,
    ) -> Self {
        Self {
            cleanup_thread: None,
            keepalive_thread: None,
            reachout_thread: None,
            stopped: Arc::new((Condvar::new(), Mutex::new(false))),
            network,
            peer_connector,
            flags,
            network_params,
            stats,
            syn_cookies,
            keepalive_factory,
            online_reps,
        }
    }

    pub fn start(&mut self) {
        let cleanup = CleanupLoop {
            stopped: self.stopped.clone(),
            network_params: self.network_params.clone(),
            stats: self.stats.clone(),
            flags: self.flags.clone(),
            syn_cookies: self.syn_cookies.clone(),
            network: self.network.clone(),
            online_reps: self.online_reps.clone(),
        };

        self.cleanup_thread = Some(
            std::thread::Builder::new()
                .name("Net cleanup".to_string())
                .spawn(move || cleanup.run())
                .unwrap(),
        );

        let keepalive = KeepaliveLoop {
            stopped: self.stopped.clone(),
            network: Arc::clone(&self.network),
            network_params: self.network_params.clone(),
            stats: Arc::clone(&self.stats),
            keepalive_factory: Arc::clone(&self.keepalive_factory),
        };

        self.keepalive_thread = Some(
            std::thread::Builder::new()
                .name("Net keepalive".to_string())
                .spawn(move || keepalive.run())
                .unwrap(),
        );

        if !self.network_params.network.merge_period.is_zero() {
            let reachout = ReachoutLoop {
                stopped: self.stopped.clone(),
                reachout_interval: self.network_params.network.merge_period,
                stats: self.stats.clone(),
                network: self.network.clone(),
                peer_connector: self.peer_connector.clone(),
            };

            self.reachout_thread = Some(
                std::thread::Builder::new()
                    .name("Net reachout".to_string())
                    .spawn(move || reachout.run())
                    .unwrap(),
            );
        }
    }
    pub fn stop(&mut self) {
        *self.stopped.1.lock().unwrap() = true;
        self.stopped.0.notify_all();
        self.network.stop();
        if let Some(t) = self.keepalive_thread.take() {
            t.join().unwrap();
        }
        if let Some(t) = self.cleanup_thread.take() {
            t.join().unwrap();
        }
        if let Some(t) = self.reachout_thread.take() {
            t.join().unwrap();
        }
    }
}

impl Drop for NetworkThreads {
    fn drop(&mut self) {
        // All threads must be stopped before this destructor
        debug_assert!(self.cleanup_thread.is_none());
        debug_assert!(self.keepalive_thread.is_none());
    }
}

struct CleanupLoop {
    stopped: Arc<(Condvar, Mutex<bool>)>,
    network_params: NetworkParams,
    stats: Arc<Stats>,
    flags: NodeFlags,
    syn_cookies: Arc<SynCookies>,
    network: Arc<Network>,
    online_reps: Arc<Mutex<OnlineReps>>,
}

impl CleanupLoop {
    fn run(&self) {
        let mut stopped = self.stopped.1.lock().unwrap();
        while !*stopped {
            let timeout = if self.network_params.network.is_dev_network() {
                Duration::from_secs(1)
            } else {
                Duration::from_secs(5)
            };
            stopped = self.stopped.0.wait_timeout(stopped, timeout).unwrap().0;

            if *stopped {
                return;
            }
            drop(stopped);

            self.stats.inc(StatType::Network, DetailType::LoopCleanup);

            if !self.flags.disable_connection_cleanup {
                let removed_channel_ids = self
                    .network
                    .purge(SystemTime::now() - self.network_params.network.cleanup_cutoff());

                let mut online_reps = self.online_reps.lock().unwrap();
                for channel_id in removed_channel_ids {
                    let removed_reps = online_reps.remove_peer(channel_id);
                    for rep in removed_reps {
                        info!(
                            "Evicting representative {} with dead channel",
                            rep.encode_account(),
                        );
                        self.stats.inc_dir(
                            StatType::RepCrawler,
                            DetailType::ChannelDead,
                            Direction::In,
                        );
                    }
                }
            }

            self.syn_cookies
                .purge(self.network_params.network.sync_cookie_cutoff);

            stopped = self.stopped.1.lock().unwrap();
        }
    }
}

pub struct KeepaliveFactory {
    pub network: Arc<Network>,
    pub config: NodeConfig,
}

impl KeepaliveFactory {
    pub fn create_keepalive_self(&self) -> Keepalive {
        let mut result = Keepalive::default();
        self.network.random_fill(&mut result.peers);
        // We will clobber values in index 0 and 1 and if there are only 2 nodes in the system, these are the only positions occupied
        // Move these items to index 2 and 3 so they propagate
        result.peers[2] = result.peers[0];
        result.peers[3] = result.peers[1];
        // Replace part of message with node external address or listening port
        result.peers[1] = SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 0, 0, 0); // For node v19 (response channels)
        if self.config.external_address != Ipv6Addr::UNSPECIFIED.to_string()
            && self.config.external_port != 0
        {
            result.peers[0] = SocketAddrV6::new(
                self.config.external_address.parse().unwrap(),
                self.config.external_port,
                0,
                0,
            );
        } else {
            // TODO Read external address from port_mapping!
            //let external_address  node.port_mapping.external_address ());
            let external_address = SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 0, 0, 0);
            if !external_address.ip().is_unspecified() {
                result.peers[0] =
                    SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, self.network.port(), 0, 0);
                result.peers[1] = external_address;
            } else {
                result.peers[0] =
                    SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, self.network.port(), 0, 0);
            }
        }
        result
    }
}

struct KeepaliveLoop {
    stopped: Arc<(Condvar, Mutex<bool>)>,
    network_params: NetworkParams,
    stats: Arc<Stats>,
    network: Arc<Network>,
    keepalive_factory: Arc<KeepaliveFactory>,
}

impl KeepaliveLoop {
    fn run(&self) {
        let mut stopped = self.stopped.1.lock().unwrap();
        while !*stopped {
            stopped = self
                .stopped
                .0
                .wait_timeout(stopped, self.network_params.network.keepalive_period)
                .unwrap()
                .0;

            if *stopped {
                return;
            }
            drop(stopped);

            self.stats.inc(StatType::Network, DetailType::LoopKeepalive);
            self.flood_keepalive(0.75);
            self.flood_keepalive_self(0.25);

            self.network.keepalive();

            stopped = self.stopped.1.lock().unwrap();
        }
    }

    fn flood_keepalive(&self, scale: f32) {
        let mut keepalive = Keepalive::default();
        self.network.random_fill(&mut keepalive.peers);
        self.network
            .flood_message(&Message::Keepalive(keepalive), scale);
    }

    fn flood_keepalive_self(&self, scale: f32) {
        let keepalive = self.keepalive_factory.create_keepalive_self();
        self.network
            .flood_message(&Message::Keepalive(keepalive), scale);
    }
}

struct ReachoutLoop {
    stopped: Arc<(Condvar, Mutex<bool>)>,
    reachout_interval: Duration,
    stats: Arc<Stats>,
    network: Arc<Network>,
    peer_connector: Arc<PeerConnector>,
}

impl ReachoutLoop {
    fn run(&self) {
        let mut stopped = self.stopped.1.lock().unwrap();
        while !*stopped {
            stopped = self
                .stopped
                .0
                .wait_timeout(stopped, self.reachout_interval)
                .unwrap()
                .0;

            if *stopped {
                return;
            }
            drop(stopped);

            self.stats.inc(StatType::Network, DetailType::LoopReachout);

            if let Some(keepalive) = self.network.sample_keepalive() {
                for peer in keepalive.peers {
                    self.stats.inc(StatType::Network, DetailType::ReachoutLive);
                    self.peer_connector.connect_to(peer);

                    // Throttle reachout attempts
                    std::thread::sleep(self.reachout_interval);
                }
            }

            stopped = self.stopped.1.lock().unwrap();
        }
    }
}
