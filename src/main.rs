mod config_parse;
mod compute_system;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // If config parsing returns None, that means an early-exit parameter like -v or --help was used, so we return before doing anything.
    let Some(cfg) = config_parse::parse()? else {
        println!("Exiting...");
        return Ok(());
    };

    

    
    
    return Ok(());
}
