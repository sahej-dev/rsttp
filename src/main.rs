use std::sync::Arc;
use std::{env, fs};

use config::Config;
use http::{ContentType, HttpResponseCode, Response};
use router::Router;
use router::path::PathParseError;
use rsttp_server::RsttpServer;

mod config;
mod http;
mod router;
mod rsttp_server;

fn setup_routes(router: &mut Router<AppContext>) -> Result<(), PathParseError> {
    router.get("/", |_req, _, _| Response::success())?;

    router.get("/user-agent", |req, _, _| {
        match req.header_val("User-Agent") {
            Some(header_val) => Response::new(
                req,
                HttpResponseCode::R200,
                Some(header_val.clone()),
                ContentType::TextPlain,
                req.protocol,
            ),
            None => Response::bad_request(),
        }
    })?;

    router.get("/echo/:text", |req, params, _| {
        if let Some(text) = get_param!(params, "text") {
            Response::new(
                req,
                HttpResponseCode::R200,
                Some(text),
                ContentType::TextPlain,
                req.protocol,
            )
        } else {
            Response::bad_request()
        }
    })?;

    router.get("/files/:path", |req, params, ctx| {
        if let Some(path) = get_param!(params, "path") {
            let file_path: String = format!("{}/{}", ctx.static_files_dir, path);
            let file_content = fs::read_to_string(file_path);
            match file_content {
                Ok(content) => Response::new(
                    req,
                    HttpResponseCode::R200,
                    Some(content),
                    ContentType::ApplicationOctectStream,
                    req.protocol,
                ),
                Err(_) => Response::not_found(),
            }
        } else {
            Response::bad_request()
        }
    })?;

    router.post("/files/:path", |req, params, ctx| {
        if let Some(path) = get_param!(params, "path") {
            let file_path: String = format!("{}/{}", ctx.static_files_dir, path);
            let _: Result<(), std::io::Error> = fs::create_dir_all(&ctx.static_files_dir);

            let is_file_written: Result<(), std::io::Error> = fs::write(file_path, &req.body);
            match is_file_written {
                Ok(_) => Response::new(
                    req,
                    HttpResponseCode::R201,
                    None,
                    ContentType::ApplicationOctectStream,
                    req.protocol,
                ),
                Err(_) => Response::not_found(),
            }
        } else {
            Response::bad_request()
        }
    })?;

    Ok(())
}

#[macro_export]
macro_rules! get_param {
    ( $opts:expr, $key:expr ) => {{ $opts.as_ref().and_then(|m| m.get($key)).cloned() }};
}

#[derive(Debug)]
struct AppContext {
    static_files_dir: String,
}

fn main() {
    tracing_subscriber::fmt::init();

    let default_file_dir: String = String::from("files/");
    let args: Vec<String> = env::args().collect();

    let files_dir: String = if args.len() >= 3 {
        args[2].clone()
    } else {
        default_file_dir
    };

    let ctx: AppContext = AppContext {
        static_files_dir: files_dir,
    };

    let config: Config<AppContext> = Config { port: 2000, ctx };

    let mut router: Router<AppContext> = Router::new();

    let _ = setup_routes(&mut router);

    let server: RsttpServer<AppContext> = RsttpServer { config, router };

    let server: Arc<RsttpServer<AppContext>> = Arc::new(server);

    server.listen();
}
