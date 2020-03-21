// curl --socks5 localhost:1080 http://www.google.com
// curl --socks5 localhost:1080 128.32.236.14:80

#![warn(rust_2018_idioms)]

use tokio::prelude::*;
use tokio::io;
use tokio::net::{TcpListener, TcpStream};

use futures::future::try_join;
use futures::FutureExt;
use std::env;
use std::error::Error;

// use hex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let listen_addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:1080".to_string());

    println!("Listening on: {}", listen_addr);

    let mut listener = TcpListener::bind(listen_addr).await?;

    while let Ok((inbound, _)) = listener.accept().await {
        let handler = handle_socks5(inbound).map(|r| {
            if let Err(e) = r {
                println!("Failed to transfer; error={}", e);
            }
        });

        tokio::spawn(handler);
    }

    Ok(())
}

enum Socks5State{
    Init,
    Connect,
    Data,
}

async fn handle_socks5(mut inbound:TcpStream) -> Result<(), Box<dyn Error>>{
    let mut state = Socks5State::Init;
    let (mut ri, mut wi) = inbound.split();
    let mut b1: [u8;255] = [0; 255];

    loop{
        match state{
            Socks5State::Init => {
                let n = ri.peek(&mut b1).await?;
                if n != 4 {
                    continue;
                }
                let read_size = ri.read(&mut b1[..4]).await?;
                println!("read init: {}", read_size);
                // println!("{:#x?}", &mut b1[..n]);
                let response: [u8; 2] = [0x05, 0x00];
                wi.write(&response[..2]).await?;
                state = Socks5State::Connect;
            },
            Socks5State::Connect =>{
                let n = ri.peek(&mut b1).await?;
                if n != 10 {
                    continue;
                }
                let read_size = ri.read(&mut b1[..n]).await?;
                println!("read connect request: {}", read_size);
                println!("{:#x?}", &mut b1[..n]);
                // TODO: decode connect request
                state = Socks5State::Data;
            }
            Socks5State::Data =>{

            }
        }
    }
    Ok(())
}

async fn transfer(mut inbound: TcpStream, proxy_addr: String) -> Result<(), Box<dyn Error>> {
    let mut outbound = TcpStream::connect(proxy_addr).await?;

    let (mut ri, mut wi) = inbound.split();
    let (mut ro, mut wo) = outbound.split();

    let client_to_server = io::copy(&mut ri, &mut wo);
    let server_to_client = io::copy(&mut ro, &mut wi);

    try_join(client_to_server, server_to_client).await?;

    Ok(())
}