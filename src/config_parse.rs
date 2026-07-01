use std::{path::PathBuf, result};

pub struct Config {
    /// File path to the target image being constructed
    pub target_fp: PathBuf,

    /// File path to the atlas texture
    pub atlas_fp: PathBuf,

    /// Random seed
    pub seed: u32,

    /// Any candidates below this threshold% are removed from the evolution cycle
    pub survival_threshold: f32,

    /// The number of evolutionary cycles for each image placed to the canvas
    pub evo_cycles: u32,

    /// The program will run this many candidates when evolving one image
    pub candidates_per_generation: u32,

    /// The total number of images that will be placed onto the canvas overall
    pub total_images: u32,

    /// Controls how strong the mutation effects are, 0.0 means no change and 1.0 means children and maximially different
    pub mutation_strength: f32,
}

impl Config {
    pub fn display(&self) -> String {
        return format!(
            "
Config:
    target: {}
    atlas: {}
    seed: {}
    surival threshold: {}
    evo cycles: {}
    candidates: {}
    total images: {}
    mutation str: {}",
            self.target_fp.display(),
            self.atlas_fp.display(),
            self.seed,
            self.survival_threshold,
            self.evo_cycles,
            self.candidates_per_generation,
            self.total_images,
            self.mutation_strength
        );
    }
}

// Parse program arguments and collect program config into a datastruct
pub fn parse() -> Result<Option<Config>, Box<dyn std::error::Error>> {
    // If -h or -v are passed, we just print value and exit program
    if std::env::args_os().any(|a| a == "-h" || a == "--help") {
        print_help();
        return Ok(None);
    } else if std::env::args_os().any(|a| a == "-v" || a == "--version") {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(None);
    }

    // Default values
    let mut target_file_path = PathBuf::default();
    let mut atlas_file_path = PathBuf::default();
    let mut seed = 0;
    let mut survival_threshold = 0.9;
    let mut evo_cycles = 10;
    let mut candidates_per_gen = 500;
    let mut total_images = 1000;
    let mut mutation_strength = 0.1;

    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        match arg.to_string_lossy().to_lowercase().as_ref() {
            "-t" | "--target" => {
                let t = args.next().ok_or(format!(
                    "{} was used, but no target file path was provided.\nHint: use -h or --help for info",
                    arg.display()
                ))?;
                target_file_path = PathBuf::from(t);
            }
            "-a" | "--atlas" => {
                let a = args.next().ok_or(format!(
                    "{} was used, but no atlas file path was provided.\nHint: use -h or --help for info",
                    arg.display()
                ))?;
                atlas_file_path = PathBuf::from(a);
            }
            "-s" | "--seed" => {
                let s = args.next().ok_or(format!(
                    "{} was used, but no seed value was provided.\nHint: use -h or --help for info",
                    arg.display()
                ))?;
                seed = s.to_string_lossy().into_owned().parse::<u32>()?;
            }
            "-st" | "--survival_threshold" => {
                let st = args.next().ok_or(format!(
                    "{} was used, but no threshold value was provided.\nHint: use -h or --help for info",
                    arg.display()
                ))?;
                survival_threshold = st.to_string_lossy().into_owned().parse::<f32>()?;
            }
            "-ec" | "--evo_cycles" => {
                let ec = args.next().ok_or(format!(
                    "{} was used, but no cycle value was provided.\nHint: use -h or --help for info",
                    arg.display()
                ))?;
                evo_cycles = ec.to_string_lossy().into_owned().parse::<u32>()?;
            }
            "-cpg" | "--candidates" => {
                let cpg = args.next().ok_or(format!(
                    "{} was used, but no candidate count value was provided.\nHint: use -h or --help for info",
                    arg.display()
                ))?;
                candidates_per_gen = cpg.to_string_lossy().into_owned().parse::<u32>()?;
            }
            "-ti" | "--total_images" => {
                let ti = args.next().ok_or(format!(
                    "{} was used, but no total image value was provided.\nHint: use -h or --help for info",
                    arg.display()
                ))?;
                total_images = ti.to_string_lossy().into_owned().parse::<u32>()?;
            }
            "-ms" | "--mutation_strength" => {
                let ms = args.next().ok_or(format!(
                    "{} was used, but no mutation strength value was provided.\nHint: use -h or --help for info",
                    arg.display()
                ))?;
                mutation_strength = ms.to_string_lossy().into_owned().parse::<f32>()?;
            }
            _ => {
                return Err(format!("Unknown parameter: '{}'", arg.display()))?;
            }
        }
    }

    return Ok(Some(Config {
        target_fp: target_file_path,
        atlas_fp: atlas_file_path,
        seed,
        survival_threshold,
        evo_cycles,
        candidates_per_generation: candidates_per_gen,
        total_images,
        mutation_strength,
    }));
}

fn print_help() {
    println!(
        "
    Usage: ./photomosaic (OPTIONS)
    
    Options:
        [-v | --version]: Display the current application version
        [-h | --help]: Display this help text
        
    Examples:
        ./photomosaic -v

    Detailed example of what console usage might look like:
        TODO
    "
    );
}
