// use json_structure;
use std::error::Error;
use std::fs;

fn read_dir() -> Result<(), Box<dyn Error>> {
    let result_dir = "../subgraph-tools/data/result/";
    let dir = fs::read_dir(result_dir)?;
    for item in dir.into_iter() {
        let file_name = item?.path();
        // read_file(file_name)?;
    }
    Ok(())
}

fn main() {
    read_dir();
}
