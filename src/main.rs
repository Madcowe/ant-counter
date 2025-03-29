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
                let wallet = get_funded_wallet().await?;
                counter_app.create(&path, &wallet).await?;
            }
            "q" => break,
            _ => {
                println!("Unrecognised command");
                continue;
            }
        }
        counter_app.print_counter_state();
    }
    println!("{:?}", counter_app.counter);
    match counter_app.counter_state {
        CounterState::Connected { ref scratchpad, .. } => println!("{:?}", scratchpad),
        _ => (),
    }
    // reset counter if needed
    match counter_app.counter.reset_if_next_period()? {
        true => counter_app.upload().await?,
        _ => (),
    }
    println!("{:?}", counter_app.counter);
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
            true => counter_app.upload().await?,
            _ => (),
        }
        // if not connected but have been connected update to local counter

        match input {
            "i" => {
                counter_app.counter.increment();
                counter_app.upload().await?;
                counter_app.print_scratchpad()?;
            }
            "r" => {
                counter_app.counter.reset();
                counter_app.upload().await?;
                counter_app.print_scratchpad()?;
            }
            "q" => break,
            _ => {
                println!("Inrecognised commad");
                continue;
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

async fn get_funded_wallet() -> Result<Wallet> {
    let private_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
    let network = Network::new(true)?;
    let wallet = Wallet::new_from_private_key(network, private_key)?;
    println!("Wallet address: {}", wallet.address());
    println!("Wallet ballance: {}", wallet.balance_of_tokens().await?);
    Ok(wallet)
}

// // loop asking user for value to store and then storing on scratch pad
// loop {
//     // get input from user
//     println!("Enter i to increment counter, r to reset or q to quit:");
//     let mut input = String::new();
//     io::stdin().read_line(&mut input)?;
//     let input = input.trim();

//     // sync counter with scratchpad and reset if needed
//     counter = get_scratchpad_counter(&client, &scratchpad, &key).await?;
//     match counter.reset_if_next_period()? {
//         true => {
//             // if counter is reset upload
//             scratchpad = update_scratchpad_counter(&client, &scratchpad, &counter, &key).await?
//         }
//         _ => (),
//     }
//     // println!("{:?}", counter);

//     match input {
//         "i" => {
//             // download data again incase it has been changed by another app
//             counter = get_scratchpad_counter(&client, &scratchpad, &key).await?;
//             counter.increment();
//         }
//         "r" => {
//             // download data again incase it has been changed by another app
//             counter = get_scratchpad_counter(&client, &scratchpad, &key).await?;
//             counter.reset();
//         }
//         "q" => break,
//         _ => {
//             println!("Unrecognised command");
//             continue;
//         }
//     }
//     // upload to antnet
//     scratchpad = update_scratchpad_counter(&client, &scratchpad, &counter, &key).await?;
//     // downlaod and print results
//     counter = bincode::deserialize(&scratchpad.decrypt_data(&key)?)?;
//     println!(
//         "scratchpad version {:?}, value: {:?}",
//         scratchpad.counter(),
//         counter
//     );
// }

// fn create_key(path: &Path) -> Result<autonomi::SecretKey> {
//     let key = autonomi::SecretKey::random();
//     let key_hex = key.to_hex();
//     println!("New key: {}", key_hex);
//     let mut file = File::create_new(&path)?;
//     file.write_all(key_hex.as_bytes())?;
//     Ok(key)
// }

// async fn get_scratchpad_counter(
//     client: &Client,
//     scratchpad: &Scratchpad,
//     key: &autonomi::SecretKey,
// ) -> Result<Counter> {
//     let counter: Counter = bincode::deserialize(
//         &client
//             .scratchpad_get(&scratchpad.address())
//             .await?
//             .decrypt_data(&key)?,
//     )?;
//     Ok(counter)
// }

// async fn update_scratchpad_counter(
//     client: &Client,
//     scratchpad: &Scratchpad,
//     counter: &Counter,
//     key: &autonomi::SecretKey,
// ) -> Result<Scratchpad> {
//     println!("{:?}", counter);
//     println!("Syncing to ant network...");
//     let counter_serailzed = bincode::serialize(&counter)?;
//     let content = Bytes::from(counter_serailzed);
//     let content_type = 99;
//     client
//         .scratchpad_update(&key, content_type, &content)
//         .await?;
//     while *counter != get_scratchpad_counter(&client, &scratchpad, &key).await? {
//         tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
//         println!("Syncing to ant network...");
//     }
//     println!("Synced");
//     Ok(client.scratchpad_get(scratchpad.address()).await?)
// }
