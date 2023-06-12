use rand::{thread_rng, Rng};
use rsnano_core::RawKey;

/// The fan spreads a key out over the heap to decrease the likelihood of it being recovered by memory inspection
pub struct Fan {
    values: Vec<RawKey>,
}

impl Fan {
    pub fn new(mut key: RawKey, count: usize) -> Self {
        let mut values = Vec::with_capacity(count);
        let mut rng = thread_rng();
        for _ in 1..count {
            let entry = RawKey::from_bytes(rng.gen());
            key ^= entry;
            values.push(entry);
        }
        values.push(key);

        Self { values }
    }

    pub fn value(&self) -> RawKey {
        let mut key = RawKey::zero();
        for i in self.values.iter() {
            key ^= *i;
        }
        key
    }

    pub fn value_set(&mut self, new_value: RawKey) {
        let old_value = self.value();
        self.values[0] ^= old_value;
        self.values[0] ^= new_value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconstitute_fan() {
        let value0 = RawKey::from_bytes([0; 32]);
        let fan = Fan::new(value0, 1024);
        for i in fan.values.iter() {
            assert_ne!(i, &value0);
        }
        let value1 = fan.value();
        assert_eq!(value0, value1);
    }

    #[test]
    fn change_fan() {
        let value0 = RawKey::from_bytes([0; 32]);
        let value1 = RawKey::from_bytes([1; 32]);
        let mut fan = Fan::new(value0, 1024);
        assert_eq!(fan.values.len(), 1024);
        assert_eq!(fan.value(), value0);
        fan.value_set(value1);
        assert_eq!(fan.value(), value1);
    }
}
