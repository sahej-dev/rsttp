use std::io::Read;
use std::net::TcpStream;
use std::sync::Arc;
use std::{env, fs};

use http::{ContentType, HttpResponseCode, ReqType, Request, Response};
use rsttp_server::RsttpServer;

mod config;
mod http;
mod rsttp_server;

fn respond_success(stream: TcpStream, req: &Request) {
    respond_with_default_msg(stream, req, HttpResponseCode::R200);
}

fn respond(stream: TcpStream, response: Response) {
    match response.write_to(&stream) {
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

fn respond_with_default_msg(stream: TcpStream, req: &Request, code: HttpResponseCode) {
    respond(
        stream,
        Response::new(req, code, None, ContentType::TextPlain, &req.protocol),
    );
}

fn respond_bad_request(stream: TcpStream, req: &Request) {
    respond(
        stream,
        Response::new(
            req,
            HttpResponseCode::R400,
            None,
            ContentType::TextPlain,
            &req.protocol,
        ),
    );
}

fn handle_connection(mut stream: TcpStream, files_dir: &String) {
    println!("accepted new connection");
    let mut read_data: [u8; 8192] = [0; 8192];
    let bytes_read = match stream.read(&mut read_data) {
        Ok(n) => n,
        Err(_) => {
            println!("EOROROR");
            return respond(stream, Response::empty_http11(HttpResponseCode::R400));
        }
    };

    let read_data: &str = match std::str::from_utf8(&read_data[..bytes_read]) {
        Ok(data) => data,
        Err(_) => {
            println!("EOROROR");
            return respond(stream, Response::empty_http11(HttpResponseCode::R400));
        }
    };

    println!("read {} bytes", bytes_read);
    println!("data:\n{}", read_data);

    match Request::new(read_data) {
        Err(_) => respond(stream, Response::empty_http11(HttpResponseCode::R404)),
        Ok(req) => {
            println!("Request: {:?}", req);

            if req.path == "/" {
                respond_success(stream, &req);
            } else if req.path.starts_with("/echo/") {
                if req.path == "/echo/" {
                    respond_bad_request(stream, &req);
                } else {
                    respond(
                        stream,
                        Response::new(
                            &req,
                            HttpResponseCode::R200,
                            Some(String::from(req.path.split_at(6).1)),
                            ContentType::TextPlain,
                            &req.protocol,
                        ),
                    );
                }
            } else if req.path == "/user-agent" {
                match req.header_val("User-Agent") {
                    Some(header_val) => respond(
                        stream,
                        Response::new(
                            &req,
                            HttpResponseCode::R200,
                            Some(header_val.clone()),
                            ContentType::TextPlain,
                            &req.protocol,
                        ),
                    ),
                    None => respond_bad_request(stream, &req),
                };
            } else if req.path.starts_with("/files/") {
                let file_path: String = format!("{}/{}", files_dir, req.path.split_at(7).1);

                match req.req_type {
                    ReqType::Get => {
                        let file_content = fs::read_to_string(file_path);
                        match file_content {
                            Ok(content) => respond(
                                stream,
                                Response::new(
                                    &req,
                                    HttpResponseCode::R200,
                                    Some(content),
                                    ContentType::ApplicationOctectStream,
                                    &req.protocol,
                                ),
                            ),
                            Err(_) => {
                                respond_with_default_msg(stream, &req, HttpResponseCode::R404)
                            }
                        };
                    }
                    ReqType::Post => {
                        let _: Result<(), std::io::Error> = fs::create_dir_all(files_dir);

                        let is_file_written: Result<(), std::io::Error> =
                            fs::write(file_path, &req.body);
                        match is_file_written {
                            Ok(_) => respond(
                                stream,
                                Response::new(
                                    &req,
                                    HttpResponseCode::R201,
                                    None,
                                    ContentType::ApplicationOctectStream,
                                    &req.protocol,
                                ),
                            ),
                            Err(_) => {
                                respond_with_default_msg(stream, &req, HttpResponseCode::R404)
                            }
                        }
                    }
                    ReqType::Options => (),
                    ReqType::Connect => (),
                };
            } else {
                respond_with_default_msg(stream, &req, HttpResponseCode::R404);
            }
        }
    }
}

fn main() {
    let default_file_dir: String = String::from("files/");
    let args: Vec<String> = env::args().collect();

    let files_dir: String = if args.len() >= 3 {
        args[2].clone()
    } else {
        default_file_dir
    };

    let files_dir: Arc<String> = Arc::new(files_dir);

    let srvr: RsttpServer = RsttpServer {
        addr: String::from("127.0.0.1:2000"),
        files_dir,
        handler: handle_connection,
    };

    srvr.listen();
}
