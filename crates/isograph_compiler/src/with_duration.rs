use std::time::{Duration, Instant};

pub struct WithDuration<T> {
    pub elapsed_time: Duration,
    pub item: T,
}

impl<T> WithDuration<T> {
    pub fn new(calculate: impl FnOnce() -> T) -> WithDuration<T> {
        eprintln!("new with d");
        let start = Instant::now();
        let item = calculate();
        eprintln!("new with d 2");
        WithDuration {
            elapsed_time: start.elapsed(),
            item,
        }
    }
}
