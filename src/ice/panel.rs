use iced::{alignment, Alignment, button, Button, Column, Container, Element, Length, pane_grid, scrollable, Scrollable, Size, Text};
use iced_lazy::responsive;
use crate::ice::server_pane::ServerPane;

pub struct Panel {
    pub content: ServerPane,
    pub responsive: responsive::State,
}

impl Panel {
    pub(crate) fn new(id: usize) -> Self {
        Self {
            content: ServerPane::new(),
            responsive: responsive::State::new(),
        }
    }
}
