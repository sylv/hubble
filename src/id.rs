use std::fmt::Debug;

use async_graphql::*;
use serde::{Deserialize, Serialize};

/// Represents an IMDb ID
#[derive(Clone, Copy, PartialEq, Eq, Hash, sqlx::Type)]
#[sqlx(transparent)]
pub struct Id(u32);

impl Id {
    pub fn get(&self) -> u32 {
        self.0
    }

    pub fn to_string(&self) -> String {
        format!("tt{:07}", self.0)
    }
}

#[Scalar]
impl ScalarType for Id {
    fn parse(value: Value) -> InputValueResult<Self> {
        if let Value::String(value) = &value {
            if value.starts_with("tt") {
                let value = value[2..].parse().map_err(|_| {
                    InputValueError::custom(format!("Invalid IMDb ID \"{}\"", value))
                })?;

                Ok(Id(value))
            } else {
                Err(InputValueError::custom(format!(
                    "Invalid IMDb ID \"{}\"",
                    value
                )))
            }
        } else {
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.to_string())
    }
}

impl Serialize for Id {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // IMDb IDs are prefixed with "tt" then the number
        // the number is a minimum of 7 digits, zero-padded
        serializer.serialize_str(self.to_string().as_str())
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D>(deserializer: D) -> Result<Id, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let s = s[2..].parse().map_err(serde::de::Error::custom)?;
        Ok(Id(s))
    }
}

impl Debug for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Id({})", self.to_string())
    }
}

impl From<u64> for Id {
    fn from(id: u64) -> Self {
        Id(id as u32)
    }
}

impl From<i64> for Id {
    fn from(id: i64) -> Self {
        Id(id as u32)
    }
}
