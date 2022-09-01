mod model;

use std::ops::Deref;
use fltk::{
    app,
    button::Button,
    prelude::*,
    tree::{Tree, TreeSelect},
    window::Window,
};

pub struct KatorsApp {
    app: app::App,
}

impl Deref for KatorsApp {
    type Target = app::App;
    fn deref(&self) -> &Self::Target {
        &self.app
    }
}

impl KatorsApp {
    pub fn new() -> Self {
        Self {
            app: app::App::default().with_scheme(app::Scheme::Plastic),
        }
    }

    pub fn run(&self) -> anyhow::Result<()> {
        self.app.run().map_err(|e|e.into())
    }

    pub fn create_window() {

    }
}