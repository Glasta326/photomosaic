/// A manual stopwatch that can be re-used and is manually started and stopped
/// Used for timing individual peices of code
pub struct Stopwatch {
    text: &'static str,

    time: std::time::Instant,
}

impl Stopwatch {
    pub fn start(&mut self, text: &'static str){
        self.text = text;
        self.time = std::time::Instant::now();
    }

    pub fn end(&self) {
        println!("{} took {:?}", self.text, self.time.elapsed());
    }
}

/// A blank stopwatch with no text
pub fn new() -> Stopwatch {
    return Stopwatch {
        text: "",
        time: std::time::Instant::now(),
    };
}
