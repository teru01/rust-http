use anyhow::{self, Result};
use std::{
    collections::HashMap,
    fs::File,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
    str, thread,
};
use thiserror::Error;

#[derive(Debug, Error)]
enum HTTPError {
    #[error("{0} Bad Request")]
    BadRequest(u16),
    #[error("{0} Not Found")]
    NotFound(u16),
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

fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:50000")?;
    loop {
        let (mut stream, _) = listener.accept()?;
        thread::spawn(move || {
            handler(&mut stream).unwrap();
        });
    }
}

fn handler(stream: &mut TcpStream) -> Result<()> {
    println!("incoming connection from {}", stream.peer_addr()?);
    let request = match read_request(stream) {
        Ok(r) => r,
        Err(e) => {
            handle_error(stream, e)?;
            return Ok(());
        }
    };
    let response = match create_response_body(&request) {
        Ok(r) => r,
        Err(e) => {
            handle_error(stream, e)?;
            return Ok(());
        }
    };
    send_response(stream, "200", "OK", response)?;
    Ok(())
}

fn handle_error(stream: &mut TcpStream, e: anyhow::Error) -> Result<()> {
    match e.downcast_ref::<HTTPError>() {
        Some(HTTPError::BadRequest(_)) => send_response(stream, "400", "Bad Request", Vec::new()),
        Some(HTTPError::NotFound(_)) => send_response(stream, "404", "Not Found", Vec::new()),
        _ => send_response(stream, "500", "Internal Server Error", Vec::new()),
    }
}

fn read_request(stream: &mut TcpStream) -> Result<Request> {
    let mut reader = BufReader::new(stream);
    let mut buf = Vec::new();
    let mut is_first_line = true;
    let mut request = Request::new();
    while let Ok(n) = reader.read_until(b'\n', &mut buf) {
        match n {
            2 => {
                if !is_first_line && buf[0] == b'\r' && buf[1] == b'\n' {
                    break;
                }
            }
            n => {
                if is_first_line {
                    let rline: Vec<&str> = str::from_utf8(&buf[0..n - 2])?.split(' ').collect();
                    if rline.len() != 3 || !rline[2].starts_with("HTTP") {
                        Err(HTTPError::BadRequest(400))?
                    }
                    request.method = rline[0].to_string();
                    request.path = rline[1].to_string();
                    request.version = rline[2].to_string();
                    is_first_line = false;
                } else {
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
        request.body = vec![0; n.parse()?];
        reader.read_exact(&mut request.body)?;
    }
    Ok(request)
}

fn create_response_body(request: &Request) -> Result<Vec<u8>> {
    let path = match request.path.as_str() {
        "/" => "/index.html",
        _ => request.path.as_str(),
    };
    let file = match File::open(format!("./contents{}", path)) {
        Ok(f) => f,
        Err(_) => Err(HTTPError::NotFound(404))?,
    };
    let mut file_reader = BufReader::new(file);
    let mut resp_body = Vec::new();
    file_reader.read_to_end(&mut resp_body)?;
    Ok(resp_body)
}

fn send_response(
    stream: &mut TcpStream,
    status_code: &str,
    message: &str,
    response_body: Vec<u8>,
) -> Result<()> {
    let mut response = Vec::new();
    response.push(format!("HTTP/1.1 {} {}", status_code, message));
    response.push(format!("Content-Length: {}", response_body.len()));
    let resp_byte = [
        format!("{}{}", response.join("\r\n"), "\r\n\r\n").as_bytes(),
        &response_body,
    ]
    .concat();
    stream.write_all(&resp_byte)?;
    Ok(())
}
