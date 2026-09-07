use std::time::Duration;

/// A manual stopwatch that can be re-used and is manually started and stopped
/// Used for timing individual peices of code
pub struct Stopwatch {
    text: Option<String>,
    time: std::time::Instant,
}

impl Stopwatch {
    /// A blank stopwatch with no text
    pub fn new() -> Stopwatch {
        return Stopwatch {
            text: None,
            time: std::time::Instant::now(),
        };
    }
    
    pub fn start(&mut self, text: Option<&str>) {
        self.text = match text {
            Some(x) => Some(x.to_string()),
            None => None,
        } ;
        self.time = std::time::Instant::now();
    }

    pub fn elapse(&self) -> Duration {
        // Only print if actually given any text
        // Shouldnt be flooding output now should we...
        if let Some(s) = &self.text {
            println!("{} took {:?}", s, self.time.elapsed());
        }
        return self.time.elapsed();
    }
}
