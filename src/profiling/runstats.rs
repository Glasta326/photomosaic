use core::time;
use std::{
    collections::HashMap,
    fs::File,
    io::Write,
    ops::Add,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::{
    config_parse::Config,
    utils::math_utils::{self, average},
};

#[derive(Hash, Eq, PartialEq)]
pub enum Metric {
    ScoreShader,
    DrawShader,
    MainIteration,
    EvolutionCycle,
}

impl Metric {
    pub const ALL: [Metric; 4] = [
        Metric::ScoreShader,
        Metric::DrawShader,
        Metric::MainIteration,
        Metric::EvolutionCycle,
    ];

    pub fn name(&self) -> &str {
        match self {
            Metric::ScoreShader => "Score shader",
            Metric::DrawShader => "Draw shader",
            Metric::MainIteration => "Main iteration",
            Metric::EvolutionCycle => "Evolution cycle",
        }
    }
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
    // Initalise the map with an empty statsheet for each metric
    pub fn init() -> Result<Self, Box<dyn std::error::Error>> {
        let mut map = HashMap::<Metric, Stat>::new();
        for m in Metric::ALL {
            map.insert(m, Stat::default());
        }

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
    pub fn display_summary(&self) {
        let mut text = String::new();
        text.push_str("Profiling report:\n");

        // Go over each recorded metric's times, calculate average and append string with formatted data
        for metric in Metric::ALL {
            let mut times = self.stats[&metric].records.clone();
            times.sort();

            let mut avg = Duration::ZERO;
            for &t in &times {
                avg += t;
            }
            avg = avg.div_f32(times.len() as f32);

            text.push_str(
                format!(
                    "{} time: [Avg: {:?}, min: {:?}, max: {:?}]\n",
                    metric.name(),
                    avg,
                    times.first(),
                    times.last()
                )
                .as_str(),
            );
        }

        println!("{}", text);
    }

    /// Saves the result data to the config-specifed log file location if enabled
    pub fn save_results(&self, cfg: &Config) -> Result<(), Box<dyn std::error::Error>> {
        // Initalise file and stringbuilder
        let fp = cfg.profile_log_fp.join("performance_log.txt");
        let mut f = File::create(&fp)?;
        let mut text = String::new();

        let timestamp = chrono::Local::now().naive_local();
        // Header info
        text.push_str(
            format!(
                "{} v{} - [{}]\n",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
                timestamp
            )
            .as_str(),
        );

        for metric in Metric::ALL {
            // Add metric name to section
            text.push_str(format!("\n[{}]:\n", metric.name()).as_str());

            let mut times = self.stats[&metric].records.clone();
            if times.len() <= 0 {
                continue;
            }
            times.sort();

            // Average
            let mut avg = Duration::ZERO;
            for &t in &times {
                avg += t;
            }
            avg = avg.div_f32(times.len() as f32);

            // Min/Max
            let min = times.first().unwrap();
            let max = times.last().unwrap();

            // Sum
            let mut sum = Duration::ZERO;
            for &t in &times {
                sum += t;
            }

            // P99 / P95
            let p_95 = times[f32::round((times.len() - 1) as f32 * 95.0 / 100.0) as usize];
            let p_99 = times[f32::round((times.len() - 1) as f32 * 99.0 / 100.0) as usize];

            // Std.Dev
            let mut s = 0.0;
            for &t in &times {
                let diff = t.as_nanos() as f64 - avg.as_nanos() as f64;
                s += diff * diff;
            }
            s /= times.len() as f64;
            let std_dev = Duration::from_nanos_u128(s.sqrt() as u128);

            text.push_str(format!("Samples: {:?}\nAverage: {:?}\nStd.Dev: {:?}\nMin: {:?}\nP95: {:?}\nP99: {:?}\nMax: {:?}\nTotal: {:?}\n", times.len(), avg, std_dev, min, p_95, p_99, max, sum).as_str());
        }

        f.write_all(text.as_bytes())?;
        println!("Runtime statistics saved to: {}", fp.display());

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
