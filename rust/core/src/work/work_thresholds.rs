use crate::{
    BlockDetails, BlockEnum, BlockType, Difficulty, DifficultyV1, Epoch, Networks, Root,
    StubDifficulty, WorkVersion,
};
use once_cell::sync::Lazy;
use std::cmp::{max, min};

pub static WORK_THRESHOLDS_STUB: Lazy<WorkThresholds> = Lazy::new(|| WorkThresholds::new_stub());

pub struct WorkThresholds {
    pub epoch_1: u64,
    pub epoch_2: u64,
    pub epoch_2_receive: u64,

    // Automatically calculated. The base threshold is the maximum of all thresholds and is used for all work multiplier calculations
    pub base: u64,

    // Automatically calculated. The entry threshold is the minimum of all thresholds and defines the required work to enter the node, but does not guarantee a block is processed
    pub entry: u64,
    pub difficulty: Box<dyn Difficulty>,
}

impl Clone for WorkThresholds {
    fn clone(&self) -> Self {
        Self {
            epoch_1: self.epoch_1,
            epoch_2: self.epoch_2,
            epoch_2_receive: self.epoch_2_receive,
            base: self.base,
            entry: self.entry,
            difficulty: self.difficulty.clone(),
        }
    }
}

impl PartialEq for WorkThresholds {
    fn eq(&self, other: &Self) -> bool {
        self.epoch_1 == other.epoch_1
            && self.epoch_2 == other.epoch_2
            && self.epoch_2_receive == other.epoch_2_receive
            && self.base == other.base
            && self.entry == other.entry
            && self.difficulty.get_difficulty(&Root::default(), 0)
                == other.difficulty.get_difficulty(&Root::default(), 0)
    }
}

impl std::fmt::Debug for WorkThresholds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkThresholds")
            .field("epoch_1", &self.epoch_1)
            .field("epoch_2", &self.epoch_2)
            .field("epoch_2_receive", &self.epoch_2_receive)
            .field("base", &self.base)
            .field("entry", &self.entry)
            .finish()
    }
}

static PUBLISH_FULL: Lazy<WorkThresholds> = Lazy::new(|| {
    WorkThresholds::new(
        0xffffffc000000000,
        0xfffffff800000000, // 8x higher than epoch_1
        0xfffffe0000000000, // 8x lower than epoch_1
    )
});

static PUBLISH_BETA: Lazy<WorkThresholds> = Lazy::new(|| {
    WorkThresholds::new(
        0xfffff00000000000, // 64x lower than publish_full.epoch_1
        0xfffff00000000000, // same as epoch_1
        0xffffe00000000000, // 2x lower than epoch_1
    )
});

static PUBLISH_DEV: Lazy<WorkThresholds> = Lazy::new(|| {
    WorkThresholds::new(
        0xfe00000000000000, // Very low for tests
        0xffc0000000000000, // 8x higher than epoch_1
        0xf000000000000000, // 8x lower than epoch_1
    )
});

static PUBLISH_TEST: Lazy<WorkThresholds> = Lazy::new(|| {
    WorkThresholds::new(
        get_env_threshold_or_default("NANO_TEST_EPOCH_1", 0xffffffc000000000),
        get_env_threshold_or_default("NANO_TEST_EPOCH_2", 0xfffffff800000000), // 8x higher than epoch_1
        get_env_threshold_or_default("NANO_TEST_EPOCH_2_RECV", 0xfffffe0000000000), // 8x lower than epoch_1
    )
});

fn get_env_threshold_or_default(variable_name: &str, default_value: u64) -> u64 {
    match std::env::var(variable_name) {
        Ok(value) => parse_hex_u64(value).expect("could not parse difficulty env var"),
        Err(_) => default_value,
    }
}

fn parse_hex_u64(value: impl AsRef<str>) -> Result<u64, std::num::ParseIntError> {
    let s = value.as_ref();
    let s = s.strip_prefix("0x").unwrap_or(s);
    u64::from_str_radix(s, 16)
}

impl WorkThresholds {
    pub fn publish_full() -> &'static WorkThresholds {
        &PUBLISH_FULL
    }

    pub fn publish_beta() -> &'static WorkThresholds {
        &PUBLISH_BETA
    }

    pub fn publish_dev() -> &'static WorkThresholds {
        &PUBLISH_DEV
    }

    pub fn publish_test() -> &'static WorkThresholds {
        &PUBLISH_TEST
    }
}

impl WorkThresholds {
    pub fn new(epoch_1: u64, epoch_2: u64, epoch_2_receive: u64) -> Self {
        Self::with_difficulty(
            Box::<DifficultyV1>::default(),
            epoch_1,
            epoch_2,
            epoch_2_receive,
        )
    }

    pub fn default_for(network: Networks) -> Self {
        match network {
            Networks::NanoDevNetwork => Self::publish_dev().clone(),
            Networks::NanoBetaNetwork => Self::publish_beta().clone(),
            Networks::NanoLiveNetwork => Self::publish_full().clone(),
            Networks::NanoTestNetwork => Self::publish_test().clone(),
            Networks::Invalid => {
                panic!("no default network set")
            }
        }
    }

    pub fn new_stub() -> Self {
        WorkThresholds::with_difficulty(
            Box::new(StubDifficulty::new()),
            0xfe00000000000000, // Very low for tests
            0xffc0000000000000, // 8x higher than epoch_1
            0xf000000000000000, // 8x lower than epoch_1
        )
    }

    pub fn with_difficulty(
        difficulty: Box<dyn Difficulty>,
        epoch_1: u64,
        epoch_2: u64,
        epoch_2_receive: u64,
    ) -> Self {
        Self {
            epoch_1,
            epoch_2,
            epoch_2_receive,
            base: max(max(epoch_1, epoch_2), epoch_2_receive),
            entry: min(min(epoch_1, epoch_2), epoch_2_receive),
            difficulty,
        }
    }

    pub fn threshold_entry(&self, block_type: BlockType, work_version: WorkVersion) -> u64 {
        match block_type {
            BlockType::State => match work_version {
                WorkVersion::Work1 => self.entry,
                _ => {
                    debug_assert!(false, "Invalid version specified to work_threshold_entry");
                    u64::MAX
                }
            },
            _ => self.epoch_1,
        }
    }

    pub fn threshold(&self, details: &BlockDetails) -> u64 {
        match details.epoch {
            Epoch::Epoch2 => {
                if details.is_receive || details.is_epoch {
                    self.epoch_2_receive
                } else {
                    self.epoch_2
                }
            }
            Epoch::Epoch1 | Epoch::Epoch0 => self.epoch_1,
            _ => {
                debug_assert!(
                    false,
                    "Invalid epoch specified to work_v1 ledger work_threshold"
                );
                u64::MAX
            }
        }
    }

    pub fn threshold2(&self, work_version: WorkVersion, details: &BlockDetails) -> u64 {
        match work_version {
            WorkVersion::Work1 => self.threshold(details),
            _ => {
                // Invalid version specified to ledger work_threshold
                debug_assert!(false);
                u64::MAX
            }
        }
    }

    pub fn threshold_base(&self, work_version: WorkVersion) -> u64 {
        match work_version {
            WorkVersion::Work1 => self.base,
            _ => {
                debug_assert!(false, "Invalid version specified to work_threshold_base");
                u64::MAX
            }
        }
    }

    pub fn normalized_multiplier(&self, multiplier: f64, threshold: u64) -> f64 {
        debug_assert!(multiplier >= 1f64);
        /* Normalization rules
        ratio = multiplier of max work threshold (send epoch 2) from given threshold
        i.e. max = 0xfe00000000000000, given = 0xf000000000000000, ratio = 8.0
        normalized = (multiplier + (ratio - 1)) / ratio;
        Epoch 1
        multiplier	 | normalized
        1.0 		 | 1.0
        9.0 		 | 2.0
        25.0 		 | 4.0
        Epoch 2 (receive / epoch subtypes)
        multiplier	 | normalized
        1.0 		 | 1.0
        65.0 		 | 2.0
        241.0 		 | 4.0
        */
        if threshold == self.epoch_1 || threshold == self.epoch_2_receive {
            let ratio = DifficultyV1::to_multiplier(self.epoch_2, threshold);
            debug_assert!(ratio >= 1f64);
            let result = (multiplier + (ratio - 1f64)) / ratio;
            debug_assert!(result >= 1f64);
            result
        } else {
            multiplier
        }
    }

    pub fn denormalized_multiplier(&self, multiplier: f64, threshold: u64) -> f64 {
        debug_assert!(multiplier >= 1f64);
        if threshold == self.epoch_1 || threshold == self.epoch_2_receive {
            let ratio = DifficultyV1::to_multiplier(self.epoch_2, threshold);
            debug_assert!(ratio >= 1f64);
            let result = multiplier * ratio + 1f64 - ratio;
            debug_assert!(result >= 1f64);
            result
        } else {
            multiplier
        }
    }

    pub fn difficulty(&self, work_version: WorkVersion, root: &Root, work: u64) -> u64 {
        match work_version {
            WorkVersion::Work1 => self.difficulty.get_difficulty(root, work),
            _ => {
                debug_assert!(false, "Invalid version specified to work_difficulty");
                0
            }
        }
    }

    pub fn difficulty_block(&self, block: &BlockEnum) -> u64 {
        self.difficulty(block.work_version(), &block.root(), block.work())
    }

    //todo return true if valid!
    pub fn validate_entry(&self, work_version: WorkVersion, root: &Root, work: u64) -> bool {
        self.difficulty(work_version, root, work)
            < self.threshold_entry(BlockType::State, work_version)
    }

    //todo return true if valid!
    pub fn validate_entry_block(&self, block: &BlockEnum) -> bool {
        let difficulty = self.difficulty_block(block);
        let threshold = self.threshold_entry(block.block_type(), block.work_version());
        let is_invalid = difficulty < threshold;
        is_invalid
    }

    pub fn is_valid_pow(&self, block: &BlockEnum, details: &BlockDetails) -> bool {
        self.difficulty_block(block) >= self.threshold2(block.work_version(), details)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Amount, BlockHash, JsonBlock};

    #[test]
    fn test_parse_threshold() {
        assert_eq!(parse_hex_u64("0xffffffc000000000"), Ok(0xffffffc000000000));
        assert_eq!(parse_hex_u64("0xFFFFFFC000000000"), Ok(0xffffffc000000000));
        assert_eq!(parse_hex_u64("FFFFFFC000000000"), Ok(0xffffffc000000000));
    }

    #[test]
    fn difficulty_block() {
        let block = BlockEnum::new_test_instance();
        assert_eq!(
            WorkThresholds::publish_full().difficulty_block(&block),
            9665579333895977632
        );
    }

    #[test]
    fn threshold_epoch0_send() {
        assert_eq!(
            WorkThresholds::publish_full().threshold2(
                WorkVersion::Work1,
                &BlockDetails {
                    epoch: Epoch::Epoch0,
                    is_send: true,
                    is_receive: false,
                    is_epoch: false
                }
            ),
            0xffffffc000000000
        );
    }

    #[test]
    fn threshold_epoch0_receive() {
        assert_eq!(
            WorkThresholds::publish_full().threshold2(
                WorkVersion::Work1,
                &BlockDetails {
                    epoch: Epoch::Epoch0,
                    is_send: false,
                    is_receive: true,
                    is_epoch: false
                }
            ),
            0xffffffc000000000
        );
    }

    #[test]
    fn threshold_epoch1_send() {
        assert_eq!(
            WorkThresholds::publish_full().threshold2(
                WorkVersion::Work1,
                &BlockDetails {
                    epoch: Epoch::Epoch1,
                    is_send: true,
                    is_receive: false,
                    is_epoch: false
                }
            ),
            0xffffffc000000000
        );
    }

    #[test]
    fn threshold_epoch1_receive() {
        assert_eq!(
            WorkThresholds::publish_full().threshold2(
                WorkVersion::Work1,
                &BlockDetails {
                    epoch: Epoch::Epoch1,
                    is_send: false,
                    is_receive: true,
                    is_epoch: false
                }
            ),
            0xffffffc000000000
        );
    }

    #[test]
    fn threshold_epoch2_send() {
        assert_eq!(
            WorkThresholds::publish_full().threshold2(
                WorkVersion::Work1,
                &BlockDetails {
                    epoch: Epoch::Epoch2,
                    is_send: true,
                    is_receive: false,
                    is_epoch: false
                }
            ),
            0xfffffff800000000
        );
    }

    #[test]
    fn threshold_epoch2_receive() {
        assert_eq!(
            WorkThresholds::publish_full().threshold2(
                WorkVersion::Work1,
                &BlockDetails {
                    epoch: Epoch::Epoch2,
                    is_send: false,
                    is_receive: true,
                    is_epoch: false
                }
            ),
            0xfffffe0000000000
        );
    }

    #[test]
    fn validate_real_block() {
        let json_block = r###"{
  "type": "send",
  "previous": "991CF190094C00F0B68E2E5F75F6BEE95A2E0BD93CEAA4A6734DB9F19B728948",
  "destination": "nano_13ezf4od79h1tgj9aiu4djzcmmguendtjfuhwfukhuucboua8cpoihmh8byo",
  "balance": "FD89D89D89D89D89D89D89D89D89D89D",
  "signature": "5B11B17DB9C8FE0CC58CAC6A6EECEF9CB122DA8A81C6D3DB1B5EE3AB065AA8F8CB1D6765C8EB91B58530C5FF5987AD95E6D34BB57F44257E20795EE412E61600",
  "work": "3c82cc724905ee95"
}"###;
        let block: BlockEnum = serde_json::from_str::<JsonBlock>(json_block)
            .unwrap()
            .into();
        let thresholds = WorkThresholds::publish_full();
        assert_eq!(
            block.hash(),
            BlockHash::decode_hex(
                "A170D51B94E00371ACE76E35AC81DC9405D5D04D4CEBC399AEACE07AE05DD293"
            )
            .unwrap()
        );
        assert_eq!(
            block.balance(),
            Amount::raw(337010421085160209006996005437231978653)
        );
        assert_eq!(thresholds.validate_entry_block(&block), false);
        assert_eq!(thresholds.difficulty_block(&block), 18446743921403126366);
    }
}
