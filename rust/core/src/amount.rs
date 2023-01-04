use crate::utils::{Deserialize, Serialize, Stream};
use anyhow::Result;
use once_cell::sync::Lazy;

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug, Ord)]
pub struct Amount {
    value: u128, // native endian!
}

impl Amount {
    pub const MAX: Amount = Amount::new(u128::MAX);

    pub const fn new(value: u128) -> Self {
        Self { value }
    }

    pub fn zero() -> Self {
        Self::new(0)
    }

    pub fn is_zero(&self) -> bool {
        *self == Self::zero()
    }

    pub fn from_be_bytes(bytes: [u8; 16]) -> Self {
        Self {
            value: u128::from_be_bytes(bytes),
        }
    }

    pub fn from_le_bytes(bytes: [u8; 16]) -> Self {
        Self {
            value: u128::from_le_bytes(bytes),
        }
    }

    pub fn to_be_bytes(self) -> [u8; 16] {
        self.value.to_be_bytes()
    }

    pub fn to_le_bytes(self) -> [u8; 16] {
        self.value.to_le_bytes()
    }

    pub fn encode_hex(&self) -> String {
        format!("{:032X}", self.value)
    }

    pub fn decode_hex(s: impl AsRef<str>) -> Result<Self> {
        let value = u128::from_str_radix(s.as_ref(), 16)?;
        Ok(Amount::new(value))
    }

    pub fn decode_dec(s: impl AsRef<str>) -> Result<Self> {
        Ok(Self::new(s.as_ref().parse::<u128>()?))
    }

    pub fn to_string_dec(self) -> String {
        self.value.to_string()
    }

    pub fn number(&self) -> u128 {
        self.value
    }

    pub fn format_balance(&self, precision: usize) -> String {
        let precision = std::cmp::min(precision, 30);
        if self.value == 0 || self.value >= *MXRB_RATIO / num_traits::pow(10, precision) {
            let whole = self.value / *MXRB_RATIO;
            let decimals = self.value % *MXRB_RATIO;
            let mut buf = num_format::Buffer::default();
            buf.write_formatted(&whole, &num_format::Locale::en);
            let mut result = buf.to_string();
            if decimals != 0 && precision > 0 {
                result.push('.');
                let decimals_string = format!("{:030}", decimals);
                let trimmed = decimals_string.trim_end_matches('0');
                let decimals_count = std::cmp::min(
                    precision,
                    trimmed[..std::cmp::min(precision, trimmed.len())].len(),
                );
                result.push_str(&decimals_string[..decimals_count]);
            }
            result
        } else if precision == 0 {
            "< 1".to_owned()
        } else {
            format!("< 0.{:0width$}", 1, width = precision)
        }
    }

    pub fn wrapping_add(&self, other: Amount) -> Amount {
        self.value.wrapping_add(other.value).into()
    }

    pub fn wrapping_sub(&self, other: Amount) -> Amount {
        self.value.wrapping_sub(other.value).into()
    }

    pub unsafe fn from_ptr(ptr: *const u8) -> Self {
        let mut bytes = [0; 16];
        bytes.copy_from_slice(std::slice::from_raw_parts(ptr, 16));
        Amount::from_be_bytes(bytes)
    }
}

impl From<u128> for Amount {
    fn from(value: u128) -> Self {
        Amount::new(value)
    }
}

impl Serialize for Amount {
    fn serialized_size() -> usize {
        std::mem::size_of::<u128>()
    }

    fn serialize(&self, stream: &mut dyn Stream) -> Result<()> {
        stream.write_bytes(&self.value.to_be_bytes())
    }
}

impl Deserialize for Amount {
    type Target = Self;
    fn deserialize(stream: &mut dyn Stream) -> Result<Self> {
        let mut buffer = [0u8; 16];
        let len = buffer.len();
        stream.read_bytes(&mut buffer, len)?;
        Ok(Amount::new(u128::from_be_bytes(buffer)))
    }
}

impl std::ops::AddAssign for Amount {
    fn add_assign(&mut self, rhs: Self) {
        self.value += rhs.value;
    }
}

impl std::ops::Add for Amount {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Amount::new(self.value + rhs.value)
    }
}

impl std::ops::Sub for Amount {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Amount::new(self.value - rhs.value)
    }
}

impl std::cmp::PartialOrd for Amount {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

pub static XRB_RATIO: Lazy<u128> = Lazy::new(|| str::parse("1000000000000000000000000").unwrap()); // 10^24
pub static KXRB_RATIO: Lazy<u128> =
    Lazy::new(|| str::parse("1000000000000000000000000000").unwrap()); // 10^27
pub static MXRB_RATIO: Lazy<u128> =
    Lazy::new(|| str::parse("1000000000000000000000000000000").unwrap()); // 10^30
pub static GXRB_RATIO: Lazy<u128> =
    Lazy::new(|| str::parse("1000000000000000000000000000000000").unwrap()); // 10^33

#[cfg(test)]
mod tests {
    use crate::{KXRB_RATIO, XRB_RATIO};

    use super::*;

    #[test]
    fn format_balance() {
        assert_eq!("0", Amount::new(0).format_balance(2));
        assert_eq!(
            "340,282,366",
            Amount::decode_hex("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF")
                .unwrap()
                .format_balance(0)
        );
        assert_eq!(
            "340,282,366.920938463463374607431768211455",
            Amount::decode_hex("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF")
                .unwrap()
                .format_balance(64)
        );
        assert_eq!(
            "340,282,366",
            Amount::decode_hex("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFE")
                .unwrap()
                .format_balance(0)
        );
        assert_eq!(
            "340,282,366.920938463463374607431768211454",
            Amount::decode_hex("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFE")
                .unwrap()
                .format_balance(64)
        );
        assert_eq!(
            "170,141,183",
            Amount::decode_hex("7FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFE")
                .unwrap()
                .format_balance(0)
        );
        assert_eq!(
            "170,141,183.460469231731687303715884105726",
            Amount::decode_hex("7FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFE")
                .unwrap()
                .format_balance(64)
        );
        assert_eq!(
            "1",
            Amount::decode_dec("1000000000000000000000000000000")
                .unwrap()
                .format_balance(2)
        );
        assert_eq!(
            "1.2",
            Amount::decode_dec("1200000000000000000000000000000")
                .unwrap()
                .format_balance(2)
        );
        assert_eq!(
            "1.23",
            Amount::decode_dec("1230000000000000000000000000000")
                .unwrap()
                .format_balance(2)
        );
        assert_eq!(
            "1.2",
            Amount::decode_dec("1230000000000000000000000000000")
                .unwrap()
                .format_balance(1)
        );
        assert_eq!(
            "1",
            Amount::decode_dec("1230000000000000000000000000000")
                .unwrap()
                .format_balance(0)
        );
        assert_eq!("< 0.01", Amount::new(*XRB_RATIO * 10).format_balance(2));
        assert_eq!("< 0.1", Amount::new(*XRB_RATIO * 10).format_balance(1));
        assert_eq!("< 1", Amount::new(*XRB_RATIO * 10).format_balance(0));
        assert_eq!("< 0.01", Amount::new(*XRB_RATIO * 9999).format_balance(2));
        assert_eq!("< 0.001", Amount::new(1).format_balance(3));
        assert_eq!("0.01", Amount::new(*XRB_RATIO * 10000).format_balance(2));
        assert_eq!(
            "123,456,789",
            Amount::new(*MXRB_RATIO * 123456789).format_balance(2)
        );
        assert_eq!(
            "123,456,789.12",
            Amount::new(*MXRB_RATIO * 123456789 + *KXRB_RATIO * 123).format_balance(2)
        );
    }
}
