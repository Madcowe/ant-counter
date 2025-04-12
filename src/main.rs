use counter::{CounterApp, CounterState};
use eyre::Result;
use std::fs;
use std::io::{self};
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
    println!("{}", counter_app.get_counter_state());
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
            "q" => counter_app.counter_state = CounterState::Quitting,
            _ => {
                println!("Unrecognised command");
                continue;
            }
        }
        println!("{}", counter_app.get_counter_state());
    }

    if !(CounterState::Quitting == counter_app.counter_state) {
        println!("{}", counter_app.counter);
        if counter_app.is_connected().await {
            counter_app.download().await?;
            counter_app.print_scratchpad()?;
        }
        match counter_app.counter.reset_if_next_period()? {
            true => {
                if counter_app.is_connected().await {
                    counter_app.upload().await?;
                }
                println!("{}", counter_app.counter);
            }
            _ => (),
        }

        // loop asking user for value to store and then storing on scratch pad
        loop {
            println!("{}", counter_app.get_counter_state());
            // get input from user
            println!("Enter (i) to increment counter, (r) to reset, (m) to set max, (d) to disconnect (testing), c to connect (testing) or q to quit:");
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();
            // if connected get counter from antnet
            if counter_app.get_counter_state() == "Connected" {
                counter_app.download().await?;
            }
            match input {
                "i" => {
                    counter_app.increment();
                    counter_app.sync_to_antnet().await?;
                }
                "r" => {
                    counter_app.reset();
                    counter_app.sync_to_antnet().await?;
                }
                "m" => {
                    println!("Enter the max for a period: ");
                    let mut input = String::new();
                    io::stdin().read_line(&mut input)?;
                    let input: usize = match input.trim().parse() {
                        Ok(input) => input,
                        Err(_) => {
                            println!("Max must be a positive whole number");
                            continue;
                        }
                    };
                    counter_app.counter.set_max(input);
                    counter_app.sync_to_antnet().await?;
                }
                "d" => {
                    counter_app.disconnect();
                    println!("{}", counter_app.counter);
                }
                "c" => {
                    // if not connected attempt to connect
                    if counter_app.is_connected().await == false {
                        println!("Trying to connect to antnet...");
                        counter_app.connect().await?;
                        counter_app.print_scratchpad()?;
                    }
                }
                "q" => break,
                _ => {
                    println!("Unrecognised command");
                    continue;
                }
            }
            // // if not connected attempt to connect
            // if counter_app.is_connected().await == false {
            //     println!("Trying to connect to antnet...");
            //     counter_app.connect().await?;
            // }
            // reset counter if needed
            match counter_app.counter.reset_if_next_period()? {
                true => {
                    counter_app.sync_to_antnet().await?;
                }
                _ => (),
            }
        }
    }
    // download and print results
    println!("Final state:");
    println!("{}", counter_app.get_counter_state());
    if counter_app.is_connected().await {
        counter_app.download().await?;
        counter_app.print_scratchpad()?;
    }
    Ok(())
}
