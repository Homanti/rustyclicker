#![windows_subsystem = "windows"]

use iced::mouse::Interaction;
use iced::widget::{button, column, container, mouse_area, row, scrollable, text, text_input, Button, Space};
use iced::{window, Alignment, Border, Element, Length, Task};
use iced::border::Radius;

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .window(window::Settings {
            size: iced::Size::new(430.0, 480.0),
            ..Default::default()
        })
        .run()
}
#[derive(Debug, Clone)]
struct Point {
    id: usize,
    x: usize,
    y: usize,
}

struct App {
    points: Vec<Point>,
    hovered: Option<usize>,
    clicker_started: bool,
    x_input: String,
    y_input: String,
}

#[derive(Debug, Clone)]
enum Message {
    ListElementClicked(usize),
    ListHoverEnter(usize),
    ListHoverExit,
    StartButtonPressed,
    AddPoint,
    XAxisChanged(String),
    YAxisChanged(String),
}

fn custom_button(label: &str) -> Button<'_, Message> {
    button(
        text(label).align_x(Alignment::Center).width(Length::Fill)
    )
        .style(move |_, status| {
            let bg = match status {
                button::Status::Active => iced::Color::from_rgb8(65, 90, 119),
                button::Status::Hovered => iced::Color::from_rgb8(85, 110, 139),
                button::Status::Pressed => iced::Color::from_rgb8(45, 70, 99),
                button::Status::Disabled => iced::Color::from_rgb8(35, 45, 58),
            };

            let text_color = match status {
                button::Status::Disabled => iced::Color::from_rgb8(100, 100, 110),
                _ => iced::Color::from_rgb8(224, 225, 221),
            };

            button::Style {
                background: Some(iced::Background::Color(bg)),
                text_color,
                border: Border {
                    color: iced::Color::TRANSPARENT,
                    width: 0.0,
                    radius: Radius::from(8.0),
                },
                ..Default::default()
            }
        })
}

impl App {
    fn new() -> Self {
        Self {
            points: vec![],
            hovered: None,
            clicker_started: false,
            x_input: "0".to_string(),
            y_input: "0".to_string(),
        }
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::ListElementClicked(id) => {
                self.points.retain(|p| p.id != id);
            }
            Message::ListHoverEnter(i) => self.hovered = Some(i),
            Message::ListHoverExit => self.hovered = None,
            Message::StartButtonPressed => {
                self.clicker_started = !self.clicker_started;
                println!("Start button pressed");
            },
            Message::AddPoint => {}
            Message::XAxisChanged(val) => self.x_input = val,
            Message::YAxisChanged(val) => self.y_input = val,
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let rows = self.points.iter().map(|point| {
            let x = point.x;
            let y = point.y;
            let id = point.id;

            let bg = if self.hovered == Some(point.id) {
                iced::Background::Color(iced::Color::from_rgb8(27, 38, 59))
            } else {
                iced::Background::Color(iced::Color::TRANSPARENT)
            };

            mouse_area(
                container(text(format!("{}. {}, {}", id, x, y)))
                    .padding(5)
                    .width(Length::Fill)
                    .style(move |_| container::Style {
                        background: Some(bg),
                        border: Border {
                            color: iced::Color::TRANSPARENT,
                            width: 0.0,
                            radius: Radius {
                                top_left: 8.0,
                                top_right: 8.0,
                                bottom_left: 8.0,
                                bottom_right: 8.0
                            }
                        },
                        ..Default::default()
                    }),
            )
            .on_release(Message::ListElementClicked(point.id))
            .on_enter(Message::ListHoverEnter(point.id))
            .on_press(Message::ListHoverEnter(point.id))
            .on_move(|_| Message::ListHoverEnter(point.id))
            .on_exit(Message::ListHoverExit)
            .interaction(Interaction::Pointer)
            .into()
        });

        let points_list = column![
            container(text(format!("{} points", self.points.len()))),
            scrollable(column(rows).spacing(1)).width(Length::Fill),
        ].height(Length::Fill).spacing(10);

        let axis_shift_row = column![
            row![
                text("X-axis shift"),
                Space::default().width(Length::Fill),
                text("Y-axis shift"),
            ],
            row![
                text_input("X axis", &self.x_input).on_input(Message::XAxisChanged),
                Space::default().width(Length::Fill),
                text_input("Y axis", &self.y_input).on_input(Message::YAxisChanged),
            ]
        ];

        let app_controls = column![
            custom_button(if self.clicker_started {"Stop"} else {"Start"})
                .on_press_maybe(if self.points.is_empty() { None } else { Some(Message::StartButtonPressed) })
                .width(Length::Fill),

            custom_button("Add point")
                .on_press(Message::AddPoint)
                .width(Length::Fill),
        ].spacing(5);

        container(
            column![points_list, axis_shift_row, app_controls].spacing(10)
        ).style(move |_| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgb8(13, 27, 42))),
            text_color: Some(iced::Color::from_rgb8(224, 225, 221)),
            ..Default::default()
        }).padding(5).into()
    }
}
