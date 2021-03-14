use std::{
    error,
    io::{self, prelude::*},
    net, str, thread,
};

fn main() -> Result<(), Box<dyn error::Error>> {
    let listener = net::TcpListener::bind("127.0.0.1:50000")?;
    loop {
        let (stream, _) = listener.accept()?;
        thread::spawn(move || {
            handler(stream).unwrap();
        });
    }
}

fn handler(mut stream: net::TcpStream) -> Result<(), Box<dyn error::Error>> {
    println!("incoming connection from {}", stream.peer_addr()?);
    loop {
        let mut reader = io::BufReader::new(&stream);
        let mut buf = vec![];
        match reader.read_until(b'\n', &mut buf)? {
            0 => {
                println!("connection closed");
                return Ok(());
            }
            n => {
                stream.write_all(
                    b"HTTP/1.1 200 OK\r\n \
Server: sample\r\n \
Content-Length: 7\r\n \
Connection: Close\r\n \
Content-Type: text/plain;charset=utf8\r\n\r\n \
hello\r\n",
                )?;
            }
        }
    }
}
