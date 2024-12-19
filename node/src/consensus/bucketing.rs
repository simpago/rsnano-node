use rsnano_core::Amount;

pub(crate) struct Bucketing {
    minimums: Vec<Amount>,
}

impl Bucketing {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn bucket_index(&self, balance: Amount) -> usize {
        let result = self
            .minimums
            .iter()
            .enumerate()
            .rev()
            .find(|(_, minimum)| balance >= **minimum);

        match result {
            Some((index, _)) => index,
            None => {
                // There should always be a bucket with a minimum_balance of 0
                unreachable!()
            }
        }
    }

    pub fn bucket_count(&self) -> usize {
        self.minimums.len()
    }
}

impl Default for Bucketing {
    fn default() -> Self {
        let mut minimums = Vec::new();
        let mut build_region = |begin: u128, end: u128, count: usize| {
            let width = (end - begin) / (count as u128);
            for i in 0..count {
                let minimum_balance = begin + (i as u128 * width);
                minimums.push(minimum_balance.into())
            }
        };

        build_region(0, 1 << 79, 1);
        build_region(1 << 79, 1 << 88, 1);
        build_region(1 << 88, 1 << 92, 2);
        build_region(1 << 92, 1 << 96, 4);
        build_region(1 << 96, 1 << 100, 8);
        build_region(1 << 100, 1 << 104, 16);
        build_region(1 << 104, 1 << 108, 16);
        build_region(1 << 108, 1 << 112, 8);
        build_region(1 << 112, 1 << 116, 4);
        build_region(1 << 116, 1 << 120, 2);
        build_region(1 << 120, 1 << 127, 1);

        Self { minimums }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bucket_creation() {
        assert_eq!(Bucketing::new().bucket_count(), 63);
    }

    #[test]
    fn bucket_0() {
        assert_eq!(Bucketing::new().bucket_index(0.into()), 0);
        assert_eq!(Bucketing::new().bucket_index(1.into()), 0);
        assert_eq!(Bucketing::new().bucket_index(Amount::raw((1 << 79) - 1)), 0);
    }

    #[test]
    fn bucket_1() {
        assert_eq!(Bucketing::new().bucket_index(Amount::raw(1 << 79)), 1);
    }

    #[test]
    fn nano_index() {
        assert_eq!(Bucketing::new().bucket_index(Amount::nano(1)), 14);
    }

    #[test]
    fn knano_index() {
        assert_eq!(Bucketing::new().bucket_index(Amount::nano(1000)), 49);
    }

    #[test]
    fn max_index() {
        assert_eq!(Bucketing::new().bucket_index(Amount::MAX), 62);
    }
}
