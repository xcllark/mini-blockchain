use crate::server::Message;
use crate::utils::*;
use crate::Error;
use crate::Transaction;
use alloy_primitives::U256;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::signal::ctrl_c;

#[allow(unreachable_code)]
pub async fn run_loop() -> Result<(), Error> {
    (1..=3).for_each(|x| {
        tokio::spawn(async move {
            let mut nonce = 0;
            let pk = U256::from(x);
            let pk = u256_to_signing_key(&pk).unwrap();
            loop {
                println!("{}", nonce);
                let mut tx = Transaction::default();

                let address = addr(&pk);
                tx.from = address;
                tx.nonce = nonce;
                tx.value = 100;

                tx.hash = tx.hash();

                let (v, r, s) = sign_hash(tx.hash(), &pk);

                tx.v = v;
                tx.r = r;
                tx.s = s;

                let msg: Message = Message::Transaction(tx);
                let socket = TcpStream::connect("localhost:8545").await?;
                let mut connection = crate::server::Connection::new(socket);

                connection.write_message(&msg).await?;

                let msg = connection.read_message().await?;
                println!("{:?}", msg);

                nonce += 1;
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Ok::<(), Error>(())
        });
    });

    let _ = ctrl_c().await;
    Ok(())
}

pub async fn run() -> Result<(), Error> {
    let pk = U256::from(1);
    let pk = u256_to_signing_key(&pk).unwrap();

    let mut tx = Transaction::default();

    let addr = addr(&pk);
    tx.from = addr;
    tx.value = 100;
    tx.nonce = 0;

    tx.hash = tx.hash();

    let (v, r, s) = sign_hash(tx.hash(), &pk);

    tx.v = v;
    tx.r = r;
    tx.s = s;

    let msg: Message = Message::Transaction(tx);

    let socket = TcpStream::connect("localhost:8545").await?;
    let mut connection = crate::server::Connection::new(socket);

    connection.write_message(&msg).await?;

    let msg = connection.read_message().await?;

    println!("{:?}", msg);

    Ok(())
}

