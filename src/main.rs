use anyhow::{self, Result};
use regex::Regex;
use std::{
    collections::HashMap,
    fs::File,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
    str, thread,
};
use thiserror::Error;

// HTTPのステータスに基づくエラー型の定義。
#[derive(Debug, Error)]
enum HTTPError {
    #[error("{0} Bad Request")]
    BadRequest(u16),
    #[error("{0} Not Found")]
    NotFound(u16),
}

// リクエストを表す構造体
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

// TCPコネクションのハンドラ
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

// エラーの種別に応じてステータスコードを選択し、レスポンスを返す
fn handle_error(stream: &mut TcpStream, e: anyhow::Error) -> Result<()> {
    match e.downcast_ref::<HTTPError>() {
        Some(HTTPError::BadRequest(e)) => {
            send_response(stream, &e.to_string(), "Bad Request", Vec::new())
        }
        Some(HTTPError::NotFound(e)) => {
            send_response(stream, &e.to_string(), "Not Found", Vec::new())
        }
        _ => send_response(stream, "500", "Internal Server Error", Vec::new()),
    }
}

// リクエストを読み込む
fn read_request(stream: &mut TcpStream) -> Result<Request> {
    let mut reader = BufReader::new(stream);
    let mut request = Request::new();
    let request_line_pattern = Regex::new(r"^(.*) (.*) (HTTP/1.[0|1])\r\n$")?;
    let header_pattern = Regex::new(r"^(.*): (.*)\r\n$")?;
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    match request_line_pattern.captures(&request_line) {
        Some(cap) => {
            request.method = cap[1].to_string();
            request.path = cap[2].to_string();
            request.version = cap[3].to_string();
        }
        None => {
            dbg!("request line");
            return Err(HTTPError::BadRequest(400).into());
        }
    }
    let mut header = String::new();
    while reader.read_line(&mut header).is_ok() {
        if header == "\r\n" {
            break;
        }
        match header_pattern.captures(&header) {
            Some(cap) => {
                request
                    .header
                    .insert(cap[1].to_string(), cap[2].to_string());
            }
            None => {
                dbg!("header");
                return Err(HTTPError::BadRequest(400).into());
            }
        }
        header = String::new();
    }
    if let Some(n) = request.header.get("Content-Length") {
        request.body = vec![0; n.parse()?];
        reader.read_exact(&mut request.body)?;
    }
    Ok(request)
}

// ローカルからファイルを読み込む
fn create_response_body(request: &Request) -> Result<Vec<u8>> {
    let path = match request.path.as_str() {
        "/" => "/index.html",
        _ => request.path.as_str(),
    };
    let file = match File::open(format!("./contents{}", path)) {
        Ok(f) => f,
        Err(_) => return Err(HTTPError::NotFound(404).into()),
    };
    let mut file_reader = BufReader::new(file);
    let mut resp_body = Vec::new();
    file_reader.read_to_end(&mut resp_body)?;
    Ok(resp_body)
}

// レスポンスを送信する
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
