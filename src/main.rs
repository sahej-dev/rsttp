use std::sync::Arc;
use std::{env, fs};

use config::Config;
use http::{ContentType, HttpResponseCode, ReqType, Request, Response};
use rsttp_server::RsttpServer;

mod config;
mod http;
mod rsttp_server;

fn handle_connection(req: Request, files_dir: &String) -> Response {
    println!("Request: {:?}", req);

    if req.path == "/" {
        Response::success()
    } else if req.path.starts_with("/echo/") {
        if req.path == "/echo/" {
            Response::bad_request()
        } else {
            Response::new(
                &req,
                HttpResponseCode::R200,
                Some(String::from(req.path.split_at(6).1)),
                ContentType::TextPlain,
                req.protocol,
            )
        }
    } else if req.path == "/user-agent" {
        match req.header_val("User-Agent") {
            Some(header_val) => Response::new(
                &req,
                HttpResponseCode::R200,
                Some(header_val.clone()),
                ContentType::TextPlain,
                req.protocol,
            ),
            None => Response::bad_request(),
        }
    } else if req.path.starts_with("/files/") {
        let file_path: String = format!("{}/{}", files_dir, req.path.split_at(7).1);

        match req.req_type {
            ReqType::Get => {
                let file_content = fs::read_to_string(file_path);
                match file_content {
                    Ok(content) => Response::new(
                        &req,
                        HttpResponseCode::R200,
                        Some(content),
                        ContentType::ApplicationOctectStream,
                        req.protocol,
                    ),
                    Err(_) => Response::not_found(),
                }
            }
            ReqType::Post => {
                let _: Result<(), std::io::Error> = fs::create_dir_all(files_dir);

                let is_file_written: Result<(), std::io::Error> = fs::write(file_path, &req.body);
                match is_file_written {
                    Ok(_) => Response::new(
                        &req,
                        HttpResponseCode::R201,
                        None,
                        ContentType::ApplicationOctectStream,
                        req.protocol,
                    ),
                    Err(_) => Response::not_found(),
                }
            }
            ReqType::Options => todo!(),
            ReqType::Connect => todo!(),
        }
    } else {
        Response::not_found()
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

    // let files_dir: Arc<String> = Arc::new(files_dir);

    let server: RsttpServer = RsttpServer {
        config: Config {
            port: 2000,
            static_files_dir: files_dir,
        },
        handler: handle_connection,
    };

    let server: Arc<RsttpServer> = Arc::new(server);

    server.listen();
}
