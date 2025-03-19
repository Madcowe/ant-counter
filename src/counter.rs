use jiff::{ToSpan, Zoned};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Counter {
    pub count: usize,
    pub max: usize,
    pub previous_count: usize,
    pub rolling_total: usize, // to calcualte rolling mean from
    pub reset_zoned_date_time: Zoned,
}

impl Counter {
    pub fn new() -> Result<Counter, jiff::Error> {
        Ok(Counter {
            count: 0,
            max: 0,
            previous_count: 0,
            rolling_total: 0,
            reset_zoned_date_time: get_a_minute_from_now()?,
        })
    }

    pub fn set_max(&mut self, max: usize) {
        self.max = max;
    }

    pub fn reset(&mut self) {
        self.previous_count = self.count;
        self.rolling_total += self.count;
        self.count = 0;
    }

    // checks if time is past rest_zoned_data_time and if so resets the counter
    // and updates reset_zoned_date_time to next period start
    pub fn reset_if_next_period(&mut self) -> Result<(), jiff::Error> {
        let now = Zoned::now();
        if now > self.reset_zoned_date_time {
            self.reset();
            self.reset_zoned_date_time = get_a_minute_from_now()?;
            println!("Reseting as in new period")
        }
        Ok(())
    }

    pub fn increment(&mut self) {
        self.count += 1;
    }
}

fn get_start_of_next_week() -> Result<Zoned, jiff::Error> {
    let now = Zoned::now().start_of_day()?;
    let days_to_next_week = 7 - now.weekday().to_monday_zero_offset();
    Ok(&now + days_to_next_week.days())
}

// alternate period for testing
fn get_a_minute_from_now() -> Result<Zoned, jiff::Error> {
    let now = Zoned::now();
    Ok(&now + 1.minute())
}
