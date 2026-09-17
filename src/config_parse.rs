use std::path::PathBuf;

use crate::profiling::Dropwatch;

/// Collection of all program configuration settings & useful data based on program configuration that is not explicitly specified
/// For example, the dimensions of the target and canvas images, while not specified as program args,
/// are still inlcuded here as they are based on the images the user provided
pub struct Config {
    /// File path to the target image being constructed
    pub target_texture: PathBuf,

    /// File path to the atlas texture
    pub atlas_texture_fp: PathBuf,

    /// File path to the atlas json. If not provided, will default to atlas_texture_fp, but with .json extension
    pub atlas_json_fp: PathBuf,

    /// Folder path to the folder used for performance profiling logs. If not provided, will default to the same folder as the program is run in
    pub profile_log_fp: PathBuf,

    /// The target image will be internally downscaled to reach this pixel target
    /// For example, a 1920x1080 image has ~2,000,000 pixels, so if the pixel_target is 10k, it will be downscaled by ~14x
    /// ~14x is because the total pixel count scales n², so to reduce by a factor of 200, you need to downscale by sqrt(200) ≈ 14
    pub pixel_target: u32,

    /// Random seed
    pub seed: u64,

    /// Any candidates below this threshold are removed from the evolution cycle
    pub survival_threshold: usize,

    /// The number of evolutionary cycles for each image placed to the canvas
    pub evo_cycles: usize,

    /// The program will simulate and keep track of this many candidates when evolving one image
    pub candidates_per_generation: usize,

    /// The total number of images that will be placed onto the canvas overall
    pub total_images: usize,

    /// Controls how strong the mutation effects are, 0.0 means no change and 1.0 means children and maximially different
    pub mutation_strength: f32,

    /// Toggles the ability for images to have hue shift mutations
    pub enable_hue: bool,

    /// Toggles the ability for the program to save a video showing the image generation process
    pub enable_video: bool,

    /// Extra program data not extrapolated from data specified by the user
    pub extra_data: ConfigData,
}
pub struct ConfigData {
    pub atlas_dimensions: (u32, u32),
    pub target_dimensions: (u32, u32),
    /// The target image will be internally downscaled by this amount to match the pixel target
    pub downscale_factor: f32,
    pub child_count: usize,
    pub atlas_entry_count: u32,
}
impl std::fmt::Display for ConfigData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "
atlas dimensions:  {:?}
target dimensions: {:?}
downscale factor:  {}
child count:       {}
atlas_entry_count: {}",
            self.atlas_dimensions,
            self.target_dimensions,
            self.downscale_factor,
            self.child_count,
            self.atlas_entry_count
        )?;
        return Ok(());
    }
}

impl ConfigData {
    pub fn init(candidate_count: usize, threshold: usize) -> Self {
        return ConfigData {
            atlas_dimensions: (0, 0),
            target_dimensions: (0, 0),
            downscale_factor: -1.0,
            child_count: (candidate_count as f32 / threshold as f32).round() as usize - 1, // -1 to account for the parent staying alive
            atlas_entry_count: 0,
        };
    }
}

impl Config {
    pub fn display(&self) -> String {
        return format!(
            "
target:             {}
atlas texture:      {}
atlas json:         {}
profile log folder: {}
pixel target:       {}
seed:               {}
survival threshold: {}
evolution cycles:   {}
candidates:         {}
total images:       {}
mutation strength:  {}
hue shifting:       {}
video generation:   {}",
            self.target_texture.display(),
            self.atlas_texture_fp.display(),
            self.atlas_json_fp.display(),
            self.profile_log_fp.display(),
            self.pixel_target,
            self.seed,
            self.survival_threshold,
            self.evo_cycles,
            self.candidates_per_generation,
            self.total_images,
            self.mutation_strength,
            self.enable_hue,
            self.enable_video
        );
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            target_texture: PathBuf::from("debug/testing_input/target.png"),
            atlas_texture_fp: PathBuf::from("debug/testing_input/atlas.png"),
            atlas_json_fp: PathBuf::from("debug/testing_input/atlas.json"),
            profile_log_fp: std::env::current_dir()
                .expect("The current working directory could not be opened."),
            pixel_target: 20000,
            seed: rand::random::<u64>(), // Default is random. Set-seeds would cause the same image each time
            survival_threshold: 5,
            evo_cycles: 10,
            candidates_per_generation: 500,
            total_images: 1000,
            mutation_strength: 0.1,
            extra_data: ConfigData::init(500, 5),
            enable_hue: false,
            enable_video: false,
        }
    }
}

// Parse program arguments and collect program config into a datastruct
// NOTE: in the future, we will have a proper dialogue box with a settings window, so there will always be input for every single value
// As of right now, it just errors if you dont provide a target file or atlas texture file, but this will get resolved with the dialogue box
pub fn parse() -> Result<Option<Config>, Box<dyn std::error::Error>> {
    // If -h or -v are passed, we just print value and exit program
    if std::env::args_os().any(|a| a == "-h" || a == "--help") {
        print_help();
        return Ok(None);
    } else if std::env::args_os().any(|a| a == "-v" || a == "--version") {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(None);
    }

    let _d = Dropwatch::new("Config parsing");

    // Default values
    let default = Config::default();
    let mut target_file_path = default.target_texture;
    let mut atlas_file_path = default.atlas_texture_fp;
    let mut atlas_json_path = default.atlas_json_fp;
    let mut profile_log_fp = default.profile_log_fp;
    let mut pixel_target = default.pixel_target;
    let mut seed = default.seed;
    let mut survival_threshold = default.survival_threshold;
    let mut evo_cycles: usize = default.evo_cycles;
    let mut candidates_per_gen: usize = default.candidates_per_generation;
    let mut total_images: usize = default.total_images;
    let mut mutation_strength = default.mutation_strength;
    let mut hue_shift = default.enable_hue;
    let mut video_gen = default.enable_video;

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
            "-aj" | "--atlas_json" => {
                let aj = args.next().ok_or(format!(
                    "{} was used, but no atlas json file path was provided.\nHint: use -h or --help for info",
                    arg.display()
                ))?;
                atlas_json_path = PathBuf::from(aj);
            }
            "-pl" | "--profile_log" => {
                let pl = args.next().ok_or(format!(
                    "{} was used, but no profile log file path was provided.\nHint: use -h or --help for info",
                    arg.display()
                ))?;
                profile_log_fp = PathBuf::from(pl);
            }
            "-pt" | "--pixel_target" => {
                let ds = args.next().ok_or(format!(
                    "{} was used, but no target value was provided.\nHint: use -h or --help for info",
                    arg.display()
                ))?;
                pixel_target = ds.to_string_lossy().into_owned().parse::<u32>()?;
            }
            "-s" | "--seed" => {
                let s = args.next().ok_or(format!(
                    "{} was used, but no seed value was provided.\nHint: use -h or --help for info",
                    arg.display()
                ))?;
                seed = s.to_string_lossy().into_owned().parse::<u64>()?;
            }
            "-st" | "--survival_threshold" => {
                let st = args.next().ok_or(format!(
                    "{} was used, but no threshold value was provided.\nHint: use -h or --help for info",
                    arg.display()
                ))?;
                survival_threshold = st.to_string_lossy().into_owned().parse::<usize>()?;
            }
            "-ec" | "--evolution_cycles" => {
                let ec = args.next().ok_or(format!(
                    "{} was used, but no cycle value was provided.\nHint: use -h or --help for info",
                    arg.display()
                ))?;
                evo_cycles = ec.to_string_lossy().into_owned().parse::<usize>()?;
            }
            "-cpg" | "--candidates" => {
                let cpg = args.next().ok_or(format!(
                    "{} was used, but no candidate count value was provided.\nHint: use -h or --help for info",
                    arg.display()
                ))?;
                candidates_per_gen = cpg.to_string_lossy().into_owned().parse::<usize>()?;
            }
            "-ti" | "--total_images" => {
                let ti = args.next().ok_or(format!(
                    "{} was used, but no total image value was provided.\nHint: use -h or --help for info",
                    arg.display()
                ))?;
                total_images = ti.to_string_lossy().into_owned().parse::<usize>()?;
            }
            "-ms" | "--mutation_strength" => {
                let ms = args.next().ok_or(format!(
                    "{} was used, but no mutation strength value was provided.\nHint: use -h or --help for info",
                    arg.display()
                ))?;
                mutation_strength = ms.to_string_lossy().into_owned().parse::<f32>()?;
            }
            "-h" | "--hue_shift" => {
                hue_shift = true;
            }
            "-vg" | "--video_gen" => {
                video_gen = true;
            }
            _ => {
                return Err(format!("Unknown parameter: '{}'", arg.display()))?;
            }
        }
    }

    // If the atlas json path is still default, then use the same path as the atlas texture
    if atlas_json_path == PathBuf::default() {
        atlas_json_path = atlas_file_path.clone();
        atlas_json_path.set_extension("json");
    }

    // Calculate the required downscale effect for the image

    safety_checks(
        &pixel_target,
        &mut survival_threshold,
        &mut candidates_per_gen,
        &mut mutation_strength,
    )?;

    // Calculate extra miscelaneous data from user specified configuration
    // TODO: this is bad
    // the data in here is half calculated on init and half done later and thats bad coding
    let extra_data = ConfigData::init(candidates_per_gen, survival_threshold);

    return Ok(Some(Config {
        target_texture: target_file_path,
        atlas_texture_fp: atlas_file_path,
        atlas_json_fp: atlas_json_path,
        profile_log_fp,
        pixel_target,
        seed,
        survival_threshold,
        evo_cycles,
        candidates_per_generation: candidates_per_gen,
        total_images,
        mutation_strength,
        enable_hue: hue_shift,
        extra_data: extra_data,
        enable_video: video_gen,
    }));
}

//mmm i love dereferencing
fn safety_checks(
    pixel_target: &u32,
    survival_threshold: &mut usize,
    candidates_per_gen: &mut usize,
    mutation_strength: &mut f32,
) -> Result<(), Box<dyn std::error::Error>> {
    if *pixel_target <= 0 {
        return Err(format!("pixel_target is {}, which is not allowed!", *pixel_target).into());
    }

    // Ensure candidate count is > survival threshold
    if *candidates_per_gen < *survival_threshold {
        println!(
            "Automatically adjusted candidate count from {} to {} as candidate count must be greater than surivial threshold!",
            *candidates_per_gen, *survival_threshold
        );
        *candidates_per_gen = *survival_threshold;
    }

    // Ensure candidate count is a multiple of survival threshold
    let offset = *candidates_per_gen % *survival_threshold;
    if offset != 0 {
        println!(
            "Automatically adjusted candidate count from {} to {} to ensure divisiblity!",
            *candidates_per_gen,
            *candidates_per_gen + (*survival_threshold - offset)
        );
        *candidates_per_gen += *survival_threshold - offset;
    }

    if *mutation_strength < 0.0 || *mutation_strength > 1.0 {
        println!(
            "Automatically clamped mutation strength to {} as it was {}, which is outside the bounds of [0.0, 1.0]",
            mutation_strength.clamp(0.0, 1.0),
            *mutation_strength
        );
        *mutation_strength = mutation_strength.clamp(0.0, 1.0);
    }

    return Ok(());
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
