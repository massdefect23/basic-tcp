use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string_pretty};
use std::env;
use std::fs::{OpenOptions};
use std::io::{Read, Write};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Debug, Serialize, Deserialize)]
struct JsonData {
    integers: Vec<i32>,
}

async fn handle_client(mut stream: TcpStream) {
    let mut buffer = [0; 1024];
    let n = stream.read(&mut buffer).await.unwrap();
    let integer: i32 = String::from_utf8_lossy(&buffer[..n]).trim().parse().unwrap();

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open("data.json")
        .unwrap();

    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    let mut data: JsonData = if contents.is_empty() {
        JsonData { integers: vec![] }
    } else {
        from_str(&contents).unwrap()
    };

    data.integers.push(integer);

    let json = to_string_pretty(&data).unwrap();
    file.set_len(0).unwrap();
    file.write_all(json.as_bytes()).unwrap();

    stream.write_all(b"Integer added to the JSON file.\n").await.unwrap();
}

async fn server_mode(addr: SocketAddr) {
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Server listening on {}", addr);

    loop {
        let (stream, _) = listener.accept().await.unwrap();
        tokio::spawn(async move {
            handle_client(stream).await;
        });
    }
}

async fn client_mode(addr: SocketAddr, integer: i32) {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let data = integer.to_string();
    stream.write_all(data.as_bytes()).await.unwrap();

    let mut buffer = [0; 1024];
    stream.read(&mut buffer).await.unwrap();
    println!("{}", String::from_utf8_lossy(&buffer).trim());
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Usage: {} <mode(server|client)> <address> [integer]", args[0]);
        return;
    }

    let mode = &args[1];
    let addr: SocketAddr = args[2].parse().unwrap();

    match mode.as_str() {
        "server" => server_mode(addr).await,
        "client" => {
            if args.len() < 4 {
                println!("Usage: {} client <address> <integer>", args[0]);
                return;
            }
            let integer: i32 = args[3].parse().unwrap();
            client_mode(addr, integer).await;
        }
        _ => println!("Invalid mode. Use 'Server or Client'"),
    }
}
