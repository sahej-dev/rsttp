use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

use tracing::{error, info, instrument};

use crate::config::Config;
use crate::http::{Request, Response};
use crate::router::Router;

#[derive(Debug)]
pub struct RsttpServer<Ctx: Send + Sync + std::fmt::Debug + 'static> {
    pub config: Config<Ctx>,
    pub router: Router<Ctx>,
}

impl<Ctx: Send + Sync + std::fmt::Debug> RsttpServer<Ctx> {
    #[instrument]
    pub fn listen(self: Arc<Self>) {
        match TcpListener::bind(self.addr_as_string()) {
            Ok(listener) => {
                for stream in listener.incoming() {
                    let server: Arc<Self> = Arc::clone(&self);

                    match stream {
                        Ok(stream) => {
                            std::thread::spawn(move || {
                                server.tcp_event_handler(&stream, &server);
                            });
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
            Ok(_) => {}
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
    fn tcp_event_handler(&self, stream: &TcpStream, server: &RsttpServer<Ctx>) {
        info!("accepted new connection");
        let req: Request = match self.get_request_from_stream(stream) {
            Ok(req) => req,
            Err(_) => {
                Self::respond(stream, Response::bad_request());
                return;
            }
        };

        let response: Response = server.router.handle_request(req, &server.config.ctx);
        Self::respond(stream, response);
    }

    fn get_request_from_stream(&self, mut stream: &TcpStream) -> Result<Request, String> {
        let mut read_data: [u8; 8192] = [0; 8192];
        let bytes_read: usize = stream.read(&mut read_data).map_err(|e| e.to_string())?;

        let read_data: &str =
            std::str::from_utf8(&read_data[..bytes_read]).map_err(|e| e.to_string())?;

        Request::new(read_data).map_err(|e| e.to_string())
    }
}
