use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

pub mod listener;

pub use listener::*;

pub struct RsttpServer {
    pub addr: String,
    pub files_dir: Arc<String>,
    pub handler: fn(TcpStream, &String),
}

impl RsttpServer {
    pub fn listen(&self) {
        match TcpListener::bind(&self.addr) {
            Ok(listener) => {
                for stream in listener.incoming() {
                    match stream {
                        Ok(stream) => {
                            let files_dir: Arc<String> = Arc::clone(&self.files_dir);
                            let handler: fn(TcpStream, &String) = self.handler;

                            std::thread::spawn(move || (handler)(stream, &files_dir));
                            // handle_connecttion(stream);
                        }
                        Err(e) => {
                            println!("error: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                println!("Failed to establish a TCP Listener. Error:\n{}", e);
            }
        }
    }

    pub fn addr_as_string(&self) -> String {
        String::from(&self.addr)
    }
}
