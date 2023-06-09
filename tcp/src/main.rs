use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string_pretty};
use std::env;
use std::fs::{OpenOptions};
use std::io::{Read, Write, SeekFrom};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::error::Error;
use std::io::Seek;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{Sender};


#[derive(Clone)]
struct Client {
    sender: Sender<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonData {
    integers: Vec<i32>,
}


async fn handle_client(mut stream: TcpStream, clients: Arc<Mutex<Vec<Client>>>,) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = [0; 1024];
    let n = stream.read(&mut buffer).await?;
    let integer: i32 = String::from_utf8_lossy(&buffer[..n]).trim().parse()?;

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open("data.json")?;

    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    // Reset the cursor to the beginning of th efile
    file.seek(SeekFrom::Start(0))?;

    let mut data: JsonData = if contents.is_empty() {
        JsonData { integers: vec![] }
    } else {
        from_str(&contents)?
    };

    data.integers.push(integer);

    let json = to_string_pretty(&data)?;
    file.seek(SeekFrom::Start(0))?;
    file.set_len(0)?;
    file.write_all(json.as_bytes())?;

    // Send the updated JSOn back to client
    let message = format!("Updated JSON data: {}", json);
    broadcast(&message, &clients).await;
    Ok(())
}


async fn server_mode(addr: SocketAddr) -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind(addr).await?;
    let clients: Arc<Mutex<Vec<Client>>> = Arc::new(Mutex::new(Vec::new()));

    println!("Server listening on {}", addr);

    loop {
        let (stream, _) = listener.accept().await?;
        let clients_clone = clients.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_client(stream, clients_clone).await {
                eprintln!("Error handling client: {}", e);
            }
        });
    }
}

async fn client_mode(addr: SocketAddr, integer: i32) -> Result<(), Box<dyn Error>> {
    let mut stream = TcpStream::connect(addr).await?;

    // Send the integer to the server
    let data = integer.to_string();
    stream.write_all(data.as_bytes()).await?;

    // Continuously listen for incoming messages from the server
    let mut buffer = [0; 4096];
    loop {
        let n = stream.read(&mut buffer).await?;
        if n == 0 {
            // Server has closed connection
            break;
        }
        let message = String:: from_utf8_lossy(&buffer[..n]);
        println!("Received: {}", message);

        // Save the Updated json to a file in the clients directory
        let updated_json = message.trim_start_matches("Updated JSON data: ");
        let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open("client_data.json")?;
    file.write_all(updated_json.as_bytes())?;
    }
    Ok(())
}

async fn broadcast(message: &str, clients: &Arc<Mutex<Vec<Client>>>) {
    let locked_clients = clients.lock().await;
    for client in locked_clients.iter() {
        if let Err(e) = client.sender.send(message.to_string()).await {
            eprintln!("Error sending message: {}", e);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Usage: {} <mode(server|client)> <address> [integer]", args[0]);
        return Ok(());
    }

    let mode = &args[1];
    let addr: SocketAddr = args[2].parse()?;

    match mode.as_str() {
        "server" => server_mode(addr).await?,
        "client" => {
            if args.len() < 4 {
                println!("Usage: {} client <address> <integer>", args[0]);
                return Ok(());
            }
            let integer: i32 = args[3].parse()?;
            client_mode(addr, integer).await?;
        }
        _ => println!("Invalid mode. Use 'server' or 'client'"),
    }

    Ok(())
}
