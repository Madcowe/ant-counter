use autonomi::client::payment::PaymentOption;
use autonomi::client::scratchpad::Bytes;
use autonomi::{Client, Scratchpad, SecretKey, Wallet};
use eyre::Result;
use jiff::{ToSpan, Zoned};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::task::Wake;

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
    pub fn reset_if_next_period(&mut self) -> Result<bool, jiff::Error> {
        let mut reset = false;
        let now = Zoned::now();
        if now > self.reset_zoned_date_time {
            self.reset();
            self.reset_zoned_date_time = get_a_minute_from_now()?;
            reset = true;
            println!("Reseting as in new period")
        }
        Ok(reset)
    }

    pub fn increment(&mut self) {
        self.count += 1;
    }
}

pub struct CreationItems {
    wallet: Wallet,
}

pub struct ConnectedScratchpad {
    client: Client,
    scratchpad: Scratchpad,
    key: SecretKey,
}

pub enum AppMode {
    Initiating,
    Creating(),
    Counting(CountingMode),
}

pub enum CountingMode {
    Local,
    Connected(ConnectedScratchpad),
}

pub struct CounterApp {
    pub app_mode: AppMode,
    pub counter: Counter,
}

pub enum CreationError<'a> {
    FailedToCreateFile(&'a Path),
    FailedToWriteToFile(&'a Path),
    FailedToIntiateClient(autonomi::client::ConnectError),
    FailedToCreatCounter(jiff::Error),
    FailedToSerailzeCounter(bincode::Error),
    FailedToCreateScratchPad(autonomi::client::data_types::scratchpad::ScratchpadError),
}

impl CounterApp {
    pub fn new() -> Result<CounterApp, jiff::Error> {
        Ok(CounterApp {
            app_mode: AppMode::Initiating,
            counter: Counter::new()?,
        })
    }

    pub async fn create(&mut self, path: &Path, wallet: Wallet) -> Result<()> {
        let key = autonomi::SecretKey::random();
        let key_hex = key.to_hex();
        println!("New key: {}", key_hex);
        let mut file = File::create_new(&path)?;
        file.write_all(key_hex.as_bytes())?;
        // initiate a client (connect)
        let client = Client::init_local().await?;
        self.counter = Counter::new()?;
        let counter_seralized = bincode::serialize(&self.counter)?;
        let content = Bytes::from(counter_seralized);
        let payment_option = PaymentOption::from(wallet);
        let content_type = 99;
        let (cost, addr) = client
            .scratchpad_create(&key, content_type, &content, payment_option)
            .await?;
        println!("Scratchpad created, cost: {cost} addr {addr}");
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        let scratchpad = client.scratchpad_get(&addr).await?;
        let connected_scratchpad = ConnectedScratchpad {
            client: client,
            scratchpad: scratchpad,
            key: key,
        };
        self.app_mode = AppMode::Counting(CountingMode::Connected(connected_scratchpad));
        Ok(())
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
