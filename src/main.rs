use autonomi::client::payment::PaymentOption;
use autonomi::client::scratchpad::Bytes;
use autonomi::{Client, Network, Scratchpad, SecretKey, Wallet};
use counter::Counter;
use eyre::Result;
use std::fs;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

mod counter;

#[tokio::main]
async fn main() -> Result<()> {
    scratchpad_counter().await?;
    Ok(())
}

async fn scratchpad_counter() -> Result<()> {
    // initialize a local client and test wallet
    let client = Client::init_local().await?;
    let wallet = get_funded_wallet().await?;
    let path = Path::new("key");
    // let key = create_key(path)?;

    // if file exists import key from file otherwise create it and save to file
    let key = match fs::read_to_string(path) {
        Ok(key) => SecretKey::from_hex(&key)?,
        Err(_) => create_key(path)?,
    };
    println!("{}", key.to_hex());

    let public_key = key.public_key();

    // if scratchpad already exists download data...and deserialize into a local Counter
    // populate local scratch pad variable with it
    // if not creat a new local counter then store in a new scratch pad
    // populate local scrach pad variable with it

    let mut counter;
    let scratchpad = match client.scratchpad_get_from_public_key(&public_key).await {
        Ok(scratchpad) => {
            counter = bincode::deserialize(&scratchpad.decrypt_data(&key)?)?;
            scratchpad
        }
        Err(_) => {
            counter = Counter::new();
            let counter_seralized = bincode::serialize(&counter)?;
            let content = Bytes::from(counter_seralized);
            let payment_option = PaymentOption::from(wallet);
            let content_type = 99;
            let (cost, addr) = client
                .scratchpad_create(&key, content_type, &content, payment_option)
                .await?;
            println!("scratchpad create cost: {cost} addr {addr}");
            //wait for the scratchpad to be replicated
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            client.scratchpad_get(&addr).await?
        }
    };

    println!("{:?}", counter);
    println!("{:?}", scratchpad);

    // loop asking user for value to store and then storing on scratch pad
    loop {
        println!("Enter i to increment counter, r to reset or q to quit:");

        let mut input = String::new();

        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        match input {
            "i" => {
                // download data again incase it has been changed by another app
                // counter = bincode::deserialize(&scratchpad.decrypt_data(&key)?)?;
                counter = get_scratchpad_counter(&client, &scratchpad, &key).await?;
                counter.increment();
            }
            "r" => {
                // download data again incase it has been changed by another app
                counter = get_scratchpad_counter(&client, &scratchpad, &key).await?;
                counter.reset();
            }
            "q" => break,
            _ => {
                println!("Unrecognised command");
                continue;
            }
        }
        println!("{:?}", counter);
        println!("Syncing to ant network...");
        let counter_serailzed = bincode::serialize(&counter)?;
        let content = Bytes::from(counter_serailzed);
        let content_type = 99;
        client
            .scratchpad_update(&key, content_type, &content)
            .await?;
        while counter != get_scratchpad_counter(&client, &scratchpad, &key).await? {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            println!("Syncing to ant network...");
        }
        // tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        println!("Synced");
        let got = client.scratchpad_get(&scratchpad.address()).await?;
        let decoded: Counter = bincode::deserialize(&got.decrypt_data(&key)?)?;
        println!(
            "scratchpad version {:?}, value: {:?}",
            got.counter(),
            decoded
        );
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

fn create_key(path: &Path) -> Result<autonomi::SecretKey> {
    let key = autonomi::SecretKey::random();
    let key_hex = key.to_hex();
    println!("New key: {}", key_hex);
    let mut file = File::create_new(&path)?;
    file.write_all(key_hex.as_bytes())?;
    Ok(key)
}

async fn get_scratchpad_counter(
    client: &Client,
    scratchpad: &Scratchpad,
    key: &autonomi::SecretKey,
) -> Result<Counter> {
    let counter: Counter = bincode::deserialize(
        &client
            .scratchpad_get(&scratchpad.address())
            .await?
            .decrypt_data(&key)?,
    )?;
    Ok(counter)
}
