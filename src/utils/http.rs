use once_cell::sync::Lazy;
pub use reqwest::*;

pub fn client() -> Client {
    static CLIENT: Lazy<Client> = Lazy::new(|| Client::new());
    CLIENT.clone()
}
