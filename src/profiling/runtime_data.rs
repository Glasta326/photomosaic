use std::{format, println, time::Duration};

use crate::{config_parse::Config, gpu::score_shader};

// Stores frame times for each main iteration and evolution cycle for display after program completion
pub struct RuntimeData {
    evo_times: Vec<Duration>,
    iter_times: Vec<Duration>,
    score_shader_times: Vec<Duration>,
    draw_shader_times: Vec<Duration>,
}

impl RuntimeData {
    pub fn new() -> Self {
        return RuntimeData {
            evo_times: Vec::new(),
            iter_times: Vec::new(),
            score_shader_times: Vec::new(),
            draw_shader_times: Vec::new(),
        };
    }

    pub fn add_evo_time(&mut self, t: Duration) {
        self.evo_times.push(t);
    }

    pub fn add_iter_time(&mut self, t: Duration) {
        self.iter_times.push(t);
    }

    pub fn add_score_shader_time(&mut self, t: Duration) {
        self.score_shader_times.push(t);
    }

    pub fn add_draw_shader_time(&mut self, t: Duration) {
        self.draw_shader_times.push(t);
    }

    pub fn display_stats(&mut self, cfg: &Config) {
        let mut evo_avg = Duration::ZERO;
        let mut iter_avg = Duration::ZERO;
        let mut score_shader_avg = Duration::ZERO;
        let mut draw_shader_avg = Duration::ZERO;

        self.evo_times.sort();
        self.iter_times.sort();
        self.score_shader_times.sort();
        self.draw_shader_times.sort();

        // Average evo cycle time
        for &t in &self.evo_times {
            evo_avg += t;
        }
        evo_avg /= self.evo_times.len() as u32;

        // Average iteration time
        for &t in &self.iter_times {
            iter_avg += t;
        }
        iter_avg /= self.iter_times.len() as u32;

        // Average score shader time
        for &t in &self.score_shader_times {
            score_shader_avg += t;
        }
        score_shader_avg /= self.score_shader_times.len() as u32;

        // Average draw shader time
        for &t in &self.draw_shader_times {
            draw_shader_avg += t;
        }
        draw_shader_avg /= self.draw_shader_times.len() as u32;

        let text = format!(
            "
            Evolution cycle time data: [Average: {:?}, Min: {:?}, Max: {:?}],
            Total iteration time data: [Average: {:?}, Min: {:?}, Max: {:?}],
            
            score shader time data: [Average: {:?}, Scaled average: {:?}, Min: {:?}, Max :{:?}],
            draw shader time data: [Average: {:?}, Scaled average: {:?}, Min: {:?}, Max :{:?}],
            ",
            evo_avg,
            self.evo_times.first(),
            self.evo_times.last(),
            iter_avg,
            self.iter_times.first(),
            self.iter_times.last(),
            score_shader_avg,
            score_shader_avg / cfg.candidates_per_generation as u32,
            self.score_shader_times.first(),
            self.score_shader_times.last(),
            draw_shader_avg,
            draw_shader_avg / cfg.candidates_per_generation as u32,
            self.draw_shader_times.first(),
            self.draw_shader_times.last(),
        );
        println!("{}", text);
    }
}
