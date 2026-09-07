/// A lazy Start-Stop stopwatch that stops and prints elapsed time once it dropped from scope
/// Should only be used to time whole functions that are not run repeatedly
pub struct Dropwatch {
    text: String,
    time: std::time::Instant,
}

impl Drop for Dropwatch {
    fn drop(&mut self){
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
