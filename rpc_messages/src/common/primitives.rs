use serde::{de::Visitor, Deserialize, Serialize};
use std::fmt::Debug;

#[macro_export]
macro_rules! rpc_number {
    ($name:ident, $type:ty, $visitor:ident) => {
        #[derive(Copy, Clone, PartialEq, Eq, Default, PartialOrd, Ord)]
        pub struct $name($type);

        impl $name{
            pub fn inner(&self) -> $type{
                self.0
            }
        }

        impl From<$type> for $name {
            fn from(value: $type) -> Self {
                Self(value)
            }
        }

        impl From<$name> for $type {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(&self.0.to_string())
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                deserializer.deserialize_str($visitor {})
            }
        }

        struct $visitor {}

        impl<'de> serde::de::Visitor<'de> for $visitor {
            type Value = $name;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str(stringify!($type))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let value = v
                    .parse::<$type>()
                    .map_err(|_| serde::de::Error::custom(stringify!("expected " $type)))?;
                Ok(value.into())
            }
        }
    };
}

rpc_number!(RpcU8, u8, RpcU8Visitor);
rpc_number!(RpcU16, u16, RpcU16Visitor);
rpc_number!(RpcU32, u32, RpcU32Visitor);
rpc_number!(RpcU64, u64, RpcU64Visitor);
rpc_number!(RpcUsize, usize, RpcUsizeVisitor);

#[derive(Copy, Clone, PartialEq, Default, PartialOrd)]
pub struct RpcF32(f32);

impl From<f32> for RpcF32 {
    fn from(value: f32) -> Self {
        Self(value)
    }
}

impl From<RpcF32> for f32 {
    fn from(value: RpcF32) -> Self {
        value.0
    }
}

impl Debug for RpcF32 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Serialize for RpcF32 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for RpcF32 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(F32Visitor {})
    }
}

struct F32Visitor {}

impl<'de> Visitor<'de> for F32Visitor {
    type Value = RpcF32;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("f32")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let value = v
            .parse::<f32>()
            .map_err(|_| serde::de::Error::custom("expected f32"))?;
        Ok(value.into())
    }
}

#[derive(Copy, Clone, PartialEq, Default, PartialOrd)]
pub struct RpcF64(f64);

impl RpcF64 {
    pub fn inner(&self) -> f64 {
        self.0
    }
}

impl From<f64> for RpcF64 {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl From<RpcF64> for f64 {
    fn from(value: RpcF64) -> Self {
        value.0
    }
}

impl Debug for RpcF64 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Serialize for RpcF64 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for RpcF64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(F64Visitor {})
    }
}

struct F64Visitor {}

impl<'de> Visitor<'de> for F64Visitor {
    type Value = RpcF64;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("f64")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let value = v
            .parse::<f64>()
            .map_err(|_| serde::de::Error::custom("expected f64"))?;
        Ok(value.into())
    }
}

/// Bool expressed as "1"=true and "0"=false
#[derive(Copy, Clone, PartialEq, Eq, Default)]
pub struct RpcBoolNumber(bool);

impl From<bool> for RpcBoolNumber {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

impl From<RpcBoolNumber> for bool {
    fn from(value: RpcBoolNumber) -> Self {
        value.0
    }
}

impl Debug for RpcBoolNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Serialize for RpcBoolNumber {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(if self.0 { "1" } else { "0" })
    }
}

impl<'de> Deserialize<'de> for RpcBoolNumber {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let result = deserializer.deserialize_str(BoolVisitor {})?;
        Ok(RpcBoolNumber(result))
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Default)]
pub struct RpcBool(bool);

impl RpcBool {
    pub fn inner(&self) -> bool {
        self.0
    }
}

impl From<bool> for RpcBool {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

impl From<RpcBool> for bool {
    fn from(value: RpcBool) -> Self {
        value.0
    }
}

impl Debug for RpcBool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Serialize for RpcBool {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(if self.0 { "true" } else { "false" })
    }
}

impl<'de> Deserialize<'de> for RpcBool {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let result = deserializer.deserialize_str(BoolVisitor {})?;
        Ok(RpcBool(result))
    }
}

struct BoolVisitor {}

impl<'de> Visitor<'de> for BoolVisitor {
    type Value = bool;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("bool")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match v {
            "1" | "true" => Ok(true),
            "0" | "false" => Ok(false),
            _ => Err(serde::de::Error::custom("bool expected")),
        }
    }
}

pub fn unwrap_u64_or(i: Option<RpcU64>, default_value: u64) -> u64 {
    i.map(|x| x.into()).unwrap_or(default_value)
}

pub fn unwrap_u64_or_max(i: Option<RpcU64>) -> u64 {
    i.unwrap_or(u64::MAX.into()).into()
}

pub fn unwrap_u64_or_zero(i: Option<RpcU64>) -> u64 {
    i.unwrap_or_default().into()
}

pub fn unwrap_bool_or_false(i: Option<RpcBool>) -> bool {
    i.unwrap_or_default().into()
}

pub fn unwrap_bool_or_true(i: Option<RpcBool>) -> bool {
    i.unwrap_or(true.into()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u16_serialize() {
        let value = RpcU16::from(123);
        assert_eq!(format!("{:?}", value), "123");
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(json, "\"123\"");
    }

    #[test]
    fn u16_deserialize() {
        let value: RpcU16 = serde_json::from_str("\"123\"").unwrap();
        assert_eq!(value, 123.into());
    }

    #[test]
    fn u64_serialize() {
        let value = RpcU64::from(123);
        assert_eq!(format!("{:?}", value), "123");
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(json, "\"123\"");
    }

    #[test]
    fn u64_deserialize() {
        let value: RpcU64 = serde_json::from_str("\"123\"").unwrap();
        assert_eq!(value, 123.into());
    }

    #[test]
    fn f32_serialize() {
        let value = RpcF32::from(1.23);
        assert_eq!(format!("{:?}", value), "1.23");
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(json, "\"1.23\"");
    }

    #[test]
    fn f32_deserialize() {
        let value: RpcF32 = serde_json::from_str("\"1.23\"").unwrap();
        assert_eq!(value, 1.23.into());
    }

    #[test]
    fn bool_number_serialize() {
        let true_value = RpcBoolNumber::from(true);
        let false_value = RpcBoolNumber::from(false);
        assert_eq!(format!("{:?}", true_value), "true");
        assert_eq!(format!("{:?}", false_value), "false");
        let json = serde_json::to_string(&true_value).unwrap();
        assert_eq!(json, "\"1\"");
        let json = serde_json::to_string(&false_value).unwrap();
        assert_eq!(json, "\"0\"");
    }

    #[test]
    fn bool_number_deserialize() {
        let a: RpcBoolNumber = serde_json::from_str("\"1\"").unwrap();
        let b: RpcBoolNumber = serde_json::from_str("\"0\"").unwrap();
        let c: RpcBoolNumber = serde_json::from_str("\"true\"").unwrap();
        let d: RpcBoolNumber = serde_json::from_str("\"false\"").unwrap();
        assert_eq!(a, true.into());
        assert_eq!(b, false.into());
        assert_eq!(c, true.into());
        assert_eq!(d, false.into());
    }

    #[test]
    fn bool_serialize() {
        let true_value = RpcBool::from(true);
        let false_value = RpcBool::from(false);
        assert_eq!(format!("{:?}", true_value), "true");
        assert_eq!(format!("{:?}", false_value), "false");
        let json = serde_json::to_string(&true_value).unwrap();
        assert_eq!(json, "\"true\"");
        let json = serde_json::to_string(&false_value).unwrap();
        assert_eq!(json, "\"false\"");
    }

    #[test]
    fn bool_deserialize() {
        let a: RpcBoolNumber = serde_json::from_str("\"1\"").unwrap();
        let b: RpcBoolNumber = serde_json::from_str("\"0\"").unwrap();
        let c: RpcBoolNumber = serde_json::from_str("\"true\"").unwrap();
        let d: RpcBoolNumber = serde_json::from_str("\"false\"").unwrap();
        assert_eq!(a, true.into());
        assert_eq!(b, false.into());
        assert_eq!(c, true.into());
        assert_eq!(d, false.into());
    }
}
