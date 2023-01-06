#![allow(non_snake_case)]

use serde::de::{Deserializer, Visitor};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
struct VaultLog {
    __typename: String,
    timestamp: String,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Deserialize)]
struct VaultWithLog {
    cdpId: Option<String>,
    logs: Vec<VaultLog>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Vault {
    vaults: Vec<VaultWithLog>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AllVaultsAtBlock {}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(transparent)]
struct StringOrF64(f64);

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
    timestamp: String,
    resultArray: Vec<SubgraphVault>,
    price: StringOrF64,
    rate: String,
    liquidationRatio: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SubgraphVault {
    id: String,
    collateral: String,
    debt: String,
    cdpId: Option<String>,
    updatedAt: Option<String>,
    updatedAtBlock: Option<String>,
    updatedAtTransaction: Option<String>,
    safetyLevel: String,
}

#[derive(Debug, Serialize)]
struct Data<'a> {
    firstBlock: String,
    secondBlock: String,
    vaultsAtFirstBlock: &'a HashMap<String, VaultSet>,
    vaultsAtSecondBlock: &'a HashMap<String, VaultSet>,
}

#[derive(Debug, Serialize)]
struct VaultTransitionInnerType<'a> {
    first: &'a SubgraphVault,
    second: &'a SubgraphVault,
    liquidated: bool,
    liquidationTimestamp: Option<u64>,
}

#[derive(Debug, Serialize)]
struct BlockDiffMetadata {
    firstBlock: String,
    firstTimestamp: String,
    firstPrice: String,
    firstRate: String,
    firstLiquidationRatio: String,

    secondBlock: String,
    secondTimestamp: String,
    secondPrice: String,
    secondRate: String,
    secondLiquidationRatio: String,
}

#[derive(Debug, Serialize)]
struct VaultTransitionWithMetadata<'a> {
    meta: BlockDiffMetadata,
    vaultTransition: HashMap<&'a String, VaultTransitionInnerType<'a>>,
}

fn read_vault_history_from_file<P: AsRef<Path>>(
    path: P,
) -> Result<HashMap<String, Vault>, Box<dyn Error>> {
    // Open the file in read-only mode with buffer.
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // Read the JSON contents of the file as an instance of `User`.
    let u = serde_json::from_reader(reader)?;
    Ok(u)
}

fn read_vault_set_from_file<P: AsRef<Path>>(
    path: P,
) -> Result<HashMap<String, VaultSet>, Box<dyn Error>> {
    // Open the file in read-only mode with buffer.
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // Read the JSON contents of the file as an instance of `User`.
    let u = serde_json::from_reader(reader);
    match u {
        Ok(data) => Ok(data),
        Err(e) => {
            println!("error reading json file: {}", e.to_string());
            return Err(Box::new(e));
        }
    }
}

pub fn read_dir(
    path: &str,
    allVaultsAtBlock: &mut HashMap<String, HashMap<String, VaultSet>>,
) -> Result<(), Box<dyn Error>> {
    let dir = fs::read_dir(path)?;
    for item in dir.into_iter() {
        let item = item?;
        let block_number = item.file_name().into_string();
        let block_number_path = item.path();
        println!(
            "block_number: {}, block_number_path: {}",
            item.file_name().into_string().unwrap(),
            item.path().to_str().unwrap(),
        );
        match block_number {
            Ok(block_number_str) => {
                let inner_dir = fs::read_dir(block_number_path)?;
                let mut json_file: Option<PathBuf> = None;
                for inner_item in inner_dir.into_iter() {
                    json_file = Some(inner_item?.path());
                }
                match json_file {
                    Some(x) => {
                        println!("json_file: {}", x.to_str().unwrap());
                        let vault_set = read_vault_set_from_file(x)?;
                        allVaultsAtBlock.insert(block_number_str, vault_set);
                    }
                    None => {}
                }
            }
            Err(e) => {
                println!("error: {}", e.to_str().unwrap())
            }
        }
    }
    Ok(())
}

fn main() {
    let vaults =
        read_vault_history_from_file("../subgraph-tools/data/jsons/vaultHistory.json").unwrap();
    let vault_ids = vaults.keys();
    let mut liquidationTimestampListByVault: HashMap<String, Vec<u64>> = HashMap::new();
    for vault_id in vault_ids.into_iter() {
        let timestamp: Vec<u64> = vaults[vault_id].vaults[0]
            .logs
            .iter()
            .filter(|vaultLog| vaultLog.__typename == "liquidationStartLog")
            .filter_map(|vaultLog| vaultLog.timestamp.parse::<u64>().ok())
            .collect();
        liquidationTimestampListByVault.insert(vault_id.clone(), timestamp);
    }

    let mut allVaultsAtBlock: HashMap<String, HashMap<String, VaultSet>> = HashMap::new();
    // list up directories
    let read_dir_result = read_dir("../subgraph-tools/data/vaultSet", &mut allVaultsAtBlock);
    // now allVaultsAtBlock contains data
    if read_dir_result.is_ok() {
        let blocks_keys: Vec<String> = allVaultsAtBlock
            .keys()
            .into_iter()
            .map(|v| v.to_string())
            .collect();
        // let mut blocks_keys_2 = blocks_keys;
        let blocks_count = blocks_keys.len();
        println!(
            "blocks_count: {}, blocks_keys has 16266198: {}",
            blocks_count,
            blocks_keys.contains(&("16266198").to_string()),
        );

        let mut dataset: Vec<Data> = vec![];
        for block_key_1_index in 0..blocks_count {
            let block_key_1 = &blocks_keys[block_key_1_index];
            let vaultsAtFirstBlock = &allVaultsAtBlock[block_key_1];
            for block_key_2_index in 0..blocks_count {
                let block_key_2 = &blocks_keys[block_key_2_index];
                let first_block = block_key_1.parse::<u64>();
                let second_block = block_key_2.parse::<u64>();
                match (first_block, second_block) {
                    (Ok(first_block_num), Ok(second_block_num)) => {
                        if first_block_num < second_block_num {
                            let vaultsAtSecondBlock = &allVaultsAtBlock[block_key_2];
                            let data: Data = Data {
                                firstBlock: block_key_1.to_string(),
                                secondBlock: block_key_2.to_string(),
                                vaultsAtFirstBlock: vaultsAtFirstBlock,
                                vaultsAtSecondBlock: vaultsAtSecondBlock,
                            };
                            dataset.push(data);
                        }
                    }
                    _ => {}
                }
            }
        }

        let mut vaultTransitionSet: Vec<VaultTransitionWithMetadata> = vec![];
        let splitFileCount = 100;
        for index in 0..dataset.len() {
            let row = &dataset[index];

            let firstBlock = &row.firstBlock;
            let firstTimestamp = &row.vaultsAtFirstBlock["ETH-A"].timestamp;
            let firstVaults = &row.vaultsAtFirstBlock["ETH-A"].resultArray;
            let firstPrice = &row.vaultsAtFirstBlock["ETH-A"].price;
            let firstRate = &row.vaultsAtFirstBlock["ETH-A"].rate;
            let firstLiquidationRatio = &row.vaultsAtFirstBlock["ETH-A"].liquidationRatio;

            let secondBlock = &row.secondBlock;
            let secondTimestamp = &row.vaultsAtSecondBlock["ETH-A"].timestamp;
            let secondVaults = &row.vaultsAtSecondBlock["ETH-A"].resultArray;
            let secondPrice = &row.vaultsAtSecondBlock["ETH-A"].price;
            let secondRate = &row.vaultsAtSecondBlock["ETH-A"].rate;
            let secondLiquidationRatio = &row.vaultsAtSecondBlock["ETH-A"].liquidationRatio;

            let blockDiffMetadata: BlockDiffMetadata = BlockDiffMetadata {
                firstBlock: firstBlock.to_string(),
                firstPrice: firstPrice.0.to_string(),
                firstRate: firstRate.to_string(),
                firstLiquidationRatio: firstLiquidationRatio.to_string(),
                firstTimestamp: firstTimestamp.to_string(),
                secondBlock: secondBlock.to_string(),
                secondPrice: secondPrice.0.to_string(),
                secondRate: secondRate.to_string(),
                secondLiquidationRatio: secondLiquidationRatio.to_string(),
                secondTimestamp: secondTimestamp.to_string(),
            };

            let mut vaultTransition: HashMap<&String, VaultTransitionInnerType> = HashMap::new();

            let mut secondvaultsById: HashMap<&String, &SubgraphVault> = HashMap::new();
            for i in 0..secondVaults.len() {
                secondvaultsById.insert(&secondVaults[i].id, &secondVaults[i]);
            }

            for j in 0..firstVaults.len() {
                let vault = &firstVaults[j];
                let collateral = vault.collateral.parse::<f64>();
                let debt = vault.collateral.parse::<f64>();
                match (collateral, debt) {
                    (Ok(collateral), Ok(debt)) => {
                        if collateral > 0.0 && debt > 0.0 {
                            let liquidationTimestampList =
                                &liquidationTimestampListByVault[&vault.id];
                            let liquidationTimestampAny =
                                liquidationTimestampList
                                    .iter()
                                    .find(|liquidationTimestamp| {
                                        let firstTimestampU64 = firstTimestamp.parse::<u64>();
                                        let secondTimestampU64 = firstTimestamp.parse::<u64>();
                                        return match (firstTimestampU64, secondTimestampU64) {
                                            (Ok(firstTimestampU64), Ok(secondTimestampU64)) => {
                                                return &&firstTimestampU64 < liquidationTimestamp
                                                    && liquidationTimestamp < &&secondTimestampU64;
                                            }
                                            _ => false,
                                        };
                                    });
                            vaultTransition.insert(
                                &vault.id,
                                match liquidationTimestampAny {
                                    Some(liquidationTimestamp) => VaultTransitionInnerType {
                                        first: vault,
                                        second: secondvaultsById[&vault.id],
                                        liquidated: true,
                                        liquidationTimestamp: Some(*liquidationTimestamp),
                                    },
                                    None => VaultTransitionInnerType {
                                        first: vault,
                                        second: secondvaultsById[&vault.id],
                                        liquidated: false,
                                        liquidationTimestamp: None,
                                    },
                                },
                            );
                        }
                    }
                    _ => {}
                }
            }

            let vaultTransitionWithMetadata: VaultTransitionWithMetadata =
                VaultTransitionWithMetadata {
                    meta: blockDiffMetadata,
                    vaultTransition: vaultTransition,
                };
            vaultTransitionSet.push(vaultTransitionWithMetadata);

            if index % splitFileCount == splitFileCount - 1 {
                println!(
                    "`save split content in file: {} ... {}",
                    index - (splitFileCount - 1),
                    index
                );
                let file_name = format!(
                    "../subgraph-tools/data/result/result-{}-{}.json",
                    index - (splitFileCount - 1),
                    index
                );

                let json_str = serde_json::to_string(&vaultTransitionSet).unwrap();
                fs::write(file_name, json_str).expect("Unable to write file");
                vaultTransitionSet = vec![];
            }
        }

        // check if dataSet can be considered valid using liquidationTimestampListByVault
        // vaultTransitionSet array contents can be occasionally saved to file.

        println!(
            "{:#?}",
            vaults["0x0000485d124ca18832ebc0e0e3d1947ee4db8427-ETH-A"].vaults[0].cdpId
        );
        println!(
            "{:#?}",
            allVaultsAtBlock["16266198"]["ETH-A"].resultArray[0].cdpId
        );
    }
}
