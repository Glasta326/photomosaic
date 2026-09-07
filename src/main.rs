use std::time::Duration;

use rand::{RngExt, SeedableRng, rngs::StdRng};
use wgpu::naga::back::spv::SourceLanguage::Rust;

use crate::{
    candidate::Candidate,
    config_parse::Config,
    gpu::GpuContext,
    profiling::{Dropwatch, Metric, RuntimeStats, Stopwatch},
    utils::buffer_utils,
};

mod gpu;
mod profiling;
mod utils;

mod candidate;
mod config_parse;
mod data_reader;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Performance logging
    let mut performance = RuntimeStats::init()?;
    
    // Create all the stopwatches in one place
    let mut initialization_sw = Stopwatch::new();
    let mut candidate_populating_sw = Stopwatch::new();
    let mut score_index_sorting_sw = Stopwatch::new();

    initialization_sw.start(None);

    // If config parsing returns None, that means an early-exit parameter like -v or --help was used, so we return before doing anything.
    let Some(mut cfg) = config_parse::parse()? else {
        println!("Exiting...");
        return Ok(());
    };
    println!("\n[Config]:{}\n", cfg.display());

    // Create the rng from the config seed
    let mut rng = StdRng::seed_from_u64(cfg.seed);

    let (atlas_texture, atlas_entries) = data_reader::read_atlas_data(&cfg)?;
    println!(
        "Loaded {} by {} texture atlas with {} entries",
        atlas_texture.width(),
        atlas_texture.height(),
        atlas_entries.len()
    );

    let target_texture = data_reader::read_target_texture(&cfg)?;
    println!(
        "Loaded {} by {} target texture: [{}]",
        target_texture.width(),
        target_texture.height(),
        cfg.target_texture.file_name().unwrap().display()
    );

    // Update cfg data
    cfg.extra_data.atlas_dimensions = atlas_texture.dimensions();
    cfg.extra_data.target_dimensions = target_texture.dimensions();
    cfg.extra_data.atlas_entry_count = atlas_entries.len() as u32;

    // Initalise gpu resources
    let context = gpu::GpuContext::init(&cfg, atlas_texture, atlas_entries, target_texture)?;
    let score_shader = gpu::ScoreShader::init(&cfg, &context)?;
    let draw_shader = gpu::DrawShader::init(&cfg, &context)?;

    // Initalise other resources
    let mut candidates: Vec<Candidate> =
        vec![Candidate::new(0, 0.0, 0.0, 0.0, 0.0); cfg.candidates_per_generation];
    let mut candidate_scores: Vec<f32> = Vec::with_capacity(cfg.candidates_per_generation);

    performance.record(Metric::Initialization, initialization_sw.elapse());

    // Padding for the live display
    print!("\n\n\n");

    // Main loop - every cycle of this adds one image to the final output
    for i in 1..=cfg.total_images {
        candidate_populating_sw.start(None);

        // Initalise the candidate array with a bunch of random ones
        candidates.fill_with(|| Candidate::random(&cfg, &mut rng));
        performance.record(
            Metric::CandidatePopulation,
            candidate_populating_sw.elapse(),
        );

        // Evolve a new image to draw to the canvas
        for e in 1..=cfg.evo_cycles {
            // Get the array of scores per each candidate
            candidate_scores = score_shader.run(&mut performance, &cfg, &context, &candidates)?;

            score_index_sorting_sw.start(None);

            // To avoid moving the array out of scope, we sort an array of indicies based on the score values
            let mut indicies: Vec<usize> = (0..candidates.len()).collect();
            indicies.sort_by(|&a, &b| {
                candidate_scores[a]
                    .partial_cmp(&candidate_scores[b])
                    .unwrap()
            });
            performance.record(Metric::ScoreIndexSorting, score_index_sorting_sw.elapse());

            if e < cfg.evo_cycles {
                // We go down the list of candidates in desceding order, untill we hit our limit determined by the survival threshold
                // Each of these candidates is allowed to create children, and then both the candidate and the child is moved into the new array
                let mut new_candidates: Vec<Candidate> =
                    Vec::with_capacity(cfg.candidates_per_generation);

                for j in 0..cfg.survival_threshold {
                    let index = indicies[j];
                    let candidate = candidates[index];
                    // First add all the children
                    for _k in 0..cfg.extra_data.child_count {
                        new_candidates.push(candidate.mutate_new(&cfg, &mut rng));
                    }
                    // Then add the parent
                    new_candidates.push(candidate);
                }
                // Override the candidate pool with our new candidates
                candidates = new_candidates;
            } else {
                // This is the final iteration, so we get the best candidate and put it to the top
                candidates[0] = candidates[indicies[0]];
                candidate_scores[0] = candidate_scores[indicies[0]];
            }
        }

        // Draw the winning candidate onto the internal canvas
        let mut winner = candidates[0];
        draw_shader.run_small(
            &mut performance,
            &context,
            &winner,
            &context.buffers.input_canvas_texture,
        )?;

        // Scale up the winner's position and scale for the output canvas
        winner.scale *= cfg.downscale_factor;
        winner.pos_x *= cfg.downscale_factor;
        winner.pos_y *= cfg.downscale_factor;
        draw_shader.run_large(
            &mut performance,
            &context,
            &winner,
            &context.buffers.output_canvas_texture,
        )?;

        // Output progress info
        let scale_factor = cfg.total_images as f32 / 100.0;
        let filled = i as f32 / 5.0 / scale_factor;
        let empty = (cfg.total_images as f32 / 5.0 / scale_factor) - filled;

        print!("\x1b[2A");
        print!(
            "\r\x1b[2KProgress: [{}{}] {}/{}\n",
            "=".repeat(filled.round() as usize), " ".repeat(empty.round() as usize), i, cfg.total_images
        );
        print!("\r\x1b[2KBest score: {}\n", candidate_scores[0]);

        std::io::Write::flush(&mut std::io::stdout()).unwrap();
    }
    println!("");

    // Save images and logs at the end
    save_output(&cfg, &context)?;
    performance.save_results(&cfg)?;

    println!("Done!");
    // println!("\nPerformance summary:");
    // performance.display_summary();
    return Ok(());
}

// TODO: Dynamic save location based on config
fn save_output(cfg: &Config, context: &GpuContext) -> Result<(), Box<dyn std::error::Error>> {
    let img =
        buffer_utils::texture_to_image(&cfg, &context, &context.buffers.input_canvas_texture)?;
    img.save("debug/testing_output/test_output_internal.png")?;

    let img =
        buffer_utils::texture_to_image(&cfg, &context, &context.buffers.output_canvas_texture)?;
    img.save("debug/testing_output/test_output.png")?;

    return Ok(());
}
