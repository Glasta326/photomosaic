use std::path::PathBuf;

use crate::candidate::Candidate;

mod candidate;
mod compute_system;
mod config_parse;
mod data_reader;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // If config parsing returns None, that means an early-exit parameter like -v or --help was used, so we return before doing anything.
    let Some(cfg) = config_parse::parse()? else {
        println!("Exiting...");
        return Ok(());
    };
    println!("{}", cfg.display());

    let (atlas_texture, atlas_entries) = data_reader::read_atlas_data(cfg)?;

    for &e in &atlas_entries {
        println!("{}",e);
    }


    return Ok(());
}
