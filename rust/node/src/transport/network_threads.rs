use super::{LatestKeepalives, MessagePublisher, SynCookies};
use crate::{
    config::{NodeConfig, NodeFlags},
    stats::{DetailType, StatType, Stats},
    NetworkParams,
};
use rsnano_core::utils::NULL_ENDPOINT;
use rsnano_messages::{Keepalive, Message};
use rsnano_network::{DeadChannelCleanup, DropPolicy, NetworkInfo, PeerConnector, TrafficType};
use rsnano_nullable_clock::SteadyClock;
use std::{
    net::{Ipv6Addr, SocketAddrV6},
    sync::{Arc, Condvar, Mutex, RwLock},
    thread::JoinHandle,
    time::Duration,
};

pub(crate) struct NetworkThreads {
    cleanup_thread: Option<JoinHandle<()>>,
    keepalive_thread: Option<JoinHandle<()>>,
    reachout_thread: Option<JoinHandle<()>>,
    stopped: Arc<(Condvar, Mutex<bool>)>,
    network: Arc<RwLock<NetworkInfo>>,
    peer_connector: Arc<PeerConnector>,
    flags: NodeFlags,
    network_params: NetworkParams,
    stats: Arc<Stats>,
    syn_cookies: Arc<SynCookies>,
    keepalive_factory: Arc<KeepaliveFactory>,
    latest_keepalives: Arc<Mutex<LatestKeepalives>>,
    dead_channel_cleanup: Option<DeadChannelCleanup>,
    message_publisher: MessagePublisher,
    clock: Arc<SteadyClock>,
}

impl NetworkThreads {
    pub fn new(
        network: Arc<RwLock<NetworkInfo>>,
        peer_connector: Arc<PeerConnector>,
        flags: NodeFlags,
        network_params: NetworkParams,
        stats: Arc<Stats>,
        syn_cookies: Arc<SynCookies>,
        keepalive_factory: Arc<KeepaliveFactory>,
        latest_keepalives: Arc<Mutex<LatestKeepalives>>,
        dead_channel_cleanup: DeadChannelCleanup,
        message_publisher: MessagePublisher,
        clock: Arc<SteadyClock>,
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
            latest_keepalives,
            dead_channel_cleanup: Some(dead_channel_cleanup),
            message_publisher,
            clock,
        }
    }

    pub fn start(&mut self) {
        let cleanup = CleanupLoop {
            stopped: self.stopped.clone(),
            network_params: self.network_params.clone(),
            flags: self.flags.clone(),
            syn_cookies: self.syn_cookies.clone(),
            dead_channel_cleanup: self.dead_channel_cleanup.take().unwrap(),
        };

        self.cleanup_thread = Some(
            std::thread::Builder::new()
                .name("Net cleanup".to_string())
                .spawn(move || cleanup.run())
                .unwrap(),
        );

        let mut keepalive = KeepaliveLoop {
            stopped: self.stopped.clone(),
            network: self.network.clone(),
            keepalive_period: self.network_params.network.keepalive_period,
            stats: Arc::clone(&self.stats),
            keepalive_factory: Arc::clone(&self.keepalive_factory),
            message_publisher: self.message_publisher.clone(),
            clock: self.clock.clone(),
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
                peer_connector: self.peer_connector.clone(),
                latest_keepalives: self.latest_keepalives.clone(),
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
        self.network.write().unwrap().stop();
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
    flags: NodeFlags,
    syn_cookies: Arc<SynCookies>,
    dead_channel_cleanup: DeadChannelCleanup,
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

            if !self.flags.disable_connection_cleanup {
                self.dead_channel_cleanup.clean_up();
            }

            self.syn_cookies
                .purge(self.network_params.network.sync_cookie_cutoff);

            stopped = self.stopped.1.lock().unwrap();
        }
    }
}

pub struct KeepaliveFactory {
    pub network: Arc<RwLock<NetworkInfo>>,
    pub config: NodeConfig,
}

impl KeepaliveFactory {
    pub fn create_keepalive_self(&self) -> Keepalive {
        let mut result = Keepalive::default();
        let network = self.network.read().unwrap();
        network.random_fill_realtime(&mut result.peers);
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
                    SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, network.listening_port(), 0, 0);
                result.peers[1] = external_address;
            } else {
                result.peers[0] =
                    SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, network.listening_port(), 0, 0);
            }
        }
        result
    }
}

struct KeepaliveLoop {
    stopped: Arc<(Condvar, Mutex<bool>)>,
    stats: Arc<Stats>,
    network: Arc<RwLock<NetworkInfo>>,
    keepalive_factory: Arc<KeepaliveFactory>,
    message_publisher: MessagePublisher,
    clock: Arc<SteadyClock>,
    keepalive_period: Duration,
}

impl KeepaliveLoop {
    fn run(&mut self) {
        let mut stopped = self.stopped.1.lock().unwrap();
        while !*stopped {
            stopped = self
                .stopped
                .0
                .wait_timeout(stopped, self.keepalive_period)
                .unwrap()
                .0;

            if *stopped {
                return;
            }
            drop(stopped);

            self.stats.inc(StatType::Network, DetailType::LoopKeepalive);
            self.flood_keepalive(0.75);
            self.flood_keepalive_self(0.25);

            self.keepalive();

            stopped = self.stopped.1.lock().unwrap();
        }
    }

    fn keepalive(&mut self) {
        let (message, keepalive_list) = {
            let network = self.network.read().unwrap();
            let mut peers = [NULL_ENDPOINT; 8];
            network.random_fill_realtime(&mut peers);
            let message = Message::Keepalive(Keepalive { peers });
            let list = network.idle_channels(self.keepalive_period, self.clock.now());
            (message, list)
        };

        for channel_id in keepalive_list {
            self.message_publisher.try_send(
                channel_id,
                &message,
                DropPolicy::CanDrop,
                TrafficType::Generic,
            );
        }
    }

    fn flood_keepalive(&mut self, scale: f32) {
        let mut keepalive = Keepalive::default();
        self.network
            .read()
            .unwrap()
            .random_fill_realtime(&mut keepalive.peers);
        self.message_publisher
            .flood(&Message::Keepalive(keepalive), DropPolicy::CanDrop, scale);
    }

    fn flood_keepalive_self(&mut self, scale: f32) {
        let keepalive = self.keepalive_factory.create_keepalive_self();
        self.message_publisher
            .flood(&Message::Keepalive(keepalive), DropPolicy::CanDrop, scale);
    }
}

struct ReachoutLoop {
    stopped: Arc<(Condvar, Mutex<bool>)>,
    reachout_interval: Duration,
    stats: Arc<Stats>,
    peer_connector: Arc<PeerConnector>,
    latest_keepalives: Arc<Mutex<LatestKeepalives>>,
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

            let keepalive = self.latest_keepalives.lock().unwrap().pop_random();
            if let Some(keepalive) = keepalive {
                for peer in keepalive.peers {
                    if peer.ip().is_unspecified() {
                        continue;
                    }
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
