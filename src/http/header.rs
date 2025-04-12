pub trait HttpHeader {
    fn key(&self) -> &str;
    fn val(&self) -> String;

    fn in_raw_http_form(&self) -> String {
        format!("{}: {}\r\n", self.key(), self.val())
    }
}
