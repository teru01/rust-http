use anyhow::Result;
use std::{
    collections::HashMap,
    error,
    io::{self, prelude::*},
    net, str, thread,
};

fn main() -> Result<()> {
    let listener = net::TcpListener::bind("127.0.0.1:50000")?;
    loop {
        let (stream, _) = listener.accept()?;
        thread::spawn(move || {
            handler(stream).unwrap();
        });
    }
}

struct Request {
    version: String,
    path: String,
    method: String,
    header: HashMap<String, String>,
    body: Vec<u8>,
}

impl Request {
    fn new() -> Self {
        Self {
            version: String::new(),
            path: String::new(),
            method: String::new(),
            header: HashMap::new(),
            body: Vec::new(),
        }
    }
}

fn handler(mut stream: net::TcpStream) -> Result<()> {
    println!("incoming connection from {}", stream.peer_addr()?);
    let mut reader = io::BufReader::new(&stream);
    let mut buf = Vec::new();
    let mut is_first_line = true;
    let mut request = Request::new();
    while let Ok(n) = reader.read_until(b'\n', &mut buf) {
        match n {
            0 => {
                println!("connection closed");
                return Ok(());
            }
            n => {
                if is_first_line {
                    let rline: Vec<&str> = str::from_utf8(&buf[0..n - 2])?.split(' ').collect();
                    request.method = rline[0].to_string();
                    request.path = rline[1].to_string();
                    request.version = rline[2].to_string();
                    is_first_line = false;
                } else {
                    if n == 2 && buf[0] == b'\r' && buf[1] == b'\n' {
                        break;
                    }
                    let header: Vec<&str> = str::from_utf8(&buf[0..n - 2])?
                        .split(": ")
                        .map(|s| s.trim())
                        .collect();
                    request
                        .header
                        .insert(header[0].to_string(), header[1].to_string());
                }
            }
        }
        buf = Vec::new();
    }
    if let Some(n) = request.header.get("Content-Length") {
        let mut buf = vec![0; n.parse()?];
        reader.read_exact(&mut buf)?;
        request.body = buf;
    }

    let response_body = &request.body;
    let mut response = Vec::new();
    response.push("HTTP/1.1 200 OK");
    let length = format!("Content-Length: {}", response_body.len());
    response.push(length.as_str());
    let resp_byte = [
        format!("{}{}", response.join("\r\n"), "\r\n\r\n").as_bytes(),
        &response_body,
    ]
    .concat();
    stream.write_all(&resp_byte)?;
    Ok(())
}
