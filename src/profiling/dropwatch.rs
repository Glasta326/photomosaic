/// An automatic start-stop stopwatch that starts upon creation and ends when dropped.
/// Used for timing whole functions 
pub struct Dropwatch {
    text: String,

    time: std::time::Instant,
}

impl Drop for Dropwatch {
    fn drop(&mut self) {
        println!("{} took {:?}", self.text, self.time.elapsed());
    }
}

impl Dropwatch {
    pub fn new(text: impl Into<String>) -> Self {
        return Dropwatch {
            text: text.into(),
            time: std::time::Instant::now(),
        };
    }
}
