use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Counter {
    pub count: usize,
    pub max: usize,
    pub previous_count: usize,
    pub rolling_total: usize, // to calcualte rolling mean from
}

impl Counter {
    pub fn new() -> Counter {
        Counter {
            count: 0,
            max: 0,
            previous_count: 0,
            rolling_total: 0,
        }
    }

    pub fn set_max(&mut self, max: usize) {
        self.max = max;
    }

    pub fn reset(&mut self) {
        self.previous_count = self.count;
        self.rolling_total += self.count;
        self.count = 0;
    }

    pub fn increment(&mut self) {
        self.count += 1;
    }
}
