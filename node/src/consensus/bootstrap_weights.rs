use rsnano_core::{Account, Amount, Networks, PublicKey};
use rsnano_ledger::RepWeightCache;
use std::collections::HashMap;
use tracing::info;

pub(crate) fn get_bootstrap_weights(network: Networks) -> (u64, HashMap<PublicKey, Amount>) {
    let buffer = get_bootstrap_weights_text(network);
    deserialize_bootstrap_weights(buffer)
}

fn get_bootstrap_weights_text(network: Networks) -> &'static str {
    if network == Networks::NanoLiveNetwork {
        include_str!("../../rep_weights_live.txt")
    } else {
        include_str!("../../rep_weights_beta.txt")
    }
}

fn deserialize_bootstrap_weights(buffer: &str) -> (u64, HashMap<PublicKey, Amount>) {
    let mut weights = HashMap::new();
    let mut first_line = true;
    let mut max_blocks = 0;
    for line in buffer.lines() {
        if first_line {
            max_blocks = line.parse().unwrap();
            first_line = false;
            continue;
        }

        let mut it = line.split(':');
        let account = Account::decode_account(it.next().unwrap()).unwrap();
        let weight = Amount::decode_dec(it.next().unwrap()).unwrap();
        weights.insert(account.into(), weight);
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
    fn bootstrap_weights_text() {
        assert_eq!(
            get_bootstrap_weights_text(Networks::NanoLiveNetwork).len(),
            13921,
            "expected live weights don't match'"
        );
        assert_eq!(
            get_bootstrap_weights_text(Networks::NanoBetaNetwork).len(),
            1161,
            "expected beta weights don't match'"
        );
    }

    #[test]
    fn bootstrap_weights() {
        let (max_blocks, weights) = get_bootstrap_weights(Networks::NanoLiveNetwork);
        assert_eq!(weights.len(), 135);
        assert_eq!(max_blocks, 204_137_485);
    }
}
