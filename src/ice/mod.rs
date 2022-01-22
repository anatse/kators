mod panel;
mod server_pane;

pub use panel::Panel;

use iced::{alignment, Application, Button, Color, Column, Container, Element, Length, pane_grid, PaneGrid, pick_list, PickList, Row, Text};
use iced_native::Command;
use iced_native::widget::pane_grid::{Axis, Pane};
use iced::executor;
use iced_lazy::{Responsive, responsive};
use crate::ice::server_pane::{Server, ServerMessage};

pub struct MainWindow {
    panes: pane_grid::State<Panel>,
    // pick_list: pick_list::State<>,
}

impl Application for MainWindow {
    type Executor = executor::Default;
    type Message = ServerMessage;
    type Flags = ();

    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let panes = pane_grid::State::with_configuration(pane_grid::Configuration::Split {
            axis: Axis::Vertical,
            ratio: 0.2,
            a: Box::new(pane_grid::Configuration::Pane(Panel::new(0))),
            b: Box::new(pane_grid::Configuration::Pane(Panel::new(1)))
        });

        (
            Self {
                panes,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "Kafka Tool Rs".to_string()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            ServerMessage::Resized(pane_grid::ResizeEvent { split, ratio }) => {
                self.panes.resize(&split, ratio);
            }

            _ => {},
        }

        Command::none()
    }

    fn view(&mut self) -> Element<'_, Self::Message> {
        let pane_grid = PaneGrid::new(
            &mut self.panes, |id, pane|
        {
            let is_focused = true;
            let Panel {
                content,
                responsive,
                ..
            } = pane;

            pane_grid::Content::new(Responsive::new(responsive, move |size| {
                content.view()
            }))
                // .style(if is_focused {
                //     style::Pane::Focused
                // } else {
                //     style::Pane::Active
                // })
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(5)
        // .on_click(Message::Clicked)
        // .on_drag(Message::Dragged)
        .on_resize(10, ServerMessage::Resized);

        // let server_list = PickList::new(
        //
        // );

        let title = Text::new("todos")
            .width(Length::Fill)
            .size(100)
            .color([0.5, 0.5, 0.5])
            .horizontal_alignment(alignment::Horizontal::Center);

        let content = Column::new()
            .spacing(5)
            .push(title)
            .push(pane_grid);

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(5)
            .into()
    }
}

const PANE_ID_COLOR_UNFOCUSED: Color = Color::from_rgb(
    0xFF as f32 / 255.0,
    0xC7 as f32 / 255.0,
    0xC7 as f32 / 255.0,
);
const PANE_ID_COLOR_FOCUSED: Color = Color::from_rgb(
    0xFF as f32 / 255.0,
    0x47 as f32 / 255.0,
    0x47 as f32 / 255.0,
);