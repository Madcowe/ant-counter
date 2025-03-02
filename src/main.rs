use autonomi::client::payment::PaymentOption;
use autonomi::client::scratchpad::{Bytes, Scratchpad};
use autonomi::{Client, Network, Wallet};
use counter::Counter;
use eyre::Result;
use std::io;

mod counter;

#[tokio::main]
async fn main() -> Result<()> {
    scratchpad_example().await?;
    Ok(())
}

async fn scratchpad_example() -> Result<()> {
    // initialize a local client and test wallet
    let client = Client::init_local().await?;
    let wallet = get_funded_wallet().await?;

    // create keys
    let key = autonomi::SecretKey::random();
    let public_key = key.public_key();

    // create counter
    let mut counter = Counter::new();
    counter.set_max(3);
    println!("{:?}", counter);

    // convert to bytes for scratchpad
    let counter_serailzed = bincode::serialize(&counter)?;
    let content = Bytes::from(counter_serailzed);
    let content_type = 99;

    // estimate the cost of the scratchpad
    // let cost = client.scratchpad_cost(&public_key).await?;
    // println!("scratchpad cost: {cost}");

    // create the scratchpad
    let payment_option = PaymentOption::from(&wallet);
    let (cost, addr) = client
        .scratchpad_create(&key, content_type, &content, payment_option)
        .await?;
    println!("scratchpad create cost: {cost}");

    //wait for the scratchpad to be replicated
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // check that the scrachpad is stored
    let got = client.scratchpad_get(&addr).await?;
    assert_eq!(*got.owner(), public_key);
    assert_eq!(got.data_encoding(), content_type);
    assert_eq!(got.decrypt_data(&key), Ok(content.clone()));
    assert_eq!(got.counter(), 0);
    assert!(got.verify_signature());
    println!(
        "scratchpad version {:?}, value: {:?}",
        got.counter(),
        got.decrypt_data(&key)
    );

    // check that the content is decrypted correctly
    let got_content = got.decrypt_data(&key)?;
    assert_eq!(got_content, content);

    // loop asking user for value to store and then storing on scratch pad
    loop {
        println!("Enter i to increment counter, r to rest or q to quit:");

        let mut input = String::new();

        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        match input {
            "i" => counter.increment(),
            "r" => counter.reset(),
            "q" => break,
            _ => println!("Unrecognised command"),
        }
        println!("{:?}", counter);
        // try to update scratchpad
        // convert to bytes for scratchpad
        let counter_serailzed = bincode::serialize(&counter)?;
        let content = Bytes::from(counter_serailzed);
        let content_type = 99;
        client
            .scratchpad_update(&key, content_type, &content)
            .await?;

        //wait for the scratchpad to be replicated
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        // check that the scrachpad is stored
        let got = client.scratchpad_get(&addr).await?;
        assert_eq!(*got.owner(), public_key);
        assert_eq!(got.data_encoding(), content_type);
        assert_eq!(got.decrypt_data(&key), Ok(content.clone()));
        assert!(got.verify_signature());
        println!(
            "scratchpad version {:?}, value: {:?}",
            got.counter(),
            got.decrypt_data(&key)?
        );

        // check that the content is decrypted correctly
        let got_content = got.decrypt_data(&key)?;
        assert_eq!(got_content, content);
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
