use counter::{ConnectionType, CounterApp, CounterState};
use eyre::Result;
use std::io::{self};
use std::path::Path;

mod counter;

#[tokio::main]
async fn main() -> Result<()> {
    run().await?;
    Ok(())
}

async fn run() -> Result<()> {
    let path = Path::new(""); // diretory path, file name givn in counter
                              // create app
    let mut counter_app = CounterApp::new()?;
    // get what type of connection to use
    loop {
        println!("Enter (a) to connect to antnet or (l) for a local network or (q) to quit:");
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        match input {
            "a" => {
                counter_app.connection_type = ConnectionType::Antnet;
                break;
            }
            "l" => {
                counter_app.connection_type = ConnectionType::Local;
                break;
            }
            "q" => {
                counter_app.counter_state = CounterState::Quitting;
                break;
            }
            _ => {
                println!("Unrecognised command");
                continue;
            }
        }
    }
    counter_app.set_path(&path);
    println!("{}", counter_app.get_counter_state());
    // let use choose to use existing coutner from key file or create a new one
    while let CounterState::Initiating = counter_app.counter_state {
        println!("Enter (u) to use existing counter, (c) to create a new one or (q) to quit:");
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        match input {
            "u" => {
                if let Ok(_) = counter_app.set_key_from_file() {
                    counter_app.connect(true).await?;
                } else {
                    println!(
                        "Failed to load key from path: {:?}",
                        &counter_app.key_file_path
                    );
                    continue;
                }
            }
            "c" => {
                if let ConnectionType::Antnet = counter_app.connection_type {
                    println!("Please enter private key:");
                    let mut input = String::new();
                    io::stdin().read_line(&mut input)?;
                    let private_key = input.trim();
                    counter_app.create(&private_key).await?
                } else {
                    let private_key =
                        "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
                    counter_app.create(&private_key).await?;
                }
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
            println!("Enter (i) to increment counter, (r) to reset, (rs) to reset statistics, (m) to set max, (d) to disconnect (testing), c to connect (testing) or q to quit:");
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
                "rs" => {
                    counter_app.counter.reset_stats();
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
                    if counter_app.get_counter_state() != "Connected" {
                        println!("Trying to connect to antnet...");
                        counter_app.connect(false).await?;
                        counter_app.print_scratchpad()?;
                    }
                }
                "q" => {
                    counter_app.counter_state = CounterState::Quitting;
                    break;
                }
                _ => {
                    println!("Unrecognised command");
                    continue;
                }
            }
            if !(counter_app.get_counter_state() == "Quitting") {
                match counter_app.counter.reset_if_next_period()? {
                    true => {
                        counter_app.sync_to_antnet().await?;
                    }
                    _ => (),
                }
            }
        }
    }
    println!("Final counter:");
    println!("{}", counter_app.counter);
    Ok(())
}
