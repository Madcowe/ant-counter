use autonomi::client::payment::PaymentOption;
use autonomi::client::scratchpad::Bytes;
use autonomi::self_encryption::Error;
use autonomi::{Client, ClientConfig, Network, Scratchpad, SecretKey, Wallet};
use counter::{Counter, CounterApp};
use eyre::Result;
use std::fs;
use std::fs::File;
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
    loop {
        println!("Enter (u) to use existing counter, (c) to create a new one or (q) to quit:");
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        match input {
            //         "u" => {
            //             if path.try_exists().unwrap_or(false) {
            //                 counter_app.connected_scratchpad
            //                 counter_app.connect();
            //             } else {
            //                 println!("Key file is missing at path: {path:?}");
            //             }
            //         }
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
    }
    Ok(())
}
//     "u" -> {

// let path = Path::new("key");
// // if path file already exist connect using that otherwise create new key
// if path.try_exists().unwrap_or(false) {
// }
// }
// }
// // // initialize a local client and test wallet
// // let client = Client::init_local().await?;
// } else {
//     let wallet = get_funded_wallet().await?;
//     counter_app.create(path, wallet).await?;
// }

// // if file exists import key from file otherwise create it and save to file
// let key = match fs::read_to_string(path) {
//     Ok(key) => SecretKey::from_hex(&key)?,
//     Err(_) => create_key(path)?,
// };
// println!("{}", key.to_hex());

// let public_key = key.public_key();

// // if scratchpad already exists download data...and deserialize into a local Counter
// // populate local scratch pad variable with it
// // if not creat a new local counter then store in a new scratch pad
// // populate local scrach pad variable with it

// let mut counter;
// let mut scratchpad = match client.scratchpad_get_from_public_key(&public_key).await {
//     Ok(scratchpad) => {
//         counter = bincode::deserialize(&scratchpad.decrypt_data(&key)?)?;
//         scratchpad
//     }
//     Err(_) => {
//         counter = Counter::new()?;
//         let counter_seralized = bincode::serialize(&counter)?;
//         let content = Bytes::from(counter_seralized);
//         let payment_option = PaymentOption::from(wallet);
//         let content_type = 99;
//         let (cost, addr) = client
//             .scratchpad_create(&key, content_type, &content, payment_option)
//             .await?;
//         println!("scratchpad create cost: {cost} addr {addr}");
//         //wait for the scratchpad to be replicated
//         tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
//         client.scratchpad_get(&addr).await?
//     }
// };

// println!("{:?}", counter);
// println!("{:?}", scratchpad);
// match counter.reset_if_next_period()? {
//     true => {
//         scratchpad = update_scratchpad_counter(&client, &scratchpad, &counter, &key).await?
//     }
//     _ => (),
// }
// println!("{:?}", counter);

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

async fn get_funded_wallet() -> Result<Wallet> {
    let private_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
    let network = Network::new(true)?;
    let wallet = Wallet::new_from_private_key(network, private_key)?;
    println!("Wallet address: {}", wallet.address());
    println!("Wallet ballance: {}", wallet.balance_of_tokens().await?);
    Ok(wallet)
}

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
