use std::{collections::HashMap, fs::File, path::PathBuf, time::Duration};

use crate::config_parse::Config;

#[derive(Hash, Eq, PartialEq)]
pub enum Metric {
    ScoreShader,
    DrawShader,
    MainIteration,
    EvolutionCycle,
}

pub struct RuntimeStats {
    stats: HashMap<Metric, Stat>,
}

struct Stat {
    records: Vec<Duration>,
    total: Duration,
    min: Duration,
    max: Duration,
}

impl RuntimeStats {
    pub fn init() -> Result<Self, Box<dyn std::error::Error>> {
        let mut map = HashMap::<Metric, Stat>::new();

        // I really wish there was a way to iterate over enum entries
        map.insert(Metric::DrawShader, Stat::default());
        map.insert(Metric::ScoreShader, Stat::default());
        map.insert(Metric::EvolutionCycle, Stat::default());
        map.insert(Metric::MainIteration, Stat::default());

        return Ok(RuntimeStats { stats: (map) });
    }

    /// Submits a single time to this metric
    pub fn record(&mut self, metric: Metric, time: Duration) {
        let s = self.stats.get_mut(&metric).unwrap();

        s.records.push(time);
        s.total += time;

        if time < s.min {
            s.min = time;
        } else if time > s.max {
            s.max = time;
        }
    }

    /// Prints a condensed version of the runtime data to the console
    pub fn display_summary(&self, cfg: &Config) {
        let mut evo_avg = Duration::ZERO;
        let mut iter_avg = Duration::ZERO;
        let mut score_shader_avg = Duration::ZERO;
        let mut draw_shader_avg = Duration::ZERO;

        let evo_times = self.stats[&Metric::EvolutionCycle].records.clone();
        let iter_times = self.stats[&Metric::MainIteration].records.clone();
        let score_shader_times = self.stats[&Metric::ScoreShader].records.clone();
        let draw_shader_times = self.stats[&Metric::DrawShader].records.clone();

        // Average evo cycle time
        for &t in &evo_times {
            evo_avg += t;
        }
        evo_avg /= evo_times.len() as u32;

        // Average iteration time
        for &t in &iter_times {
            iter_avg += t;
        }
        iter_avg /= iter_times.len() as u32;

        // Average score shader time
        for &t in &score_shader_times {
            score_shader_avg += t;
        }
        score_shader_avg /= score_shader_times.len() as u32;

        // Average draw shader time
        for &t in &draw_shader_times {
            draw_shader_avg += t;
        }
        draw_shader_avg /= draw_shader_times.len() as u32;

        println!(
            "
            Profiling report:
            Main iteration time: [Avg: {:?}, min: {:?}, max: {:?}]
            Evo cycle time: [Avg: {:?}, min: {:?}, max: {:?}]
            Score shader time: [Avg: {:?}, min: {:?}, max: {:?}]
            Draw shader time: [Avg: {:?}, min: {:?}, max: {:?}]
            ",
            iter_avg,
            self.stats[&Metric::MainIteration].min,
            self.stats[&Metric::MainIteration].max,
            evo_avg,
            self.stats[&Metric::EvolutionCycle].min,
            self.stats[&Metric::EvolutionCycle].max,
            score_shader_avg,
            self.stats[&Metric::ScoreShader].min,
            self.stats[&Metric::ScoreShader].max,
            draw_shader_avg,
            self.stats[&Metric::DrawShader].min,
            self.stats[&Metric::DrawShader].max,
        );
    }

    /// Saves the result data to the config-specifed log file location if enabled
    pub fn save_results() -> Result<(), Box<dyn std::error::Error>> {
        println!();
        // TODO
        return Ok(());
    }
}

impl Default for Stat {
    fn default() -> Self {
        Self {
            records: Vec::default(),
            total: Duration::ZERO,
            min: Duration::ZERO,
            max: Duration::MAX,
        }
    }
}
