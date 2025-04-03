use autonomi::{Client, ClientConfig, Network, Scratchpad, SecretKey, Wallet};
use counter::{Counter, CounterApp, CounterState};
use eyre::Result;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

mod counter;

#[tokio::main]
async fn main() -> Result<()> {
    run().await?;
    Ok(())
}

async fn run() -> Result<()> {
    let path = Path::new("key");
    let private_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
    // create app
    let mut counter_app = CounterApp::new()?;
    // get input from user
    counter_app.print_counter_state();
    while let CounterState::Initiating = counter_app.counter_state {
        println!("Enter (u) to use existing counter, (c) to create a new one or (q) to quit:");
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        match input {
            "u" => {
                if let Ok(hex_key) = fs::read_to_string(path) {
                    counter_app.set_key_from_hex(&hex_key)?;
                    counter_app.connect().await?;
                } else {
                    println!("Failed to load key file at path: {path:?}");
                }
            }
            "c" => {
                counter_app.create(&path, &private_key).await?;
            }
            "q" => counter_app.counter_state = CounterState::Quiting,
            _ => {
                println!("Unrecognised command");
                continue;
            }
        }
        counter_app.print_counter_state();
    }

    if !(CounterState::Quiting == counter_app.counter_state) {
        println!("{:?}", counter_app.counter);
        if counter_app.is_connected().await {
            counter_app.download().await?;
            counter_app.print_scratchpad()?;
        }
        match counter_app.counter.reset_if_next_period()? {
            true => {
                counter_app.upload().await?;
                println!("{:?}", counter_app.counter);
            }
            _ => (),
        }

        // loop asking user for value to store and then storing on scratch pad
        loop {
            // get input from user
            println!("Enter i to increment counter, r to reset or q to quit:");
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();
            // if connected get counter from antnet
            if counter_app.is_connected().await {
                // need to implement
                // if not connected this app instance but now can get counter and add local counter to it
                counter_app.download().await?;
            }
            // reset counter if needed
            match counter_app.counter.reset_if_next_period()? {
                true => {
                    counter_app.upload().await?;
                    counter_app.download().await?; // so local scratchpad synced
                    counter_app.print_scratchpad()?;
                }
                _ => (),
            }
            // if not connected but have been connected update to local counter

            match input {
                "i" => {
                    counter_app.counter.increment();
                    counter_app.upload().await?;
                    counter_app.download().await?; // so local scratchpad synced
                    counter_app.print_scratchpad()?;
                }
                "r" => {
                    counter_app.counter.reset();
                    counter_app.upload().await?;
                    counter_app.download().await?; // so local scratchpad synced
                    counter_app.print_scratchpad()?;
                }
                "q" => break,
                _ => {
                    println!("Inrecognised command");
                    continue;
                }
            }
        }
    }
    // download and print results
    println!("Final state:");
    counter_app.print_counter_state();
    if counter_app.is_connected().await {
        counter_app.download().await?;
        counter_app.print_scratchpad()?;
    }
    Ok(())
}
