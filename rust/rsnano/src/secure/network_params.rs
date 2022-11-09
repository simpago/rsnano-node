use std::sync::{Arc, RwLock};

use crate::{
    config::NetworkConstants,
    core::{Account, BlockEnum, BlockHash, Networks},
    ledger::LedgerConstants,
    work::WorkThresholds,
    BootstrapConstants, NodeConstants, PortmappingConstants, VotingConstants,
};
use anyhow::Result;
use once_cell::sync::Lazy;

pub static DEV_NETWORK_PARAMS: Lazy<NetworkParams> =
    Lazy::new(|| NetworkParams::new(Networks::NanoDevNetwork).unwrap());

pub static DEV_CONSTANTS: Lazy<&LedgerConstants> = Lazy::new(|| &DEV_NETWORK_PARAMS.ledger);

pub static DEV_GENESIS: Lazy<Arc<RwLock<BlockEnum>>> = Lazy::new(|| DEV_CONSTANTS.genesis.clone());
pub static DEV_GENESIS_ACCOUNT: Lazy<Account> =
    Lazy::new(|| DEV_GENESIS.read().unwrap().as_block().account());
pub static DEV_GENESIS_HASH: Lazy<BlockHash> =
    Lazy::new(|| DEV_GENESIS.read().unwrap().as_block().hash());

pub struct NetworkParams {
    pub kdf_work: u32,
    pub work: WorkThresholds,
    pub network: NetworkConstants,
    pub ledger: LedgerConstants,
    pub voting: VotingConstants,
    pub node: NodeConstants,
    pub portmapping: PortmappingConstants,
    pub bootstrap: BootstrapConstants,
}

impl NetworkParams {
    pub fn new(network: Networks) -> Result<Self> {
        let work = if network == Networks::NanoLiveNetwork {
            WorkThresholds::publish_full()
        } else if network == Networks::NanoBetaNetwork {
            WorkThresholds::publish_beta()
        } else if network == Networks::NanoTestNetwork {
            WorkThresholds::publish_test()
        } else {
            WorkThresholds::publish_dev()
        };
        let network_constants = NetworkConstants::new(work.clone(), network);
        let kdf_full_work = 64 * 1024;
        let kdf_dev_work = 8;
        Ok(Self {
            kdf_work: if network_constants.is_dev_network() {
                kdf_dev_work
            } else {
                kdf_full_work
            },
            work: work.clone(),
            ledger: LedgerConstants::new(work.clone(), network)?,
            voting: VotingConstants::new(&network_constants),
            node: NodeConstants::new(&network_constants),
            portmapping: PortmappingConstants::new(&network_constants),
            bootstrap: BootstrapConstants::new(&network_constants),
            network: network_constants,
        })
    }
}
