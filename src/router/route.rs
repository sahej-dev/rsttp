use crate::http::ReqType;

use super::{Handler, path::Path};

#[derive(Debug)]
pub struct Route<Ctx: Send + Sync> {
    pub req_type: ReqType,
    pub path: Path,
    pub handler: Handler<Ctx>,
}
