use autonomi::client::payment::PaymentOption;
use autonomi::client::scratchpad;
use autonomi::client::scratchpad::Bytes;
use autonomi::{Client, Network, Scratchpad, SecretKey, Wallet};
use eyre::Result;
use jiff::{ToSpan, Zoned};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub struct LastSixValues {
    values: Vec<usize>,
}

impl LastSixValues {
    pub fn new() -> LastSixValues {
        LastSixValues { values: Vec::new() }
    }

    pub fn add(&mut self, value: usize) {
        if self.values.len() >= 6 {
            self.values.remove(0); // could use VecDeque if performance were an issue
        }
        self.values.push(value);
    }

    pub fn get_last_value(&self) -> usize {
        if self.values.len() > 0 {
            return self.values[self.values.len() - 1];
        } else {
            0
        }
    }

    pub fn get_mean(&self) -> f64 {
        let total = self.values.iter().sum::<usize>() as f64;
        total / self.values.len() as f64
    }
}

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
            reset_zoned_date_time: get_start_of_next_week()?,
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
            self.reset_zoned_date_time = get_start_of_next_week()?;
            // self.reset_zoned_date_time = get_a_minute_from_now()?;
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
    Local,
    LocalWithKey(SecretKey),
    Connected {
        client: Client,
        scratchpad: Scratchpad,
        key: SecretKey,
    },
    Quitting,
}

// so only matches on enum name not any cotanied elements
impl PartialEq for CounterState {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (CounterState::Initiating, CounterState::Initiating) => true,
            (CounterState::Local, CounterState::Local) => true,
            (CounterState::LocalWithKey(_), CounterState::LocalWithKey(_)) => true,
            (CounterState::Connected { .. }, CounterState::Connected { .. }) => true,
            (CounterState::Quitting, CounterState::Quitting) => true,
            _ => false,
        }
    }
}

pub struct CounterApp {
    pub counter_state: CounterState,
    pub counter: Counter,
    pub content_type: u64,
    pub disconnected_count: usize, // counts increment during disconnected periods
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
            disconnected_count: 0,
        })
    }

    pub async fn create(&mut self, path: &Path, private_key: &str) -> Result<()> {
        // create new key and save to file
        let key = autonomi::SecretKey::random();
        let key_hex = key.to_hex();
        println!("New key: {}", key_hex);
        let mut file = File::create(&path)?;
        file.write_all(key_hex.as_bytes())?;
        // create local counter
        self.counter = Counter::new()?;
        // attempt to creat wallet
        let wallet = match get_funded_wallet(&private_key).await {
            Err(_) => {
                println!("Cannot get funds to create wallet.");
                self.counter_state = CounterState::LocalWithKey(key);
                return Ok(());
            }
            Ok(wallet) => wallet,
        };
        // attempt to connect safenet and create new scratch pad
        if let Ok(client) = Client::init_local().await {
            // seralize counter and create scratchpad with it
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
            return Ok(());
        }
        println!("Cannot connect to antnet to create scratchpad...using local counter");
        self.counter_state = CounterState::LocalWithKey(key);
        Ok(())
    }

    pub fn increment(&mut self) {
        self.counter.increment();
        if self.get_counter_state() != "Connected" {
            self.disconnected_count += 1;
        }
    }

    pub fn reset(&mut self) {
        self.counter.reset();
        self.disconnected_count = 0;
    }

    pub fn get_counter_state(&self) -> &str {
        match self.counter_state {
            CounterState::Initiating => "Initiating",
            CounterState::Local => "Local",
            CounterState::LocalWithKey(_) => "Local With Key",
            CounterState::Connected { .. } => "Connected",
            CounterState::Quitting => "Quitting",
        }
    }

    pub fn print_scratchpad(&self) -> Result<()> {
        if let CounterState::Connected { scratchpad, .. } = &self.counter_state {
            println!(
                "scratchpad version {:?}, value: {:?}",
                scratchpad.counter(),
                self.counter
            );
        }
        Ok(())
    }

    pub fn set_key_from_hex(&mut self, hex_key: &str) -> Result<()> {
        self.counter_state = CounterState::LocalWithKey(SecretKey::from_hex(&hex_key)?);
        if let Some(key) = self.get_key() {
            println!("Key loaded: {}", key.to_hex());
        }
        Ok(())
    }

    pub fn get_key(&self) -> Option<&SecretKey> {
        match &self.counter_state {
            CounterState::LocalWithKey(key) => Some(key),
            CounterState::Connected { key, .. } => Some(key),
            _ => None,
        }
    }

    // try and connect to existing scratchpad
    pub async fn connect(&mut self) -> Result<()> {
        let Some(key) = self.get_key() else {
            match self.counter_state {
                CounterState::Initiating => {
                    println!("No key is loaded");
                    return Ok(());
                }
                _ => return Ok(()), // if they was never a key it just continues to run locally
            }
        };
        let key = key.clone();
        let public_key = key.public_key();
        let Ok(client) = Client::init_local().await else {
            println!("Can't connect to antnet...using local counter");
            self.counter_state = CounterState::LocalWithKey(key);
            return Ok(());
        };
        let Ok(scratchpad) = client.scratchpad_get_from_public_key(&public_key).await else {
            println!("No scratchpad with that key on antnet...using local counter");
            self.counter_state = CounterState::Local;
            return Ok(());
        };
        self.counter = bincode::deserialize(&scratchpad.decrypt_data(&key)?)?;
        self.counter.count += self.disconnected_count;
        self.counter_state = CounterState::Connected {
            client,
            scratchpad,
            key: key.clone(),
        };
        // reset disconnected count
        self.disconnected_count = 0;
        // sync the new counter value by uploading and downloading,
        // if just connecting from intilising to existing scratchpad this is uneccesary and make extra version of scratchpad
        self.upload().await?;
        self.download().await?;
        Ok(())
    }

    // to test disconnecting as reconnecting as can't seme to restart local network
    pub fn disconnect(&mut self) {
        let Some(key) = self.get_key() else { return };
        self.counter_state = CounterState::LocalWithKey(key.clone());
    }

    pub async fn get_network_counter(&self) -> Result<Counter> {
        let CounterState::Connected {
            client,
            scratchpad,
            key,
        } = &self.counter_state
        else {
            println!("Can't get network counter");
            return Err(scratchpad::ScratchpadError::Missing.into()); // replace with local error
        };
        let counter = bincode::deserialize(
            &client
                .scratchpad_get(&scratchpad.address())
                .await?
                .decrypt_data(&key)?,
        )?;
        Ok(counter)
    }

    pub async fn upload(&mut self) -> Result<()> {
        let counter = self.counter.clone();
        let counter_serailzed = bincode::serialize(&self.counter)?;
        let content = Bytes::from(counter_serailzed);
        let CounterState::Connected {
            client,
            scratchpad: _,
            key,
        } = &self.counter_state
        else {
            println!("Not connected");
            return Ok(());
        };
        println!("Syncing to antnet...");
        client
            .scratchpad_update(&key, self.content_type, &content)
            .await?;
        while counter != self.get_network_counter().await? {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            println!("Syncing to ant network...");
        }
        println!("Synced");
        Ok(())
    }

    pub async fn download(&mut self) -> Result<()> {
        let CounterState::Connected {
            client,
            scratchpad,
            key,
        } = &mut self.counter_state
        else {
            println!("Not connected to antnet");
            // self.counter_state = match self.get_key() {
            //     Some(key) => CounterState::LocalWithKey(key.clone()),
            //     None => CounterState::Local,
            // };
            return Ok(());
        };
        let addr = scratchpad.address();
        let scratchpad = client.scratchpad_get(addr).await?;
        self.counter = bincode::deserialize(&scratchpad.decrypt_data(&key)?)?;
        self.counter_state = CounterState::Connected {
            client: client.clone(),
            scratchpad,
            key: key.clone(),
        };
        Ok(())
    }

    // this interprets any error as inditive of not being connected hence false returned iseteaf of result
    // changes coutner state from connected to LocalWithKey if fails
    pub async fn is_connected(&mut self) -> bool {
        let mut connected = false;
        match &self.counter_state {
            CounterState::Connected {
                client,
                scratchpad,
                key,
            } => {
                connected = client
                    .scratchpad_check_existance(scratchpad.address())
                    .await
                    .unwrap_or(false);
                if connected == false {
                    self.counter_state = CounterState::LocalWithKey(key.clone());
                }
            }
            _ => (),
        }
        connected
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

async fn get_funded_wallet(private_key: &str) -> Result<Wallet> {
    let network = Network::new(true)?;
    let wallet = Wallet::new_from_private_key(network, private_key)?;
    println!("Wallet address: {}", wallet.address());
    println!("Wallet ballance: {}", wallet.balance_of_tokens().await?);
    Ok(wallet)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mean_test() {
        let mut last_six_values = LastSixValues::new();
        assert!(last_six_values.get_mean().is_nan());
        last_six_values.add(3);
        assert_eq!(last_six_values.get_mean(), 3.0);
        last_six_values.add(7);
        assert_eq!(last_six_values.get_mean(), 5.0);
        last_six_values.add(5);
        last_six_values.add(5);
        last_six_values.add(5);
        last_six_values.add(5);
        assert_eq!(last_six_values.get_mean(), 5.0);
        last_six_values.add(33);
        assert_eq!(last_six_values.get_mean(), 10.0);
    }

    #[test]
    fn last_value_test() {
        let mut last_six_values = LastSixValues::new();
        assert_eq!(last_six_values.get_last_value(), 0);
        last_six_values.add(3);
        assert_eq!(last_six_values.get_last_value(), 3);
        last_six_values.add(7);
        assert_eq!(last_six_values.get_last_value(), 7);
        for _ in 1..6 {
            last_six_values.add(5);
        }
        assert_eq!(last_six_values.get_last_value(), 5);
    }
}
