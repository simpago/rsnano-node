use std::sync::{Arc, RwLock};

use anyhow::Result;
use once_cell::sync::Lazy;

use crate::{
    config::get_env_or_default_string,
    core::{
        deserialize_block_json, Account, Amount, BlockDetails, BlockEnum, BlockHash, BlockSideband,
        Epoch, Epochs, KeyPair, Link, Networks,
    },
    utils::{seconds_since_epoch, SerdePropertyTree},
    work::WorkThresholds,
};

static DEV_PRIVATE_KEY_DATA: &str =
    "34F0A37AAD20F4A260F0A5B3CB3D7FB50673212263E58A380BC10474BB039CE4";
static DEV_PUBLIC_KEY_DATA: &str =
    "B0311EA55708D6A53C75CDBF88300259C6D018522FE3D4D0A242E431F9E8B6D0"; // xrb_3e3j5tkog48pnny9dmfzj1r16pg8t1e76dz5tmac6iq689wyjfpiij4txtdo
static BETA_PUBLIC_KEY_DATA: &str =
    "259A43ABDB779E97452E188BA3EB951B41C961D3318CA6B925380F4D99F0577A"; // nano_1betagoxpxwykx4kw86dnhosc8t3s7ix8eeentwkcg1hbpez1outjrcyg4n1
static LIVE_PUBLIC_KEY_DATA: &str =
    "E89208DD038FBB269987689621D52292AE9C35941A7484756ECCED92A65093BA"; // xrb_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3
static TEST_PUBLIC_KEY_DATA: Lazy<String> = Lazy::new(|| {
    get_env_or_default_string(
        "NANO_TEST_GENESIS_PUB",
        "45C6FF9D1706D61F0821327752671BDA9F9ED2DA40326B01935AB566FB9E08ED",
    ) // nano_1jg8zygjg3pp5w644emqcbmjqpnzmubfni3kfe1s8pooeuxsw49fdq1mco9j
});

static DEV_GENESIS_DATA: &str = r###"{
	"type": "open",
	"source": "B0311EA55708D6A53C75CDBF88300259C6D018522FE3D4D0A242E431F9E8B6D0",
	"representative": "xrb_3e3j5tkog48pnny9dmfzj1r16pg8t1e76dz5tmac6iq689wyjfpiij4txtdo",
	"account": "xrb_3e3j5tkog48pnny9dmfzj1r16pg8t1e76dz5tmac6iq689wyjfpiij4txtdo",
	"work": "7b42a00ee91d5810",
	"signature": "ECDA914373A2F0CA1296475BAEE40500A7F0A7AD72A5A80C81D7FAB7F6C802B2CC7DB50F5DD0FB25B2EF11761FA7344A158DD5A700B21BD47DE5BD0F63153A02"
    }"###;

static BETA_GENESIS_DATA: &str = r###"{
	"type": "open",
	"source": "259A43ABDB779E97452E188BA3EB951B41C961D3318CA6B925380F4D99F0577A",
	"representative": "nano_1betagoxpxwykx4kw86dnhosc8t3s7ix8eeentwkcg1hbpez1outjrcyg4n1",
	"account": "nano_1betagoxpxwykx4kw86dnhosc8t3s7ix8eeentwkcg1hbpez1outjrcyg4n1",
	"work": "79d4e27dc873c6f2",
	"signature": "4BD7F96F9ED2721BCEE5EAED400EA50AD00524C629AE55E9AFF11220D2C1B00C3D4B3BB770BF67D4F8658023B677F91110193B6C101C2666931F57046A6DB806"
    }"###;

static LIVE_GENESIS_DATA: &str = r###"{
	"type": "open",
	"source": "E89208DD038FBB269987689621D52292AE9C35941A7484756ECCED92A65093BA",
	"representative": "xrb_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3",
	"account": "xrb_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3",
	"work": "62f05417dd3fb691",
	"signature": "9F0C933C8ADE004D808EA1985FA746A7E95BA2A38F867640F53EC8F180BDFE9E2C1268DEAD7C2664F356E37ABA362BC58E46DBA03E523A7B5A19E4B6EB12BB02"
    }"###;

static TEST_GENESIS_DATA: Lazy<String> = Lazy::new(|| {
    get_env_or_default_string(
        "NANO_TEST_GENESIS_BLOCK",
        r###"{
        "type": "open",
        "source": "45C6FF9D1706D61F0821327752671BDA9F9ED2DA40326B01935AB566FB9E08ED",
        "representative": "nano_1jg8zygjg3pp5w644emqcbmjqpnzmubfni3kfe1s8pooeuxsw49fdq1mco9j",
        "account": "nano_1jg8zygjg3pp5w644emqcbmjqpnzmubfni3kfe1s8pooeuxsw49fdq1mco9j",
        "work": "bc1ef279c1a34eb1",
        "signature": "15049467CAEE3EC768639E8E35792399B6078DA763DA4EBA8ECAD33B0EDC4AF2E7403893A5A602EB89B978DABEF1D6606BB00F3C0EE11449232B143B6E07170E"
        }"###,
    )
});

static BETA_CANARY_PUBLIC_KEY_DATA: &str =
    "868C6A9F79D4506E029B378262B91538C5CB26D7C346B63902FFEB365F1C1947"; // nano_33nefchqmo4ifr3bpfw4ecwjcg87semfhit8prwi7zzd8shjr8c9qdxeqmnx
static LIVE_CANARY_PUBLIC_KEY_DATA: &str =
    "7CBAF192A3763DAEC9F9BAC1B2CDF665D8369F8400B4BC5AB4BA31C00BAA4404"; // nano_1z7ty8bc8xjxou6zmgp3pd8zesgr8thra17nqjfdbgjjr17tnj16fjntfqfn

static TEST_CANARY_PUBLIC_KEY_DATA: Lazy<String> = Lazy::new(|| {
    get_env_or_default_string(
        "NANO_TEST_CANARY_PUB",
        "3BAD2C554ACE05F5E528FBBCE79D51E552C55FA765CCFD89B289C4835DE5F04A",
    ) // nano_1gxf7jcnomi7yqkkjyxwwygo5sckrohtgsgezp6u74g6ifgydw4cajwbk8bf
});

pub static DEV_GENESIS_KEY: Lazy<KeyPair> =
    Lazy::new(|| KeyPair::from_priv_key_hex(DEV_PRIVATE_KEY_DATA).unwrap());

fn parse_block_from_genesis_data(genesis_data: &str) -> Result<BlockEnum> {
    let ptree = SerdePropertyTree::parse(genesis_data)?;
    deserialize_block_json(&ptree)
}

#[cfg(test)]
mod tests {
    use crate::core::BlockType;

    use super::*;

    #[test]
    fn test_parse_block() {
        let block_str = r###"{"type": "open", "source": "37FCEA4DA94F1635484EFCBA57483C4C654F573B435C09D8AACE1CB45E63FFB1", "representative": "nano_1fzwxb8tkmrp8o66xz7tcx65rm57bxdmpitw39ecomiwpjh89zxj33juzt6p", "account": "nano_1fzwxb8tkmrp8o66xz7tcx65rm57bxdmpitw39ecomiwpjh89zxj33juzt6p", "work": "ef0547d86748c71b", "signature": "13E33D1ADA50A79B64741C5159C0C0AFE0515581B47ABD73676FE02A1D600CDB637050D37BF92C9629649AE92949814BB57C6B5B0A44BF76E2F33043A3DF2D01"}"###;
        let block = parse_block_from_genesis_data(block_str).unwrap();
        assert_eq!(block.block_type(), BlockType::Open);
    }
}

#[derive(Clone)]
pub struct LedgerConstants {
    pub work: WorkThresholds,
    pub zero_key: KeyPair,
    pub nano_beta_account: Account,
    pub nano_live_account: Account,
    pub nano_test_account: Account,
    pub nano_dev_genesis: Arc<RwLock<BlockEnum>>,
    pub nano_beta_genesis: Arc<RwLock<BlockEnum>>,
    pub nano_live_genesis: Arc<RwLock<BlockEnum>>,
    pub nano_test_genesis: Arc<RwLock<BlockEnum>>,
    pub genesis: Arc<RwLock<BlockEnum>>,
    pub genesis_amount: Amount,
    pub burn_account: Account,
    pub nano_dev_final_votes_canary_account: Account,
    pub nano_beta_final_votes_canary_account: Account,
    pub nano_live_final_votes_canary_account: Account,
    pub nano_test_final_votes_canary_account: Account,
    pub final_votes_canary_account: Account,
    pub nano_dev_final_votes_canary_height: u64,
    pub nano_beta_final_votes_canary_height: u64,
    pub nano_live_final_votes_canary_height: u64,
    pub nano_test_final_votes_canary_height: u64,
    pub final_votes_canary_height: u64,
    pub epochs: Epochs,
}

impl LedgerConstants {
    pub fn new(work: WorkThresholds, network: Networks) -> Result<Self> {
        let mut nano_dev_genesis = parse_block_from_genesis_data(DEV_GENESIS_DATA)?;
        let mut nano_beta_genesis = parse_block_from_genesis_data(BETA_GENESIS_DATA)?;
        let mut nano_live_genesis = parse_block_from_genesis_data(LIVE_GENESIS_DATA)?;
        let mut nano_test_genesis = parse_block_from_genesis_data(TEST_GENESIS_DATA.as_str())?;

        let beta_genesis_account = nano_beta_genesis.as_block().account();
        nano_beta_genesis
            .as_block_mut()
            .set_sideband(BlockSideband::new(
                beta_genesis_account,
                BlockHash::from(0),
                Amount::new(u128::MAX),
                1,
                seconds_since_epoch(),
                BlockDetails::new(Epoch::Epoch0, false, false, false),
                Epoch::Epoch0,
            ));

        let dev_genesis_account = nano_dev_genesis.as_block().account();
        nano_dev_genesis
            .as_block_mut()
            .set_sideband(BlockSideband::new(
                dev_genesis_account,
                BlockHash::from(0),
                Amount::new(u128::MAX),
                1,
                seconds_since_epoch(),
                BlockDetails::new(Epoch::Epoch0, false, false, false),
                Epoch::Epoch0,
            ));

        let live_genesis_account = nano_live_genesis.as_block().account();
        nano_live_genesis
            .as_block_mut()
            .set_sideband(BlockSideband::new(
                live_genesis_account,
                BlockHash::from(0),
                Amount::new(u128::MAX),
                1,
                seconds_since_epoch(),
                BlockDetails::new(Epoch::Epoch0, false, false, false),
                Epoch::Epoch0,
            ));

        let test_genesis_account = nano_test_genesis.as_block().account();
        nano_test_genesis
            .as_block_mut()
            .set_sideband(BlockSideband::new(
                test_genesis_account,
                BlockHash::from(0),
                Amount::new(u128::MAX),
                1,
                seconds_since_epoch(),
                BlockDetails::new(Epoch::Epoch0, false, false, false),
                Epoch::Epoch0,
            ));

        let genesis = match network {
            Networks::NanoDevNetwork => nano_dev_genesis.clone(),
            Networks::NanoBetaNetwork => nano_beta_genesis.clone(),
            Networks::NanoTestNetwork => nano_test_genesis.clone(),
            Networks::NanoLiveNetwork => nano_live_genesis.clone(),
            Networks::Invalid => bail!("invalid network"),
        };

        let nano_dev_final_votes_canary_account = Account::decode_hex(DEV_PUBLIC_KEY_DATA)?;
        let nano_beta_final_votes_canary_account =
            Account::decode_hex(BETA_CANARY_PUBLIC_KEY_DATA)?;
        let nano_live_final_votes_canary_account =
            Account::decode_hex(LIVE_CANARY_PUBLIC_KEY_DATA)?;
        let nano_test_final_votes_canary_account =
            Account::decode_hex(TEST_CANARY_PUBLIC_KEY_DATA.as_str())?;

        let final_votes_canary_account = match network {
            Networks::NanoDevNetwork => nano_dev_final_votes_canary_account,
            Networks::NanoBetaNetwork => nano_beta_final_votes_canary_account,
            Networks::NanoLiveNetwork => nano_live_final_votes_canary_account,
            Networks::NanoTestNetwork => nano_test_final_votes_canary_account,
            Networks::Invalid => bail!("invalid network"),
        };

        let nano_beta_account = Account::decode_hex(BETA_PUBLIC_KEY_DATA)?;
        let nano_test_account = Account::decode_hex(TEST_PUBLIC_KEY_DATA.as_str())?;

        let mut epochs = Epochs::new();

        let epoch_1_signer = genesis.as_block().account().into();
        let mut link_bytes = [0u8; 32];
        link_bytes[..14].copy_from_slice(b"epoch v1 block");
        let epoch_link_v1 = Link::from_bytes(link_bytes);

        let nano_live_epoch_v2_signer = Account::decode_account(
            "nano_3qb6o6i1tkzr6jwr5s7eehfxwg9x6eemitdinbpi7u8bjjwsgqfj4wzser3x",
        )
        .unwrap();
        let epoch_2_signer = match network {
            Networks::NanoDevNetwork => DEV_GENESIS_KEY.public_key(),
            Networks::NanoBetaNetwork => nano_beta_account.into(),
            Networks::NanoLiveNetwork => nano_live_epoch_v2_signer.into(),
            Networks::NanoTestNetwork => nano_test_account.into(),
            _ => panic!("invalid network"),
        };
        link_bytes[..14].copy_from_slice(b"epoch v2 block");
        let epoch_link_v2 = Link::from_bytes(link_bytes);

        epochs.add(Epoch::Epoch1, epoch_1_signer, epoch_link_v1);
        epochs.add(Epoch::Epoch2, epoch_2_signer, epoch_link_v2);

        Ok(Self {
            work,
            zero_key: KeyPair::zero(),
            nano_beta_account,
            nano_live_account: Account::decode_hex(LIVE_PUBLIC_KEY_DATA)?,
            nano_test_account,
            nano_dev_genesis: Arc::new(RwLock::new(nano_dev_genesis)),
            nano_beta_genesis: Arc::new(RwLock::new(nano_beta_genesis)),
            nano_live_genesis: Arc::new(RwLock::new(nano_live_genesis)),
            nano_test_genesis: Arc::new(RwLock::new(nano_test_genesis)),
            genesis: Arc::new(RwLock::new(genesis)),
            genesis_amount: Amount::new(u128::MAX),
            burn_account: Account::zero(),
            nano_dev_final_votes_canary_account,
            nano_beta_final_votes_canary_account,
            nano_live_final_votes_canary_account,
            nano_test_final_votes_canary_account,
            final_votes_canary_account,
            nano_dev_final_votes_canary_height: 1,
            nano_beta_final_votes_canary_height: 1,
            nano_live_final_votes_canary_height: 1,
            nano_test_final_votes_canary_height: 1,
            final_votes_canary_height: 1,
            epochs,
        })
    }
}
