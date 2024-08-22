use rsnano_core::{
    utils::{BufferReader, Deserialize, StreamExt},
    Account, Amount, Networks, PublicKey,
};
use rsnano_ledger::RepWeightCache;
use std::collections::HashMap;
use tracing::info;

pub(crate) fn get_bootstrap_weights(network: Networks) -> (u64, HashMap<PublicKey, Amount>) {
    let buffer = get_bootstrap_weights_bin(network);
    deserialize_bootstrap_weights(buffer)
}

fn get_bootstrap_weights_bin(network: Networks) -> &'static [u8] {
    if network == Networks::NanoLiveNetwork {
        include_bytes!("../../../../rep_weights_live.bin")
    } else {
        include_bytes!("../../../../rep_weights_beta.bin")
    }
}

fn deserialize_bootstrap_weights(buffer: &[u8]) -> (u64, HashMap<PublicKey, Amount>) {
    let mut reader = BufferReader::new(buffer);
    let mut weights = HashMap::new();
    let mut max_blocks = 0;
    if let Ok(count) = reader.read_u128_be() {
        max_blocks = count as u64;
        loop {
            let Ok(account) = PublicKey::deserialize(&mut reader) else {
                break;
            };
            let Ok(weight) = Amount::deserialize(&mut reader) else {
                break;
            };
            weights.insert(account, weight);
        }
    }

    (max_blocks, weights)
}

pub(crate) fn log_bootstrap_weights(weight_cache: &RepWeightCache) {
    let mut bootstrap_weights = weight_cache.bootstrap_weights();
    if !bootstrap_weights.is_empty() {
        info!(
            "Initial bootstrap height: {}",
            weight_cache.bootstrap_weight_max_blocks()
        );
        info!("Current ledger height:    {}", weight_cache.block_count());

        // Use bootstrap weights if initial bootstrap is not completed
        if weight_cache.use_bootstrap_weights() {
            info!("Using predefined representative weights, since block count is less than bootstrap threshold");
            info!("************************************ Bootstrap weights ************************************");
            // Sort the weights
            let mut sorted_weights = bootstrap_weights.drain().collect::<Vec<_>>();
            sorted_weights.sort_by(|(_, weight_a), (_, weight_b)| weight_b.cmp(weight_a));

            for (rep, weight) in sorted_weights {
                info!(
                    "Using bootstrap rep weight: {} -> {}",
                    Account::from(&rep).encode_account(),
                    weight.format_balance(0)
                );
            }
            info!("************************************ ================= ************************************");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_weights_bin() {
        assert_eq!(
            get_bootstrap_weights_bin(Networks::NanoLiveNetwork).len(),
            6256,
            "expected live weights don't match'"
        );
        assert_eq!(
            get_bootstrap_weights_bin(Networks::NanoBetaNetwork).len(),
            0,
            "expected beta weights don't match'"
        );
    }

    #[test]
    fn bootstrap_weights() {
        let (max_blocks, weights) = get_bootstrap_weights(Networks::NanoLiveNetwork);
        assert_eq!(weights.len(), 130);
        assert_eq!(max_blocks, 184_789_962);
    }
}
