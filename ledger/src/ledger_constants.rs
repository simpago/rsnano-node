use rsnano_core::{
    epoch_v1_link, epoch_v2_link,
    utils::{get_env_or_default_string, UnixTimestamp},
    work::{WorkThresholds, WORK_THRESHOLDS_STUB},
    Account, Amount, Block, BlockDetails, BlockHash, BlockSideband, Epoch, Epochs, Networks,
    PublicKey, SavedBlock, DEV_GENESIS_KEY,
};
use std::sync::LazyLock;

static BETA_PUBLIC_KEY_DATA: &str =
    "259A438A8F9F9226130C84D902C237AF3E57C0981C7D709C288046B110D8C8AC";

static TEST_PUBLIC_KEY_DATA: LazyLock<String> = LazyLock::new(|| {
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
	"source": "259A438A8F9F9226130C84D902C237AF3E57C0981C7D709C288046B110D8C8AC",
	"representative": "nano_1betag7az9wk6rbis38s1d35hdsycz1bi95xg4g4j148p6afjk7embcurda4",
	"account": "nano_1betag7az9wk6rbis38s1d35hdsycz1bi95xg4g4j148p6afjk7embcurda4",
	"work": "e87a3ce39b43b84c",
	"signature": "BC588273AC689726D129D3137653FB319B6EE6DB178F97421D11D075B46FD52B6748223C8FF4179399D35CB1A8DF36F759325BD2D3D4504904321FAFB71D7602"
    }"###;

static LIVE_GENESIS_DATA: &str = r###"{
	"type": "open",
	"source": "E89208DD038FBB269987689621D52292AE9C35941A7484756ECCED92A65093BA",
	"representative": "xrb_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3",
	"account": "xrb_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3",
	"work": "62f05417dd3fb691",
	"signature": "9F0C933C8ADE004D808EA1985FA746A7E95BA2A38F867640F53EC8F180BDFE9E2C1268DEAD7C2664F356E37ABA362BC58E46DBA03E523A7B5A19E4B6EB12BB02"
    }"###;

static TEST_GENESIS_DATA: LazyLock<String> = LazyLock::new(|| {
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

pub static LEDGER_CONSTANTS_STUB: LazyLock<LedgerConstants> =
    LazyLock::new(|| LedgerConstants::new(WORK_THRESHOLDS_STUB.clone(), Networks::NanoDevNetwork));

pub static DEV_GENESIS_BLOCK: LazyLock<SavedBlock> =
    LazyLock::new(|| LEDGER_CONSTANTS_STUB.genesis_block.clone());

pub static DEV_GENESIS_ACCOUNT: LazyLock<Account> =
    LazyLock::new(|| DEV_GENESIS_BLOCK.account_field().unwrap());
#[allow(dead_code)]
pub static DEV_GENESIS_PUB_KEY: LazyLock<PublicKey> =
    LazyLock::new(|| DEV_GENESIS_BLOCK.account_field().unwrap().into());
pub static DEV_GENESIS_HASH: LazyLock<BlockHash> = LazyLock::new(|| DEV_GENESIS_BLOCK.hash());

fn parse_block_from_genesis_data(genesis_data: &str) -> anyhow::Result<Block> {
    let block = serde_json::from_str(genesis_data)?;
    Ok(block)
}

#[cfg(test)]
mod tests {
    use rsnano_core::BlockType;

    use super::*;

    #[test]
    fn test_parse_block() {
        let block_str = r###"{"type": "open", "source": "37FCEA4DA94F1635484EFCBA57483C4C654F573B435C09D8AACE1CB45E63FFB1", "representative": "nano_1fzwxb8tkmrp8o66xz7tcx65rm57bxdmpitw39ecomiwpjh89zxj33juzt6p", "account": "nano_1fzwxb8tkmrp8o66xz7tcx65rm57bxdmpitw39ecomiwpjh89zxj33juzt6p", "work": "ef0547d86748c71b", "signature": "13E33D1ADA50A79B64741C5159C0C0AFE0515581B47ABD73676FE02A1D600CDB637050D37BF92C9629649AE92949814BB57C6B5B0A44BF76E2F33043A3DF2D01"}"###;
        let block = parse_block_from_genesis_data(block_str).unwrap();
        assert_eq!(block.block_type(), BlockType::LegacyOpen);
    }
}

#[derive(Clone)]
pub struct LedgerConstants {
    pub work: WorkThresholds,
    pub genesis_block: SavedBlock,
    pub genesis_account: Account,
    pub genesis_amount: Amount,
    pub burn_account: Account,
    pub epochs: Epochs,
}

pub fn genesis_sideband(genesis_account: Account) -> BlockSideband {
    BlockSideband {
        height: 1,
        timestamp: UnixTimestamp::now().as_u64(),
        successor: BlockHash::zero(),
        account: genesis_account,
        balance: Amount::MAX,
        details: BlockDetails::new(Epoch::Epoch0, false, false, false),
        source_epoch: Epoch::Epoch0,
    }
}

impl LedgerConstants {
    pub fn new(work: WorkThresholds, network: Networks) -> Self {
        let dev_genesis_block = parse_block_from_genesis_data(DEV_GENESIS_DATA).unwrap();
        let beta_genesis_block = parse_block_from_genesis_data(BETA_GENESIS_DATA).unwrap();
        let live_genesis_block = parse_block_from_genesis_data(LIVE_GENESIS_DATA).unwrap();
        let test_genesis_block = parse_block_from_genesis_data(TEST_GENESIS_DATA.as_str()).unwrap();

        let genesis_block = match network {
            Networks::NanoDevNetwork => dev_genesis_block,
            Networks::NanoBetaNetwork => beta_genesis_block,
            Networks::NanoTestNetwork => test_genesis_block,
            Networks::NanoLiveNetwork => live_genesis_block,
            Networks::Invalid => panic!("invalid network"),
        };
        let genesis_account = genesis_block.account_field().unwrap();

        let nano_beta_account = Account::decode_hex(BETA_PUBLIC_KEY_DATA).unwrap();
        let nano_test_account = Account::decode_hex(TEST_PUBLIC_KEY_DATA.as_str()).unwrap();

        let mut epochs = Epochs::new();

        let epoch_1_signer = PublicKey::from(genesis_account);
        let epoch_link_v1 = epoch_v1_link();

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
        let epoch_link_v2 = epoch_v2_link();

        epochs.add(Epoch::Epoch1, epoch_1_signer, epoch_link_v1);
        epochs.add(Epoch::Epoch2, epoch_2_signer, epoch_link_v2);

        let genesis_block = SavedBlock::new(genesis_block, genesis_sideband(genesis_account));

        Self {
            work,
            genesis_block,
            genesis_account,
            genesis_amount: Amount::raw(u128::MAX),
            burn_account: Account::zero(),
            epochs,
        }
    }

    pub fn live() -> Self {
        Self::new(
            WorkThresholds::publish_full().clone(),
            Networks::NanoLiveNetwork,
        )
    }

    pub fn beta() -> Self {
        Self::new(
            WorkThresholds::publish_beta().clone(),
            Networks::NanoBetaNetwork,
        )
    }

    pub fn test() -> Self {
        Self::new(
            WorkThresholds::publish_test().clone(),
            Networks::NanoTestNetwork,
        )
    }

    pub fn dev() -> Self {
        Self::new(
            WorkThresholds::publish_dev().clone(),
            Networks::NanoDevNetwork,
        )
    }

    pub fn unit_test() -> Self {
        Self::new(WORK_THRESHOLDS_STUB.clone(), Networks::NanoDevNetwork)
    }
}
