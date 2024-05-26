mod config;
mod currency;
mod exchange;
mod ui;
mod utils;
mod vm;
mod websocket;

use dioxus::prelude::*;

pub fn entrypoint() -> anyhow::Result<()> {
    #[cfg(any(target_arch = "wasm32"))]
    {
        use web_sys::{window, Element};

        use dioxus::web::Config;

        // Get body from the document
        let document = window().unwrap().document().unwrap();
        let body = document.body().unwrap();

        let config = Config::new().rootelement(Element::from(body));
        LaunchBuilder::web().with_cfg(config).launch(ui::App);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        use dioxus::desktop::Config;
        let config = Config::new().with_disable_context_menu(true);
        LaunchBuilder::desktop().with_cfg(config).launch(ui::App);
    }

    Ok(())
}
