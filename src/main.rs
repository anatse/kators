#![forbid(unsafe_code)]
#![warn(clippy::all, rust_2018_idioms)]

mod d_gui;
mod kafka;

use crate::d_gui::KatorApp;
use log::info;
use std::path::PathBuf;
use std::rc::Rc;
use tracing_subscriber::fmt::format;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    tracing_subscriber::fmt::init();

    let home_dir = match dirs::home_dir() {
        Some(hd) => hd.to_str().unwrap().to_string(),
        None => {
            info!("User directory unknown, use current");
            ".".to_string()
        }
    };

    let config = sled::Config::default()
        .path(format!("{}/.kators", home_dir))
        .use_compression(true);

    let preferences = config.open().unwrap();
    let servers = preferences.open_tree("servers").unwrap();
    let topics = preferences.open_tree("topics").unwrap();
    let app = KatorApp::new(Rc::new(servers), Rc::new(topics));
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}
