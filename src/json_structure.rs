#![allow(non_snake_case)]

use serde::de::{Deserializer, Visitor};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultLog {
    pub __typename: String,
    pub timestamp: String,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Deserialize)]
pub struct VaultWithLog {
    pub cdpId: Option<String>,
    pub logs: Vec<VaultLog>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Vault {
    pub vaults: Vec<VaultWithLog>,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(transparent)]
pub struct StringOrF64(pub f64);

impl<'de> Deserialize<'de> for StringOrF64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct MyVisitor;

        impl<'de> Visitor<'de> for MyVisitor {
            type Value = StringOrF64;

            fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt.write_str("f64 or string")
            }

            fn visit_f64<E>(self, val: f64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(StringOrF64(val as f64))
            }

            fn visit_u64<E>(self, val: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(StringOrF64(val as f64))
            }

            fn visit_str<E>(self, val: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match val.parse::<f64>() {
                    Ok(val) => self.visit_f64(val),
                    Err(_) => Err(E::custom("failed to parse f64")),
                }
            }
        }

        deserializer.deserialize_any(MyVisitor)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultSet {
    pub timestamp: String,
    pub resultArray: Vec<SubgraphVault>,
    pub price: StringOrF64,
    pub rate: String,
    pub liquidationRatio: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubgraphVault {
    pub id: String,
    pub collateral: String,
    pub debt: String,
    pub cdpId: Option<String>,
    pub updatedAt: Option<String>,
    pub updatedAtBlock: Option<String>,
    pub updatedAtTransaction: Option<String>,
    pub safetyLevel: String,
}

#[derive(Debug, Serialize)]
pub struct Data<'a> {
    pub firstBlock: String,
    pub secondBlock: String,
    pub vaultsAtFirstBlock: &'a HashMap<String, VaultSet>,
    pub vaultsAtSecondBlock: &'a HashMap<String, VaultSet>,
}

#[derive(Debug, Serialize)]
pub struct VaultTransitionInnerType<'a> {
    pub first: &'a SubgraphVault,
    pub second: &'a SubgraphVault,
    pub liquidated: bool,
    pub liquidationTimestamp: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct BlockDiffMetadata {
    pub firstBlock: String,
    pub firstTimestamp: String,
    pub firstPrice: String,
    pub firstRate: String,
    pub firstLiquidationRatio: String,

    pub secondBlock: String,
    pub secondTimestamp: String,
    pub secondPrice: String,
    pub secondRate: String,
    pub secondLiquidationRatio: String,
}

#[derive(Debug, Serialize)]
pub struct VaultTransitionWithMetadata<'a> {
    pub meta: BlockDiffMetadata,
    pub vaultTransition: HashMap<&'a String, VaultTransitionInnerType<'a>>,
}
