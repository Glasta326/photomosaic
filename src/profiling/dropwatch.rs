/// An automatic start-stop stopwatch that starts upon creation and ends when dropped.
/// Used for timing whole functions 
pub struct Dropwatch {
    text: &'static str,

    time: std::time::Instant,
}

impl Drop for Dropwatch {
    fn drop(&mut self) {
        println!("{} took {:?}", self.text, self.time.elapsed());
    }
}

impl Dropwatch {
    pub fn new(text: &'static str) -> Self {
        return Dropwatch {
            text,
            time: std::time::Instant::now(),
        };
    }
}
