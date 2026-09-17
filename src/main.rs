use std::{
    f32, fs,
    io::{self, Write},
    process::{Command, Stdio},
    time::Duration,
};

use rand::{SeedableRng, rngs::StdRng};
use wgpu::naga::back::spv::SourceLanguage::Rust;

use crate::{
    candidate::Candidate,
    config_parse::Config,
    datastructs::ShiftRegister,
    gpu::GpuContext,
    profiling::{Dropwatch, Metric, RuntimeStats, Stopwatch},
    utils::{buffer_utils, math_utils},
    video_writer::VideoWriter,
};

mod datastructs;
mod gpu;
mod profiling;
mod utils;

mod candidate;
mod config_parse;
mod data_reader;
mod video_writer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Performance logging
    let mut performance = RuntimeStats::init()?;

    // Create all the stopwatches in one place
    let mut initialization_sw = Stopwatch::new();
    let mut candidate_populating_sw = Stopwatch::new();
    let mut score_index_sorting_sw = Stopwatch::new();
    let mut video_generation_sw = Stopwatch::new();

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

    let target_texture = data_reader::read_target_texture(&mut cfg)?;
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
    let candidate_gen_shader = gpu::CandidateGenShader::init(&cfg, &context)?;

    // Initalise other resources

    // Create the video writer, if enabled
    let mut video: Option<VideoWriter> = None;
    if cfg.enable_hue {
        video = Some(VideoWriter::new(&cfg, &context)?);
        if !VideoWriter::confirm_ffmpeg() {
            return Err("ffmpeg could not be found".into());
        }
    }

    let mut candidates: Vec<Candidate> =
        vec![Candidate::new(0, 0.0, 0.0, 0.0, 0.0, 0.0); cfg.candidates_per_generation];
    let mut candidate_scores: Vec<f32> = Vec::with_capacity(cfg.candidates_per_generation);
    let mut tracked_best_score = f32::INFINITY;
    let mut recent_history = ShiftRegister::new();

    performance.record(Metric::Initialization, initialization_sw.elapse());

    // Padding for the live display
    print!("{}", "\n".repeat(5));

    // Main loop - every cycle of this adds one image to the final output
    let mut i: usize = 1;
    while i <= cfg.total_images {
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
            // To avoid moving the array out of scope, we instead sort an array of indicies based on the score values
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
                let mut parents: Vec<Candidate> = Vec::with_capacity(cfg.survival_threshold);
                for j in 0..cfg.survival_threshold {
                    let index = indicies[j];
                    let candidate = candidates[index];
                    parents.push(candidate);
                }
                candidates = candidate_gen_shader.run(
                    &mut performance,
                    &cfg,
                    &mut rng,
                    &context,
                    &parents,
                )?;
            } else {
                // This is the final iteration, so we get the best candidate and put it to the top
                candidates[0] = candidates[indicies[0]];
                candidate_scores[0] = candidate_scores[indicies[0]];
            }
        }

        // Ensure winner is actually an improvment
        let mut winner = candidates[0];
        let current_winning_score = candidate_scores[0];

        // Terminal progress information and register updates
        let scale_factor = cfg.total_images as f32 / 100.0;
        let filled = i as f32 / 5.0 / scale_factor;
        let empty = (cfg.total_images as f32 / 5.0 / scale_factor) - filled;
        print!("\x1b[4A");
        print!(
            "\r\x1b[2KProgress: [{}{}] {}/{}\n",
            "=".repeat(filled.round() as usize),
            " ".repeat(empty.round() as usize),
            i,
            cfg.total_images
        );
        // Because the downscale factor F reduces the number of pixels by F², we need to remultiply twice to normalise the score regardless of factor
        // internally the value doesnt matter, as we just need to know if one is bigger than another, but smaller images having lower scores is misleading for debugging / users
        print!(
            "\r\x1b[2KBest score: {:.4}\nCurrent iteration score: {:.4}{}\n",
            tracked_best_score
                / (cfg.extra_data.target_dimensions.0 * cfg.extra_data.target_dimensions.1) as f32,
            current_winning_score
                / (cfg.extra_data.target_dimensions.0 * cfg.extra_data.target_dimensions.1) as f32,
            {
                // Updating the score tracker and recent history inside a fucking print statement isnt ideal but whatever
                if tracked_best_score >= current_winning_score {
                    tracked_best_score = current_winning_score;
                    recent_history.push(false);
                    "\x1b[1;32m↓\x1b[0m" // buncha ansi code stuff, \x1b[1;32m just means "print bold green" and \x1b[0m just means "go back to normal"
                } else {
                    recent_history.push(true);
                    "\x1b[1;31m↑\x1b[0m"
                }
            }
        );
        print!(
            "\r\x1b[2KRecent failure percent: {:.1}%\n",
            (recent_history.fullness()) * 100.0
        );
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        // Draw the winning candidate onto the internal canvas
        draw_shader.run_small(
            &mut performance,
            &context,
            &winner,
            &context.buffers.scaled_canvas_texture,
        )?;

        // Scale up the winner's position and scale for the output canvas
        winner.scale *= cfg.extra_data.downscale_factor;
        winner.pos_x *= cfg.extra_data.downscale_factor;
        winner.pos_y *= cfg.extra_data.downscale_factor;
        draw_shader.run_large(
            &mut performance,
            &context,
            &winner,
            &context.buffers.unscaled_canvas_texture,
        )?;

        // Write the frame into the video, if enabled
        if cfg.enable_hue {
            video_generation_sw.start(None);
            let img = buffer_utils::texture_to_u8(
                &cfg,
                &context,
                &context.buffers.unscaled_canvas_texture,
            )?;
            if let Some(video_writer) = &mut video {
                video_writer.write_frame(img)?;
            }
            performance.record(Metric::VideoGeneration, video_generation_sw.elapse());
        }

        i += 1;
    }
    println!("");

    // Save images and logs at the end
    save_output(&cfg, &context)?;
    if let Some(video) = video {
        video.finish()?;
    }
    performance.save_results(
        &cfg,
        candidate_scores[0]
            / (cfg.extra_data.target_dimensions.0 * cfg.extra_data.target_dimensions.1) as f32,
    )?;

    println!("Done!");
    return Ok(());
}

// TODO: Dynamic save location based on config
fn save_output(cfg: &Config, context: &GpuContext) -> Result<(), Box<dyn std::error::Error>> {
    let img =
        buffer_utils::texture_to_image(&cfg, &context, &context.buffers.scaled_canvas_texture)?;
    img.save("debug/testing_output/test_output_internal.png")?;

    let img =
        buffer_utils::texture_to_image(&cfg, &context, &context.buffers.unscaled_canvas_texture)?;
    img.save("debug/testing_output/test_output.png")?;

    return Ok(());
}

// TODO
// ok so this video is actually very cool we should probably implement it properly but im not sure if we do that before or after the algorithm fixes
// algorithm wise, i think we need to have the program include the previous best N candidates into the next iteration, and also have the program try to detect if a certain %
// of recent iterations have stagnated (score is the same or worse), and if so, starts increasing the mutation strength over and over untill stagnatation is resolved, and then mutation
// strength can be brought back down to the inital value
//
//
