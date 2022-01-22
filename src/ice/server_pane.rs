use std::fmt::format;
use std::os::macos::raw::stat;
use iced::{Button, Column, Container, Element, Length, pane_grid, Point, Rectangle, scrollable, Scrollable, Size, Text};
use iced_native::layout::{Limits, Node};
use iced_native::renderer::Style;
use iced_native::{Hasher, Layout, layout, text, Widget};
use rdkafka::metadata::Metadata;

pub struct ServerPane {
    servers: Vec<Server>,
    scrollable: scrollable::State,
}

#[derive(Debug, Clone, Copy)]
pub enum ServerMessage {
    OpenServer,
    CloseServer,
    SelectServer,
    Resized(pane_grid::ResizeEvent),
}

pub struct Server {
    name: String,
    metadata: Metadata,
}

impl ServerPane {
    pub fn new() -> Self {
        let mut state = scrollable::State::new();

        Self {
            servers: Vec::default(),
            scrollable: state,
        }
    }

    pub fn view(&mut self) -> Element<ServerMessage> {
        let scrollable = (0..200).into_iter().fold(Scrollable::new(&mut self.scrollable)
            .padding(10)
            .spacing(10)
            .width(Length::Fill)
            .height(Length::Fill)
            .on_scroll(move |offset| {
                ServerMessage::SelectServer
            }), |scrollable, i| {
            scrollable.push(Text::new(format!("Item: {}", i)))
        });

        Container::new(scrollable)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(5)
            .center_y()
            .into()
    }
}