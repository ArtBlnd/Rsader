pub use reqwest::*;

pub fn http_client() -> Client {
    Client::new()
}
