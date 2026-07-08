/// A manual stopwatch that can be re-used and is manually started and stopped
/// Used for timing individual peices of code
pub struct Stopwatch {
    text: String,

    time: std::time::Instant,
}

impl Stopwatch {
    pub fn start(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.time = std::time::Instant::now();
    }

    pub fn end(&self) {
        println!("{} took {:?}", self.text, self.time.elapsed());
    }
}

/// A blank stopwatch with no text
pub fn new() -> Stopwatch {
    return Stopwatch {
        text: String::new(),
        time: std::time::Instant::now(),
    };
}
