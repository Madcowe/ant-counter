use autonomi::client::data_types::scratchpad::ScratchpadError;
use autonomi::client::payment::PaymentOption;
use autonomi::client::scratchpad::Bytes;
use autonomi::{Client, Scratchpad, SecretKey, Wallet};
use eyre::Result;
use jiff::{ToSpan, Zoned};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
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

pub enum CounterState {
    Initiating,
    WithKey(SecretKey),
    Connected {
        client: Client,
        scratchpad: Scratchpad,
        key: SecretKey,
    },
}

pub struct CounterApp {
    pub counter_state: CounterState,
    pub counter: Counter,
    pub content_type: u64,
}

// #[derive(Debug, thiserror::Error)]
// pub enum Error<'a> {
//     FailedToCreateFile(&'a Path),
//     FailedToWriteToFile(&'a Path),
//     FailedToIntiateClient(autonomi::client::ConnectError),
//     FailedToCreateCounter(jiff::Error),
//     FailedToSerailzeCounter(bincode::Error),
//     FailedToCreateScratchPad(autonomi::client::data_types::scratchpad::ScratchpadError),
//     FailedToGetNetworkCounter,
// }

impl CounterApp {
    pub fn new() -> Result<CounterApp, jiff::Error> {
        Ok(CounterApp {
            counter_state: CounterState::Initiating,
            counter: Counter::new()?,
            content_type: 99,
        })
    }

    pub async fn create(&mut self, path: &Path, wallet: &Wallet) -> Result<()> {
        // create new key and save to file
        let key = autonomi::SecretKey::random();
        let key_hex = key.to_hex();
        println!("New key: {}", key_hex);
        let mut file = File::create(&path)?;
        file.write_all(key_hex.as_bytes())?;
        // initiate a client (connect) and create local counter
        let client = Client::init_local().await?;
        self.counter = Counter::new()?;
        // searlize counter and create scratchpad with it
        let counter_seralized = bincode::serialize(&self.counter)?;
        let content = Bytes::from(counter_seralized);
        let payment_option = PaymentOption::from(wallet);
        let (cost, addr) = client
            .scratchpad_create(&key, self.content_type, &content, payment_option)
            .await?;
        println!("Scratchpad created, cost: {cost} addr {addr}");
        // wait for scratchpad to be replicated
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        let scratchpad = client.scratchpad_get(&addr).await?;
        self.counter_state = CounterState::Connected {
            client,
            scratchpad,
            key,
        };
        Ok(())
    }
}

//     pub fn set_key(&mut self, hex_key: &str) -> Result<()> {
//         Ok(())
//     }

//     // try and conneect to existing scratchpad
//     pub async fn connect(&mut self) -> Result<()> {
//         // match self.app_mode {
//         //     AppMode::Counting()
//         let key = self
//             .connected_scratchpad
//             .as_ref()
//             .ok_or(ScratchpadError::Missing)?
//             .key
//             .clone();
//         let public_key = key.public_key();
//         let client_option = Client::init_local().await;
//         match client_option {
//             Err(connect_error) => Err(connect_error.into()),
//             Ok(client) => {
//                 let scratchpad = client.scratchpad_get_from_public_key(&public_key).await?;
//                 self.connected_scratchpad = Some(ConnectedScratchpad {
//                     client,
//                     scratchpad,
//                     key,
//                 });
//                 self.app_mode = AppMode::Counting(CountingMode::Connected);
//                 Ok(())
//             }
//         }
//     }

//     // this interpets any error as inditive of not being connected hence false returned iseteaf of result
//     pub async fn is_connected(&self) -> bool {
//         let mut connected = false;
//         if let Some(connected_scratchpad) = &self.connected_scratchpad {
//             connected = connected_scratchpad
//                 .client
//                 .scratchpad_check_existance(&connected_scratchpad.scratchpad.address())
//                 .await
//                 .unwrap_or(false)
//         }
//         connected
//     }

//     pub async fn get_network_counter(&self) -> Result<Counter> {
//         if let Some(connected_scratchpad) = &self.connected_scratchpad {
//             let counter: Counter = bincode::deserialize(
//                 &connected_scratchpad
//                     .client
//                     .scratchpad_get(&connected_scratchpad.scratchpad.address())
//                     .await?
//                     .decrypt_data(&connected_scratchpad.key)?,
//             )?;
//             return Ok(counter);
//         }
//         Err(ScratchpadError::Missing)? // replace with local error
//     }

//     pub async fn download(&mut self) -> Result<()> {
//         if let Some(mut connected_scratchpad) = self.connected_scratchpad.as_mut() {
//             connected_scratchpad.scratchpad = connected_scratchpad
//                 .client
//                 .scratchpad_get(connected_scratchpad.scratchpad.address())
//                 .await?;
//         }
//         Ok(())
//     }

//     pub async fn upload(&mut self) -> Result<()> {
//         let counter = self.counter.clone();
//         print!("{:?}", &self.counter);
//         let counter_serailzed = bincode::serialize(&self.counter)?;
//         let content = Bytes::from(counter_serailzed);
//         let content_type = self.content_type;
//         if let Some(counter_scratchpad) = &self.connected_scratchpad {
//             print!("Syncing to antnet...");
//             counter_scratchpad
//                 .client
//                 .scratchpad_update(&counter_scratchpad.key, content_type, &content)
//                 .await?;
//             while counter != self.get_network_counter().await? {
//                 tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
//                 println!("Syncing to ant network...");
//             }
//             println!("Synced");
//             return Ok(());
//         }
//         Err(ScratchpadError::Missing)? // replace with local error
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
