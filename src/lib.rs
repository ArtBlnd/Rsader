mod config;
mod currency;
mod exchange;
mod ui;
mod utils;
mod vm;
mod websocket;

use slint::ComponentHandle;

pub fn initialize_and_run() -> anyhow::Result<()> {
    let app = ui::main_window::MainWindow::new()?;

    app.run()?;
    Ok(())
}
