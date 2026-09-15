use crate::{
    config_parse::Config,
    profiling::Metric::{CandidatePopulation, ScoreIndexSorting},
};
use chrono::{DurationRound, TimeDelta};
use std::{collections::HashMap, fs::File, io::Write, time::Duration};

/// A collection of substatial metrics for this program's logic
/// Assume this enum may gain or lose entries over time
#[derive(Hash, Eq, PartialEq, Clone)]
pub enum Metric {
    ScoreShader,
    DrawShader,
    Initialization,
    CandidatePopulation,
    ScoreIndexSorting,
    CandidateReproduction,
    VideoGeneration,
}

impl Metric {
    pub const ALL: [Metric; 7] = [
        Metric::ScoreShader,
        Metric::DrawShader,
        Metric::Initialization,
        Metric::CandidatePopulation,
        Metric::ScoreIndexSorting,
        Metric::CandidateReproduction,
        Metric::VideoGeneration,
    ];

    pub fn name(&self) -> &str {
        match self {
            Metric::ScoreShader => "Score shader",
            Metric::DrawShader => "Draw shader",
            Metric::Initialization => "Initialization",
            Metric::CandidatePopulation => "Candidate populating",
            Metric::ScoreIndexSorting => "Score index sorting",
            Metric::CandidateReproduction => "Candidate reproduction",
            Metric::VideoGeneration => "Video generation",
        }
    }
}

pub struct RuntimeStats {
    stats: HashMap<Metric, Stat>,

    /// Used to record the total time the relevant computation took to complete
    /// "relevant" being loading atlases, running scores and ect. Not displaying stats at the end or any niche cleanup
    total_time: std::time::Instant,
}

struct Stat {
    records: Vec<Duration>,
    total: Duration,
    min: Duration,
    max: Duration,
}

impl RuntimeStats {
    /// Initalises the map with an empty statsheet for each metric
    pub fn init() -> Result<Self, Box<dyn std::error::Error>> {
        let mut map = HashMap::<Metric, Stat>::new();
        for m in Metric::ALL {
            map.insert(m, Stat::default());
        }

        return Ok(RuntimeStats {
            stats: (map),
            total_time: std::time::Instant::now(),
        });
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
                    "\n{} time:\n [Avg: {:?}, min: {:?}, max: {:?}]\n",
                    metric.name(),
                    avg,
                    times.first().unwrap(),
                    times.last().unwrap()
                )
                .as_str(),
            );
        }

        println!("{}", text);
    }

    /// Saves the result data to the config-specifed log file location if enabled
    /// Should be the final performance-profiling related function called
    pub fn save_results(
        &self,
        cfg: &Config,
        final_score: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let total_time_taken = self.total_time.elapsed();

        // Initalise file and stringbuilder
        let fp = cfg.profile_log_fp.join("performance_log.txt");
        let mut f = File::create(&fp)?;
        let mut text = String::new();

        // The file timestamp doesn't need nanosecond accuracy lol
        let timestamp = chrono::Local::now()
            .naive_local()
            .duration_round(TimeDelta::seconds(1))
            .unwrap();
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

        text.push_str(format!("\n[Configuration]:{}\n", cfg.display()).as_str());

        text.push_str(format!("\n[Extra data]:{}\n", cfg.extra_data).as_str());
        text.push_str(format!("Final score:       {:.4}\n", final_score).as_str());

        text.push_str(&format!("\n[Profiling report]:\n").to_string());
        text.push_str(
            &format!("------------------------------------------------------\n").to_string(),
        );
        text.push_str(format!("Total program execution time: {:?}\n", total_time_taken).as_str());

        // Append metric information string in order based on total time for that metric
        // That way, metrics that take up more time are prioritised and shown at the top
        let mut sorted_metrics = Metric::ALL.to_vec();
        sorted_metrics.sort_unstable_by(|a, b| self.stats[a].total.cmp(&self.stats[b].total));
        sorted_metrics.reverse(); // We want bigger time at the front
        for metric in sorted_metrics {
            // If nothing was recorded for this metric, then skip it entirely
            if self.stats[&metric].total == Duration::ZERO {
                continue;
            }

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

            // Total
            let total = self.stats[&metric].total;

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
            s = s.sqrt();
            let std_dev = Duration::from_nanos_u128(s as u128); //.div_duration_f64(avg);

            text.push_str(format!("Samples: {:?}\nAverage: {:?}\nStd.Dev: {:?}\nMin: {:?}\nP95: {:?}\nP99: {:?}\nMax: {:?}\nTotal: {:?}\n", times.len(), avg, std_dev, min, p_95, p_99, max, total).as_str());
        }

        text.push_str(
            &format!("------------------------------------------------------\n").to_string(),
        );

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
