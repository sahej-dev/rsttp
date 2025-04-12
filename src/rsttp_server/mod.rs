use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

// mod listener;

use crate::config::Config;
use crate::http::{Request, Response};

pub struct RsttpServer {
    pub config: Config,
    pub handler: fn(Request, &String) -> Response,
}

impl RsttpServer {
    pub fn listen(self: Arc<Self>) {
        match TcpListener::bind(self.addr_as_string()) {
            Ok(listener) => {
                for stream in listener.incoming() {
                    let server: Arc<RsttpServer> = Arc::clone(&self);

                    match stream {
                        Ok(stream) => {
                            std::thread::spawn(move || {
                                server.tcp_event_handler(&stream, &server);
                            });
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
        self.config.addr()
    }

    fn respond(stream: &TcpStream, response: Response) {
        match response.write_to(stream) {
            Ok(_) => {}
            Err(e) => match e.kind() {
                std::io::ErrorKind::BrokenPipe => {
                    println!("Client disconnected during response");
                }
                std::io::ErrorKind::ConnectionReset => {
                    println!("Connection reset by client");
                }
                _ => {
                    eprintln!("ERROR: Failed to write response: {}", e);
                }
            },
        }
    }

    fn tcp_event_handler(&self, stream: &TcpStream, server: &RsttpServer) {
        println!("accepted new connection");
        let req = match self.get_request_from_stream(stream) {
            Ok(req) => req,
            Err(_) => {
                RsttpServer::respond(stream, Response::bad_request());
                return;
            }
        };

        let response = (self.handler)(req, &server.config.static_files_dir);
        RsttpServer::respond(stream, response);
    }

    fn get_request_from_stream(&self, mut stream: &TcpStream) -> Result<Request, String> {
        let mut read_data: [u8; 8192] = [0; 8192];
        let bytes_read = stream.read(&mut read_data).map_err(|e| e.to_string())?;

        let read_data: &str =
            std::str::from_utf8(&read_data[..bytes_read]).map_err(|e| e.to_string())?;

        Request::new(read_data).map_err(|e| e.to_string())
    }
}
