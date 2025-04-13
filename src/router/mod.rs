use std::collections::HashMap;

use path::{Path, PathParseError};
use route::Route;

use crate::http::{ReqType, Request, Response};

pub mod path;
pub mod route;

#[derive(Debug)]
pub struct Router<Ctx: Send + Sync> {
    routes: Vec<Route<Ctx>>,
}

impl<Ctx: Send + Sync> Router<Ctx> {
    pub fn new() -> Router<Ctx> {
        Router { routes: vec![] }
    }

    pub fn get(&mut self, path: &str, handler: Handler<Ctx>) -> Result<(), PathParseError> {
        self.add_route(ReqType::Get, path, handler)
    }

    pub fn post(&mut self, path: &str, handler: Handler<Ctx>) -> Result<(), PathParseError> {
        self.add_route(ReqType::Post, path, handler)
    }

    pub fn handle_request(&self, req: Request, ctx: &Ctx) -> Response {
        for route in &self.routes {
            if route.req_type == req.req_type && route.path == req.path {
                return (route.handler)(&req, route.path.get_req_param(&req.path), ctx);
            }
        }

        Response::not_found()
    }

    fn add_route(
        &mut self,
        req_type: ReqType,
        path: &str,
        handler: Handler<Ctx>,
    ) -> Result<(), PathParseError> {
        self.routes.push(Route {
            req_type,
            path: Path::parse(path)?,
            handler,
        });

        Ok(())
    }
}

pub type Handler<Ctx> = fn(&Request, Option<HashMap<String, String>>, &Ctx) -> Response;
