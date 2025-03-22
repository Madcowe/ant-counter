use autonomi::client::payment::PaymentOption;
use autonomi::client::scratchpad::Bytes;
use autonomi::client::CONNECT_TIMEOUT_SECS;
use autonomi::{Client, Scratchpad, SecretKey, Wallet};
use eyre::Result;
use jiff::{ToSpan, Zoned};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;

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

impl CounterApp {

pub enum Error<'a> {
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

    fn connected_scratchpad(&mut self) -> Option<&ConnectedScratchpad> {
        if let AppMode::Counting(counting_mode) = &self.app_mode {
            if let CountingMode::Connected(connected_scatchpad) = counting_mode {
                return Some(connected_scatchpad);
            }
        }
        None
    }

    pub async fn create(&mut self, path: &Path, wallet: Wallet) -> Result<()> {
        // create new key and save to file
        let key = autonomi::SecretKey::random();
        let key_hex = key.to_hex();
        println!("New key: {}", key_hex);
        let mut file = File::create_new(&path)?;
        file.write_all(key_hex.as_bytes())?;
        // initiate a client (connect) and create local counter
        let client = Client::init_local().await?;
        self.counter = Counter::new()?;
        // searlize counter and create scratchpad with it
        let counter_seralized = bincode::serialize(&self.counter)?;
        let content = Bytes::from(counter_seralized);
        let payment_option = PaymentOption::from(wallet);
        let content_type = 99;
        let (cost, addr) = client
            .scratchpad_create(&key, content_type, &content, payment_option)
            .await?;
        println!("Scratchpad created, cost: {cost} addr {addr}");
        // wait for scratchpad to be replicated
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

    pub async fn connect(&mut self, key: SecretKey) -> Result<()> {
        let public_key = key.public_key();
        let client_option = Client::init_local().await;
        match client_option {
            Err(connect_error) => Err(connect_error.into()),
            Ok(client) => {
                let scratchpad = client.scratchpad_get_from_public_key(&public_key).await?;
                let connected_scratchpad = ConnectedScratchpad {
                    client: client,
                    scratchpad: scratchpad,
                    key: key,
                };
                self.app_mode = AppMode::Counting(CountingMode::Connected(connected_scratchpad));
                Ok(())
            }
        }
    }

    pub async fn download(&mut self) -> Result<()> {
        if let Some(counter_scratchpad) = self.connected_scratchpad() {
                connected_scratchpad
                    .client
                    .scratchpad_get(connected_scratchpad.scratchpad.address())
                    .await?;
        }
        Ok(())
    }
}
        // if let AppMode::Counting(counting_mode) = self.app_mode {
        //     if let CountingMode::Connected(connected_scratchpad) = counting_mode {
        //         connected_scratchpad
        //             .client
        //             .scratchpad_get(connected_scratchpad.scratchpad.address())
        //             .await?;
        //     }
        // }
        // Ok(())
        // match self.app_mode {
        //     AppMode::Counting(counting_mode) => match counting_mode {
        //         CountingMode::Connected(mut connected_scratchpad) => {
        //             connected_scratchpad.scratchpad = connected_scratchpad
        //                 .client
        //                 .scratchpad_get(connected_scratchpad.scratchpad.address())
        //                 .await?;
        //             Ok(())
        //         }
        //         _ => Ok(()),
        //     },
        //     _ => Ok(()),
        // }
//     }
// }

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
