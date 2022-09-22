use super::PublicKey;
use crate::utils::{Deserialize, Serialize, Stream};
use anyhow::Result;
use blake2::digest::{Update, VariableOutput};
use primitive_types::{U256, U512};

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug, Hash)]
pub struct Account {
    pub public_key: PublicKey,
}

const ZERO_ACCOUNT: Account = Account {
    public_key: PublicKey { value: [0; 32] },
};

impl Account {
    pub fn new() -> Self {
        Self {
            public_key: PublicKey::new(),
        }
    }

    pub fn zero() -> &'static Account {
        &ZERO_ACCOUNT
    }

    pub fn is_zero(&self) -> bool {
        self.public_key.is_zero()
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Account {
        Self {
            public_key: PublicKey::from_bytes(bytes),
        }
    }

    pub fn from_slice(bytes: &[u8]) -> Option<Self> {
        match PublicKey::from_slice(bytes) {
            Some(key) => Some(Account { public_key: key }),
            None => None,
        }
    }

    pub fn to_bytes(self) -> [u8; 32] {
        self.public_key.to_be_bytes()
    }

    pub fn as_bytes(&'_ self) -> &'_ [u8; 32] {
        self.public_key.as_bytes()
    }

    pub fn encode_account(&self) -> String {
        let mut number = U512::from_big_endian(self.public_key.as_bytes());
        let check = U512::from_little_endian(&self.account_checksum());
        number <<= 40;
        number = number | check;

        let mut result = String::with_capacity(65);

        for _i in 0..60 {
            let r = number.byte(0) & 0x1f_u8;
            number >>= 5;
            result.push(account_encode(r));
        }
        result.push_str("_onan"); // nano_
        result.chars().rev().collect()
    }

    fn account_checksum(&self) -> [u8; 5] {
        let mut check = [0u8; 5];
        let mut blake = blake2::VarBlake2b::new_keyed(&[], check.len());
        blake.update(self.public_key.as_bytes());
        blake.finalize_variable(|bytes| {
            check.copy_from_slice(bytes);
        });

        check
    }

    pub fn decode_account(source: impl AsRef<str>) -> Result<Account> {
        EncodedAccountStr(source.as_ref()).to_u512()?.to_account()
    }

    pub fn decode_hex(s: impl AsRef<str>) -> Result<Self> {
        let s = s.as_ref();
        if s.is_empty() || s.len() > 64 {
            bail!(
                "Invalid account string length. Expected <= 64 but was {}",
                s.len()
            );
        }

        let mut padded_string = String::new();
        let sanitized = if s.len() < 64 {
            for _ in 0..(64 - s.len()) {
                padded_string.push('0');
            }
            padded_string.push_str(s);
            &padded_string
        } else {
            s
        };

        let mut bytes = [0u8; 32];
        hex::decode_to_slice(sanitized, &mut bytes)?;
        Ok(Account::from_bytes(bytes))
    }
}

impl Serialize for Account {
    fn serialized_size() -> usize {
        PublicKey::serialized_size()
    }

    fn serialize(&self, stream: &mut dyn Stream) -> anyhow::Result<()> {
        self.public_key.serialize(stream)
    }
}

impl Deserialize for Account {
    type Target = Self;
    fn deserialize(stream: &mut dyn Stream) -> anyhow::Result<Account> {
        PublicKey::deserialize(stream).map(Self::from)
    }
}

impl From<PublicKey> for Account {
    fn from(public_key: PublicKey) -> Self {
        Account { public_key }
    }
}

impl From<&PublicKey> for Account {
    fn from(public_key: &PublicKey) -> Self {
        Account {
            public_key: *public_key,
        }
    }
}

struct EncodedAccountU512(U512);

impl EncodedAccountU512 {
    fn account_bytes(&self) -> [u8; 32] {
        let mut bytes_512 = [0u8; 64];
        (self.0 >> 40).to_big_endian(&mut bytes_512);
        let mut bytes_256 = [0u8; 32];
        bytes_256.copy_from_slice(&bytes_512[32..]);
        bytes_256
    }

    fn checksum_bytes(&self) -> [u8; 5] {
        [
            self.0.byte(0),
            self.0.byte(1),
            self.0.byte(2),
            self.0.byte(3),
            self.0.byte(4),
        ]
    }

    fn to_account(&self) -> Result<Account> {
        let account = Account::from_bytes(self.account_bytes());
        if account.account_checksum() == self.checksum_bytes() {
            Ok(account)
        } else {
            Err(anyhow!("invalid checksum"))
        }
    }
}

struct EncodedAccountStr<'a>(&'a str);
impl<'a> EncodedAccountStr<'a> {
    fn is_valid(&self) -> bool {
        self.0.len() > 4
            && self.has_valid_prefix()
            && self.is_length_valid()
            && self.is_first_digit_valid()
    }

    fn has_valid_prefix(&self) -> bool {
        self.has_xrb_prefix() || self.has_nano_prefix() || self.has_node_id_prefix()
    }

    fn has_xrb_prefix(&self) -> bool {
        self.0.starts_with("xrb_") || self.0.starts_with("xrb-")
    }

    fn has_nano_prefix(&self) -> bool {
        self.0.starts_with("nano_") || self.0.starts_with("nano-")
    }

    fn has_node_id_prefix(&self) -> bool {
        self.0.starts_with("node_")
    }

    fn is_length_valid(&self) -> bool {
        if self.has_xrb_prefix() && self.0.chars().count() != 64 {
            return false;
        }
        if self.has_nano_prefix() && self.0.chars().count() != 65 {
            return false;
        }
        true
    }

    fn prefix_len(&self) -> usize {
        if self.has_xrb_prefix() {
            4
        } else {
            5
        }
    }

    fn first_digit(&self) -> Option<char> {
        self.0.chars().nth(self.prefix_len())
    }

    fn is_first_digit_valid(&self) -> bool {
        matches!(self.first_digit(), Some('1') | Some('3'))
    }

    fn chars_after_prefix(&'_ self) -> impl Iterator<Item = char> + '_ {
        self.0.chars().skip(self.prefix_len())
    }

    fn to_u512(&self) -> Result<EncodedAccountU512> {
        if !self.is_valid() {
            bail!("invalid account string");
        }

        let mut number = U512::default();
        for character in self.chars_after_prefix() {
            match self.decode_byte(character) {
                Some(byte) => {
                    number <<= 5;
                    number = number + byte;
                }
                None => bail!("invalid hex string"),
            }
        }
        Ok(EncodedAccountU512(number))
    }

    fn decode_byte(&self, character: char) -> Option<u8> {
        if character.is_ascii() {
            let character = character as u8;
            if (0x30..0x80).contains(&character) {
                let byte: u8 = account_decode(character);
                if byte != b'~' {
                    return Some(byte);
                }
            }
        }

        None
    }
}

const ACCOUNT_LOOKUP: &[char] = &[
    '1', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k',
    'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'w', 'x', 'y', 'z',
];

const ACCOUNT_REVERSE: &[char] = &[
    '~', '0', '~', '1', '2', '3', '4', '5', '6', '7', '~', '~', '~', '~', '~', '~', '~', '~', '~',
    '~', '~', '~', '~', '~', '~', '~', '~', '~', '~', '~', '~', '~', '~', '~', '~', '~', '~', '~',
    '~', '~', '~', '~', '~', '~', '~', '~', '~', '~', '~', '8', '9', ':', ';', '<', '=', '>', '?',
    '@', 'A', 'B', '~', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', '~', 'L', 'M', 'N', 'O', '~',
    '~', '~', '~', '~',
];

fn account_encode(value: u8) -> char {
    ACCOUNT_LOOKUP[value as usize]
}

fn account_decode(value: u8) -> u8 {
    let mut result = ACCOUNT_REVERSE[(value - 0x30) as usize] as u8;
    if result != b'~' {
        result -= 0x30;
    }
    result
}

impl From<u64> for Account {
    fn from(value: u64) -> Self {
        let mut key = PublicKey::new();
        key.value[24..].copy_from_slice(&value.to_be_bytes());
        Account::from(key)
    }
}

impl From<U256> for Account {
    fn from(value: U256) -> Self {
        PublicKey::from(value).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // original test: account.encode_zero
    #[test]
    fn encode_zero() {
        let account = Account::new();
        let encoded = account.encode_account();
        assert_eq!(
            encoded,
            "nano_1111111111111111111111111111111111111111111111111111hifc8npp"
        );
        let copy = Account::decode_account(&encoded).expect("decode failed");
        assert_eq!(account, copy);
    }

    // original test: account.encode_all
    #[test]
    fn encode_all() {
        let account = Account::from_bytes([0xFF; 32]);
        let encoded = account.encode_account();
        assert_eq!(
            encoded,
            "nano_3zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzc3yoon41"
        );
        let copy = Account::decode_account(&encoded).expect("decode failed");
        assert_eq!(account, copy);
    }

    // original test: account.encode_fail
    #[test]
    fn encode_fail() {
        let account = Account::new();
        let mut encoded = account.encode_account();
        encoded.replace_range(16..17, "x");
        assert!(Account::decode_account(&encoded).is_err());
    }

    #[test]
    fn encode_real_account() {
        let account =
            Account::decode_hex("E7F5F39D52AC32ADF978BBCF6EA50C7A5FBBDDCADE965C542808ADAE9DEF6B20")
                .unwrap();
        let encoded = account.encode_account();
        assert_eq!(
            encoded,
            "nano_3szoyggo7d3koqwqjgyhftkirykzqhgwoqnpdjc4i47fotgyyts1j8ab3mti"
        );
        assert_eq!(
            Account::decode_account(&encoded).expect("could not decode"),
            account
        );
    }

    #[test]
    fn decode_less_than_64_chars() {
        let account = Account::decode_hex("AA").unwrap();
        assert_eq!(
            account.to_bytes(),
            [
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0xAA
            ]
        )
    }
}
