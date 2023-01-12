#![allow(non_snake_case)]

use rust_subgraph_tools::json_structure::{
    BlockDiffMetadata, Data, SubgraphVault, Vault, VaultSet, VaultTransitionInnerType,
    VaultTransitionWithMetadata,
};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;

fn read_vault_history_from_file<P: AsRef<Path>>(
    path: P,
) -> Result<HashMap<String, Vault>, Box<dyn Error>> {
    let start = Instant::now();
    // Open the file in read-only mode with buffer.
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // Read the JSON contents of the file as an instance of `User`.
    let u = serde_json::from_reader(reader)?;
    println!(
        "Time elapsed in read_vault_history_from_file() is: {:?}",
        start.elapsed()
    );
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
    let start = Instant::now();
    let dir = fs::read_dir(path)?;
    // take 10 is for debug
    for item in dir.into_iter()
    // .take(10)
    {
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
    println!("Time elapsed in read_dir() is: {:?}", start.elapsed());
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

        let start = Instant::now();
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
                        if first_block_num < second_block_num
                            && second_block_num - first_block_num < 40000
                        // 40000 blocks = around one week
                        {
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
        println!(
            "Time elapsed in preparing dataset is: {:?}",
            start.elapsed()
        );

        println!("dataset length: {}", dataset.len());

        let start = Instant::now();
        let mut dRatio: f64 = 0.0;
        let mut validDataPointCount: u32 = 0;
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
                                        let secondTimestampU64 = secondTimestamp.parse::<u64>();
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

            // calculate data from vaultTransitionSet
            let secondPrice = vaultTransitionWithMetadata.meta.secondPrice.parse::<f64>();
            let firstPrice = vaultTransitionWithMetadata.meta.firstPrice.parse::<f64>();
            match (firstPrice, secondPrice) {
                (Ok(firstPrice), Ok(secondPrice)) => {
                    let price_drop_ratio = secondPrice / firstPrice;
                    if price_drop_ratio < 1.0 {
                        // calculated capital at risk value
                        let capitalAtRiskValueRisk = vaultTransitionWithMetadata
                            .vaultTransition
                            .values()
                            .into_iter()
                            .map(|vaultTransitionInner| {
                                let first = vaultTransitionInner.first;
                                match (
                                    first.collateral.parse::<f64>(),
                                    first.debt.parse::<f64>(),
                                    vaultTransitionWithMetadata
                                        .meta
                                        .firstLiquidationRatio
                                        .parse::<f64>(),
                                    vaultTransitionWithMetadata.meta.firstRate.parse::<f64>(),
                                ) {
                                    (Ok(collateral), Ok(debt), Ok(liquidationRatio), Ok(rate)) => {
                                        if collateral * secondPrice > debt * liquidationRatio * rate
                                        {
                                            return 0.0;
                                        } else {
                                            return debt;
                                        }
                                    }
                                    _ => {
                                        return 0.0;
                                    }
                                }
                            })
                            .fold(0.0, |x, y| x + y);

                        // actual capital at risk value
                        let capitalAtRiskValueLiq = vaultTransitionWithMetadata
                            .vaultTransition
                            .values()
                            .into_iter()
                            .map(|vaultTransitionInner| {
                                let liquidatedAmount = if vaultTransitionInner.liquidated {
                                    match (&vaultTransitionInner.first.debt).parse::<f64>() {
                                        Ok(debt) => debt,
                                        _ => 0.0,
                                    }
                                } else {
                                    0.0
                                };

                                return liquidatedAmount;
                            })
                            .fold(0.0, |x, y| x + y);

                        // if any vault is liquidated?
                        let vaultTransitionIncludesLiquidatedVault = vaultTransitionWithMetadata
                            .vaultTransition
                            .values()
                            .into_iter()
                            .any(|vaultTransitionInner| return vaultTransitionInner.liquidated);

                        let vaultTransitionSetCount =
                            vaultTransitionWithMetadata.vaultTransition.len();

                        if vaultTransitionIncludesLiquidatedVault {
                            println!("liquidated: true. index: {}, firstBlock: {}, secondBlock: {}, vaults: {}", vaultTransitionIncludesLiquidatedVault, firstBlock, secondBlock, vaultTransitionSetCount)
                        }

                        // only think in case estimated risk is above zero. otherwise, the data point is invalid.
                        if capitalAtRiskValueRisk > 0.0 {
                            let maybeNan = (capitalAtRiskValueLiq - capitalAtRiskValueRisk).abs()
                                / capitalAtRiskValueRisk;
                            if maybeNan.is_nan() {
                                println!(
                                    "nan detected: {}, {}, {}, {}",
                                    // serde_json::to_string(vaultTransition).unwrap()
                                    vaultTransitionWithMetadata.meta.firstLiquidationRatio,
                                    vaultTransitionWithMetadata.meta.firstRate,
                                    capitalAtRiskValueLiq,
                                    capitalAtRiskValueRisk,
                                );
                            } else {
                                dRatio += maybeNan;
                                validDataPointCount += 1;
                                println!("price_drop_ratio: {}, index: {}, capitalAtRiskValueRisk: {}, capitalAtRiskValueLiq: {}, firstBlock: {}, secondBlock: {}, liq'd?: {}, vaults: {}",
                                price_drop_ratio, index, capitalAtRiskValueRisk, capitalAtRiskValueLiq, firstBlock, secondBlock, vaultTransitionIncludesLiquidatedVault, vaultTransitionSetCount)
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        let dRatioMean = dRatio / (validDataPointCount as f64) * 100.0;
        println!(
            "dRatio: {}, validDataPointCount: {}, d ratio mean: {}",
            dRatio, validDataPointCount, dRatioMean
        );
        println!(
            "Time elapsed in calculating dRatio is: {:?}",
            start.elapsed()
        );

        // check if dataSet can be considered valid using liquidationTimestampListByVault
        // vaultTransitionSet array contents can be occasionally saved to file.

        println!(
            "{:#?}",
            vaults["0x0000485d124ca18832ebc0e0e3d1947ee4db8427-ETH-A"].vaults[0].cdpId
        );
    }
}
