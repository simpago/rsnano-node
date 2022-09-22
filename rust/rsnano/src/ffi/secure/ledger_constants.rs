use std::{
    convert::TryFrom,
    sync::{Arc, RwLock},
};

use num::FromPrimitive;

use crate::{
    ffi::{
        config::{fill_work_thresholds_dto, WorkThresholdsDto},
        BlockHandle,
    },
    Account, Amount, BlockEnum, Epoch, Epochs, KeyPair, LedgerConstants, Link, PublicKey,
    WorkThresholds,
};

#[repr(C)]
pub struct LedgerConstantsDto {
    pub work: WorkThresholdsDto,
    pub priv_key: [u8; 32],
    pub pub_key: [u8; 32],
    pub nano_beta_account: [u8; 32],
    pub nano_live_account: [u8; 32],
    pub nano_test_account: [u8; 32],
    pub nano_dev_genesis: *mut BlockHandle,
    pub nano_beta_genesis: *mut BlockHandle,
    pub nano_live_genesis: *mut BlockHandle,
    pub nano_test_genesis: *mut BlockHandle,
    pub genesis: *mut BlockHandle,
    pub genesis_amount: [u8; 16],
    pub burn_account: [u8; 32],
    pub nano_dev_final_votes_canary_account: [u8; 32],
    pub nano_beta_final_votes_canary_account: [u8; 32],
    pub nano_live_final_votes_canary_account: [u8; 32],
    pub nano_test_final_votes_canary_account: [u8; 32],
    pub final_votes_canary_account: [u8; 32],
    pub nano_dev_final_votes_canary_height: u64,
    pub nano_beta_final_votes_canary_height: u64,
    pub nano_live_final_votes_canary_height: u64,
    pub nano_test_final_votes_canary_height: u64,
    pub final_votes_canary_height: u64,
    pub epoch_1_signer: [u8; 32],
    pub epoch_1_link: [u8; 32],
    pub epoch_2_signer: [u8; 32],
    pub epoch_2_link: [u8; 32],
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_constants_create(
    dto: *mut LedgerConstantsDto,
    work: &WorkThresholdsDto,
    network: u16,
) -> i32 {
    let network = match FromPrimitive::from_u16(network) {
        Some(n) => n,
        None => return -1,
    };

    let work = WorkThresholds::from(work);
    let ledger = match LedgerConstants::new(work, network) {
        Ok(l) => l,
        Err(_) => return -1,
    };

    fill_ledger_constants_dto(&mut (*dto), &ledger);
    0
}

fn block_to_block_handle(block: &Arc<RwLock<BlockEnum>>) -> *mut BlockHandle {
    Box::into_raw(Box::new(BlockHandle {
        block: Arc::clone(block),
    }))
}

pub fn fill_ledger_constants_dto(dto: &mut LedgerConstantsDto, ledger: &LedgerConstants) {
    fill_work_thresholds_dto(&mut dto.work, &ledger.work);
    dto.pub_key = ledger.zero_key.public_key().to_be_bytes();
    dto.priv_key = *ledger.zero_key.private_key().as_bytes();
    dto.nano_beta_account = ledger.nano_beta_account.to_bytes();
    dto.nano_live_account = ledger.nano_live_account.to_bytes();
    dto.nano_test_account = ledger.nano_test_account.to_bytes();
    dto.nano_dev_genesis = block_to_block_handle(&ledger.nano_dev_genesis);
    dto.nano_beta_genesis = block_to_block_handle(&ledger.nano_beta_genesis);
    dto.nano_live_genesis = block_to_block_handle(&ledger.nano_live_genesis);
    dto.nano_test_genesis = block_to_block_handle(&ledger.nano_test_genesis);
    dto.genesis = block_to_block_handle(&ledger.genesis);
    dto.genesis_amount = ledger.genesis_amount.to_be_bytes();
    dto.burn_account = ledger.burn_account.to_bytes();
    dto.nano_dev_final_votes_canary_account = ledger.nano_dev_final_votes_canary_account.to_bytes();
    dto.nano_beta_final_votes_canary_account =
        ledger.nano_beta_final_votes_canary_account.to_bytes();
    dto.nano_live_final_votes_canary_account =
        ledger.nano_live_final_votes_canary_account.to_bytes();
    dto.nano_test_final_votes_canary_account =
        ledger.nano_test_final_votes_canary_account.to_bytes();
    dto.final_votes_canary_account = ledger.final_votes_canary_account.to_bytes();
    dto.nano_dev_final_votes_canary_height = ledger.nano_dev_final_votes_canary_height;
    dto.nano_beta_final_votes_canary_height = ledger.nano_beta_final_votes_canary_height;
    dto.nano_live_final_votes_canary_height = ledger.nano_live_final_votes_canary_height;
    dto.nano_test_final_votes_canary_height = ledger.nano_test_final_votes_canary_height;
    dto.final_votes_canary_height = ledger.final_votes_canary_height;
    dto.epoch_1_signer = *ledger.epochs.signer(Epoch::Epoch1).unwrap().as_bytes();
    dto.epoch_1_link = *ledger.epochs.link(Epoch::Epoch1).unwrap().as_bytes();
    dto.epoch_2_signer = *ledger.epochs.signer(Epoch::Epoch2).unwrap().as_bytes();
    dto.epoch_2_link = *ledger.epochs.link(Epoch::Epoch2).unwrap().as_bytes();
}

impl TryFrom<&LedgerConstantsDto> for LedgerConstants {
    type Error = anyhow::Error;

    fn try_from(value: &LedgerConstantsDto) -> Result<Self, Self::Error> {
        let mut epochs = Epochs::new();
        epochs.add(
            Epoch::Epoch1,
            PublicKey::from_bytes(value.epoch_1_signer),
            Link::from_bytes(value.epoch_1_link),
        );
        epochs.add(
            Epoch::Epoch2,
            PublicKey::from_bytes(value.epoch_2_signer),
            Link::from_bytes(value.epoch_2_link),
        );

        let ledger = LedgerConstants {
            work: (&value.work).into(),
            zero_key: KeyPair::from_priv_key_bytes(&value.priv_key)?,
            nano_beta_account: Account::from_bytes(value.nano_beta_account),
            nano_live_account: Account::from_bytes(value.nano_live_account),
            nano_test_account: Account::from_bytes(value.nano_test_account),
            nano_dev_genesis: unsafe { &*value.nano_dev_genesis }.block.clone(),
            nano_beta_genesis: unsafe { &*value.nano_beta_genesis }.block.clone(),
            nano_live_genesis: unsafe { &*value.nano_live_genesis }.block.clone(),
            nano_test_genesis: unsafe { &*value.nano_test_genesis }.block.clone(),
            genesis: unsafe { &*value.genesis }.block.clone(),
            genesis_amount: Amount::from_be_bytes(value.genesis_amount),
            burn_account: Account::from_bytes(value.burn_account),
            nano_dev_final_votes_canary_account: Account::from_bytes(
                value.nano_dev_final_votes_canary_account,
            ),
            nano_beta_final_votes_canary_account: Account::from_bytes(
                value.nano_beta_final_votes_canary_account,
            ),
            nano_live_final_votes_canary_account: Account::from_bytes(
                value.nano_live_final_votes_canary_account,
            ),
            nano_test_final_votes_canary_account: Account::from_bytes(
                value.nano_test_final_votes_canary_account,
            ),
            final_votes_canary_account: Account::from_bytes(value.final_votes_canary_account),
            nano_dev_final_votes_canary_height: value.nano_dev_final_votes_canary_height,
            nano_beta_final_votes_canary_height: value.nano_beta_final_votes_canary_height,
            nano_live_final_votes_canary_height: value.nano_live_final_votes_canary_height,
            nano_test_final_votes_canary_height: value.nano_test_final_votes_canary_height,
            final_votes_canary_height: value.final_votes_canary_height,
            epochs,
        };

        Ok(ledger)
    }
}
