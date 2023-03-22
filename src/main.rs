use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use redis_starter_rust::resp::RespArray;
use tokio::{net::{TcpListener, TcpStream}, spawn, io::{AsyncWriteExt, BufReader, AsyncBufReadExt}, sync::Mutex};

#[tokio::main]
async fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6379").await?;
    let mut data: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));

    loop {
        let (socket, _addr) = listener.accept().await?;
        let data = data.clone();

        spawn(async move {
            if let Err(e) = handle(socket, &data).await {
                eprintln!("Dropping client due to error: `{e}`");
            }
        }); 
    }
}

async fn handle(socket: TcpStream, mutex: &Arc<Mutex<HashMap<String, String>>>) -> Result<()> {
    let (rx, mut tx) = socket.into_split();

    let mut rx = BufReader::new(rx);
    let mut buf = String::new();
    let mut resp = RespArray {
        size: 0,
        data: Vec::new()
    };

    loop {
        let bytes_read = rx
            .read_line(&mut buf)
            .await?;

        if bytes_read == 0 {
            eprintln!("closing connection");

            return Ok(());
        }

        if buf.trim().contains("*") {
            let resp_size = buf.trim().chars().nth(1).expect("cannot parse *");
            let resp_size = resp_size.to_digit(10).expect("cannot parse to digit");
            resp.set_array_size(resp_size as usize * 2);
        } else {
            resp.add_to_array(buf.trim().to_string());
            println!("{:?}", resp);
        }

        if resp.data.len() == resp.size {
            match resp.data.iter().nth(1).expect("No data").to_lowercase().as_str() {
                "command" => {
                    resp.data = Vec::new();
                    tx.write_all("+CONNECTED\r\n".as_bytes())
                        .await?;
                },
                "ping" => {
                    resp.data = Vec::new();
                    tx.write_all("+PONG\r\n".as_bytes())
                        .await?;
                },
                "echo" => {
                    let data: String = resp.data.iter().skip(3).cloned().map(|mut x| {
                        if x.contains("$") {
                            x = " ".to_string();
                        }
                        x
                    }).collect();
                    resp.data = Vec::new();
                    tx.write_all(format!("+{}\r\n", data).as_bytes())
                        .await?;
                },
                "set" => {
                    let key = resp.data.iter().skip(3).nth(0).cloned().unwrap();
                    let val = resp.data.iter().skip(5).nth(0).cloned().unwrap();
                    resp.data = Vec::new();
                    
                    let mut db = mutex.lock().await;
                    db.insert(key, val);

                    tx.write_all("+OK\r\n".as_bytes())
                        .await?;
                },
                "get" => {
                    let key = resp.data.iter().skip(3).nth(0).cloned().unwrap();
                    resp.data = Vec::new();

                    let db = mutex.lock().await;
                    match db.get(&key) {
                        Some(val) => {
                            tx.write_all(format!("+{}\r\n", val).as_bytes())
                                .await?;
                        },
                        None => {
                            tx.write_all("-Key not found\r\n".as_bytes())
                                .await?;
                        }
                    }
                },
                _inp => {
                    resp.data = Vec::new();
                    tx.write_all("-Command not found\r\n".as_bytes())
                        .await?;
                }
            }
        }
        buf.clear();
    }
}
