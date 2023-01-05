use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::path::{self, PathBuf};

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

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultSet {
    timestamp: String,
    resultArray: Vec<SubgraphVault>,
    price: u64,
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
    let u = serde_json::from_reader(reader)?;
    Ok(u)
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
                    break;
                }
                match json_file {
                    Some(x) => {
                        let vault_set = read_vault_set_from_file(x)?;
                        allVaultsAtBlock.insert(block_number_str, vault_set);
                    }
                    None => {}
                }
            }
            Err(e) => {}
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
    read_dir("./data/vaultSet", &mut allVaultsAtBlock);

    let mut blocks_keys: Vec<String> = allVaultsAtBlock
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
                    if (first_block_num < second_block_num) {
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
