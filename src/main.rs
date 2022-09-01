#![forbid(unsafe_code)]
#![warn(clippy::all, rust_2018_idioms)]

mod kafka;
mod fltk_gui;

use log::info;
use std::path::PathBuf;
use std::pin::Pin;
use std::rc::Rc;

use tracing_subscriber::fmt::format;

use std::task::{Context, Poll};
use std::time::Duration;
use fltk::{
    app,
    button::Button,
    prelude::*,
    tree::{Tree, TreeSelect},
    window::Window,
};
use tokio::io::{AsyncRead, ReadBuf};

struct SlowRead<R> {
    reader: R,
}

impl<R> SlowRead<R> {
    fn new(reader: R) -> Self {
        Self {
            reader,
        }
    }
}

impl<R> AsyncRead for SlowRead<R>
    where
        R: AsyncRead + Unpin,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let _ = tokio::time::sleep(Duration::from_secs(1));
        Pin::new(&mut self.reader).poll_read(cx, buf)
    }
}

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

    info!("Starting application...");

    let config = sled::Config::default()
        .path(format!("{}/.kators", home_dir))
        .use_compression(true);

    let preferences = config.open().unwrap();
    let servers = preferences.open_tree("servers").unwrap();
    let topics = preferences.open_tree("topics").unwrap();
    let window = preferences.open_tree("window").unwrap();

    let app = fltk_gui::KatorsApp::new();
    let mut wind = Window::default().with_size(400, 300);
    wind.make_resizable(true);
    wind.show();
    // ui.btn.set_callback(move |b| {
    //     b.set_label("clicked");
    //     win.set_label("Button clicked");
    //     println!("Works!");
    // });
    app.run().unwrap();
}
