# rsttp

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A simple, multi-threaded HTTP/1.1 server built from scratch in Rust.

`rsttp` is an HTTP/1.1 server built from the ground up in Rust to demonstrate and explore core concepts in network programming, concurrency, and API design. The entire server, including a custom thread pool, was built without relying on external web framework crates like Actix, Axum, or Rocket.

## Features

* **Multi-threaded Processing**: Uses a custom MPSC channel-based thread pool to handle concurrent connections.
* **HTTP/1.1 Compliant**:
  * Correctly parses GET and POST requests.
  * Handles various paths, methods, and headers.
  * Supports **Persistent Connections** (Keep-Alive) with configurable timeouts.
* **Expressive Router**:
  * Simple, programmatic route definitions.
  * Supports dynamic path parameters (e.g., `/users/:id`).
* **Declarative Macro for Parameter Extraction**: Includes a `get_param!` macro for clean and easy extraction of path parameters within route handlers.
* **Generic Application Context**: Share state (like database connections or configuration) with all your route handlers in a type-safe way.
* **Robust and Safe**: Written with a focus on idiomatic Rust, featuring comprehensive error handling and zero uses of `.unwrap()` or `.expect()` in the core logic.
* **Zero Web-Framework Dependencies**: The core server logic is self-contained and built on Rust's standard library.

## Design and Implementation

This project was built to explore the fundamentals of web servers, tackling many of Rust's core concepts to ensure a safe and well-structured design.

### Initial Architecture

The project began by building a TCP listener capable of parsing raw HTTP requests and serving basic responses. From the outset, the focus was on creating a clean architecture. The initial prototype was systematically refactored into logical modules (`http`, `server`, `router`, `config`).

### Concurrency and State Management

A key technical challenge was designing the concurrent connection handler. To ensure thread safety, the server instance is shared across threads using an `Arc<Self>`. This allows multiple threads from a custom-built pool to safely handle incoming requests by working on a shared, immutable reference to the server's configuration.

To solve complex lifetime requirements inherent in concurrent Rust, request-specific data is cloned to the response handler. This design choice satisfies the borrow checker and prevents lifetime-related bugs, a crucial pattern for building safe, concurrent Rust applications.

### Core Components

With a solid concurrent architecture in place, key features were developed:

* A custom thread pool was built from scratch using `std::sync::mpsc` channels for distributing work.
* A flexible router was implemented with support for dynamic paths and a generic `AppContext`.
* `tracing` was integrated for structured, asynchronous-friendly logging.
* A `get_param!` macro was created to improve the ergonomics of route handlers.

#### Protocol Handling

The server was designed to correctly handle key features of HTTP/1.1. The implementation manages TCP stream timeouts to support persistent connections (Keep-Alive), allowing multiple requests to be handled efficiently on a single connection.

## Quick Start

### Prerequisites

* Rust toolchain (`rustc` and `cargo`).

### Installation & Running

1.  Clone the repository:
    ```sh
    git clone https://github.com/sahej-dev/rsttp.git
    cd rsttp
    ```

2.  Run the server:
    ```sh
    cargo run
    ```
    By default, the server listens on port `4221` and serves files from the `./files/` directory.

3.  Run with a custom directory:
    You can provide a command-line argument to specify the directory for serving files.
    ```sh
    cargo run -- /path/to/static/files/directory
    ```

## Usage Example

Here is an example of how a user would import and use the `rsttp` library to build a simple application.

```rust
// Imports from the rsttp library crate and the Rust standard library.
use rsttp::{
    config::Config,
    http::{ContentType, HttpResponseCode, Response},
    router::{PathParseError, Router},
    server::RsttpServer,
    get_param,
};
use std::sync::Arc;
use std::time::Duration;
use std::{env, fs, process};

// 1. The user defines a struct for their application's shared state.
#[derive(Debug, Clone)]
struct AppContext {
    static_files_dir: String,
}

// 2. The user defines their application's routes.
fn define_routes(router: &mut Router<AppContext>) -> Result<(), PathParseError> {
    router.get("/", |_req, _, _| Response::success())?;

    router.get("/echo/:text", |req, params, _| {
        if let Some(text) = get_param!(params, "text") {
            Response::new(
                req,
                HttpResponseCode::R200,
                Some(text),
                ContentType::TextPlain,
            )
        } else {
            Response::bad_request()
        }
    })?;

    router.get("/files/:path", |req, params, ctx| {
        if let Some(path) = get_param!(params, "path") {
            let file_path = format!("{}/{}", ctx.static_files_dir, path);
            match fs::read_to_string(file_path) {
                Ok(content) => Response::new(
                    req,
                    HttpResponseCode::R200,
                    Some(content),
                    ContentType::ApplicationOctectStream,
                ),
                Err(_) => Response::not_found(),
            }
        } else {
            Response::bad_request()
        }
    })?;

    Ok(())
}

fn main() {
    // 3. The user initializes their application context.
    let args: Vec<String> = env::args().collect();
    let files_dir = args.get(1).cloned().unwrap_or_else(|| "files/".to_string());
    let app_context = AppContext {
        static_files_dir: files_dir,
    };

    // 4. The user sets up the server configuration.
    let config = Config {
        port: 4221,
        ctx: app_context,
        persist_connection_for: Duration::from_secs(10),
    };

    // 5. The router is created and routes are registered.
    let mut router = Router::new();
    if let Err(e) = define_routes(&mut router) {
        eprintln!("Error: Failed to define routes: {}", e);
        process::exit(1);
    }

    // 6. The server is created with the config and router, then started.
    let server = RsttpServer::new(config, router, 8); // 8 worker threads
    Arc::new(server).listen();
}
```

## Potential Improvements

* **Non-Blocking I/O with an Event Loop**: Transition from the current thread-pool model to a more advanced architecture by implementing an event loop (e.g., using a polling mechanism like `mio`) on each worker thread. This would enable handling many more concurrent connections with fewer system resources.
* **Middleware Layer**: Implement a middleware layer for cross-cutting concerns like logging, authentication, and request modification.
* **Enhanced Configuration**: Support for configuration from a file (e.g., `config.toml`).
* **Expanded HTTP Feature Set**: Add support for more headers, cookies, and multipart forms.

## Contributing

Contributions, issues, and feature requests are welcome! Feel free to check the [issues page](https://github.com/sahej-dev/rsttp/issues).

## License

This project is licensed under the **MIT License**. See the [LICENSE](LICENSE) file for details.
