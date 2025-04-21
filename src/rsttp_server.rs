use std::collections::HashMap;
use std::io::Read;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use tracing::{error, info, instrument};

use crate::config::Config;
use crate::http::{Request, Response};
use crate::router::Router;
use crate::thread_pool::ThreadPool;

#[derive(Debug)]
pub struct RsttpServer<Ctx: Send + Sync + std::fmt::Debug + 'static> {
    pub config: Config<Ctx>,
    pub router: Router<Ctx>,
    thread_pool: ThreadPool,
    peer_connections: Mutex<HashMap<SocketAddr, TcpStream>>,
}

impl<Ctx: Send + Sync + std::fmt::Debug> RsttpServer<Ctx> {
    pub fn new(config: Config<Ctx>, router: Router<Ctx>, thread_count: usize) -> RsttpServer<Ctx> {
        RsttpServer {
            config,
            router,
            thread_pool: ThreadPool::new(thread_count),
            peer_connections: Mutex::new(HashMap::new()),
        }
    }

    #[instrument]
    pub fn listen(self: Arc<Self>) {
        match TcpListener::bind(self.addr_as_string()) {
            Ok(listener) => {
                for stream in listener.incoming() {
                    let server: Arc<Self> = Arc::clone(&self);

                    match stream {
                        Ok(stream) => {
                            println!("stream: {:?}", stream);

                            if let Ok(addr) = stream.peer_addr() {
                                println!("socket: {}", addr);

                                if let Ok(mut connections) = self.peer_connections.lock() {
                                    println!("got connections lock: {}", addr);
                                    if connections.get(&addr).is_none() {
                                        connections.insert(addr, stream);
                                    }

                                    self.thread_pool.execute(move || {
                                        server.tcp_event_handler(addr, &server);
                                    });
                                }
                            }
                        }
                        Err(e) => {
                            error!(error = e.to_string());
                        }
                    }
                }
            }
            Err(e) => {
                error!(error = e.to_string());
            }
        }
    }

    pub fn addr_as_string(&self) -> String {
        self.config.addr()
    }

    #[instrument]
    fn respond(stream: &TcpStream, response: Response) {
        match response.write_to(stream) {
            Ok(_) => {
                println!("Written to stream");
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::BrokenPipe => {
                    error!("Client disconnected during response");
                }
                std::io::ErrorKind::ConnectionReset => {
                    error!("Connection reset by client");
                }
                _ => {
                    error!(error = e.to_string(), "ERROR: Failed to write response");
                }
            },
        }
    }

    #[instrument]
    fn tcp_event_handler(&self, socket_addr: SocketAddr, server: &RsttpServer<Ctx>) {
        println!("handling for {}", socket_addr);

        let stream = if let Ok(mut connections) = self.peer_connections.lock() {
            match connections.get(&socket_addr) {
                Some(stream) => {
                    if let Err(e) =
                        stream.set_read_timeout(Some(server.config.persist_connection_for))
                    {
                        error!(error = e.to_string(), "Failed to set read timeout");
                        connections.remove(&socket_addr);
                        return;
                    }

                    stream.try_clone().ok()
                }
                None => None,
            }
        } else {
            error!("Could not obtain lock on peer connections");
            return;
        };

        let stream = match stream {
            Some(s) => s,
            None => {
                error!("Could not extract stream handle");
                return;
            }
        };

        let mut keep_alive: bool = true;

        while keep_alive {
            let req = match self.get_request_from_stream(&stream) {
                Ok(req) => req,
                Err(e) => {
                    if e != "Connection closed by client" {
                        println!("bad request at error: {}", e);
                        Self::respond(&stream, Response::bad_request());
                    }
                    break;
                }
            };

            keep_alive = !req.has_connection_close_header();

            let response: Response = server.router.handle_request(req, &server.config.ctx);

            Self::respond(&stream, response);
        }

        if let Ok(mut connections) = self.peer_connections.lock() {
            connections.remove(&socket_addr);
        }
    }

    fn get_request_from_stream(&self, mut stream: &TcpStream) -> Result<Request, String> {
        let mut read_data: [u8; 8192] = [0; 8192];
        let bytes_read: usize = match stream.read(&mut read_data) {
            Ok(0) => return Err(String::from("Connection closed by client")),
            Ok(n) => n,
            Err(e) => match e.kind() {
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut => {
                    return Err(String::from("Connection timed out"));
                }
                _ => return Err(format!("IO error: {}", e)),
            },
        };

        let read_data: &str =
            std::str::from_utf8(&read_data[..bytes_read]).map_err(|e| e.to_string())?;

        println!("read data\n\n{}\n\n", read_data);

        Request::new(read_data).map_err(|e| e.to_string())
    }
}
