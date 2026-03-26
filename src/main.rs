#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashSet;
use std::path::PathBuf;
use iced::border::Radius;
use iced::mouse::Interaction;
use iced::widget::{
    Button, Space, button, column, container, mouse_area, row, scrollable, text, text_input,
};
use iced::{Alignment, Border, Element, Length, Task, window};
use rdev::{Event, EventType, listen};
use std::{fs, thread};
use std::time::Duration;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use rand::prelude::*;
use serde::{Deserialize, Serialize};

const DEFAULT_INTERVAL: u64 = 1000;

#[derive(Serialize, Deserialize)]
struct AppData {
    points: Vec<Point>,
    x_axis_shift: usize,
    y_axis_shift: usize,
    interval: u64,
}

impl Default for AppData {
    fn default() -> Self {
        Self {
            points: vec![],
            x_axis_shift: 0,
            y_axis_shift: 0,
            interval: DEFAULT_INTERVAL,
        }
    }
}

fn config_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("RustyClicker");
    path.push("config.json");
    path
}

fn save(data: &AppData) {
    let path = config_path();
    fs::create_dir_all(path.parent().unwrap()).ok();
    let json = serde_json::to_string_pretty(data).unwrap();
    fs::write(path, json).ok();
}

fn load() -> AppData {
    let path = config_path();
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .window(window::Settings {
            size: iced::Size::new(430.0, 480.0),
            ..Default::default()
        })
        .subscription(App::input_subscription)
        .title("RustyClicker")
        .exit_on_close_request(false)
        .run()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Point {
    id: usize,
    x: usize,
    y: usize,
}

struct App {
    points: Vec<Point>,
    list_element_hovered: Option<usize>,
    clicker_running: Arc<AtomicBool>,
    x_input: String,
    y_input: String,
    interval_input: String,
    waiting_for_click: bool,
    pressed_buttons: HashSet<rdev::Key>,
}

#[derive(Debug, Clone)]
enum Message {
    ListElementClicked(usize),
    ListHoverEnter(usize),
    ListHoverExit,
    ToggleClicker,
    AddPoint,
    XAxisShiftChanged(String),
    YAxisShiftChanged(String),
    IntervalChanged(String),
    PointCaptured(f64, f64),
    KeyPressed(rdev::Key),
    KeyReleased(rdev::Key),
    IcedKeyPressed(iced::keyboard::Key),
    IcedKeyReleased(iced::keyboard::Key),
    SaveAndClose
}

fn custom_button(label: &str) -> Button<'_, Message> {
    button(text(label).align_x(Alignment::Center).width(Length::Fill)).style(move |_, status| {
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
        let data = load();

        Self {
            points: data.points,
            list_element_hovered: None,
            clicker_running: Arc::new(AtomicBool::new(false)),
            x_input: data.x_axis_shift.to_string(),
            y_input: data.y_axis_shift.to_string(),
            interval_input: data.interval.to_string(),
            waiting_for_click: false,
            pressed_buttons: HashSet::new(),
        }
    }

    fn ctrl(&self) -> bool {
        self.pressed_buttons.contains(&rdev::Key::ControlLeft)
            || self.pressed_buttons.contains(&rdev::Key::ControlRight)
    }

    fn shift(&self) -> bool {
        self.pressed_buttons.contains(&rdev::Key::ShiftLeft)
            || self.pressed_buttons.contains(&rdev::Key::ShiftRight)
    }

    fn check_hotkeys(&mut self) -> Option<Message> {
        // Ctrl + Shift + C
        if self.ctrl() && self.shift() && self.pressed_buttons.contains(&rdev::Key::KeyC) {
            return Some(Message::ToggleClicker);
        }
        None
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::ToggleClicker => {
                if !self.points.is_empty() {
                    let running = self.clicker_running.load(Ordering::Relaxed);
                    if running {
                        self.clicker_running.store(false, Ordering::Relaxed);
                    } else {
                        self.start_clicker();
                    }
                }
            }
            Message::ListElementClicked(id) => {
                self.points.retain(|p| p.id != id);
                if self.points.is_empty() {
                    self.clicker_running.store(false, Ordering::Relaxed);
                }
            }
            Message::ListHoverEnter(i) => self.list_element_hovered = Some(i),
            Message::ListHoverExit => self.list_element_hovered = None,
            Message::AddPoint => {
                self.waiting_for_click = true;
                let _ = Task::<Message>::none();
            },
            Message::XAxisShiftChanged(val) => {
                if val.chars().all(|c| c.is_ascii_digit()) {
                    self.x_input = val;
                }
            },
            Message::YAxisShiftChanged(val) => {
                if val.chars().all(|c| c.is_ascii_digit()) {
                    self.y_input = val;
                }
            },
            Message::IntervalChanged(val) => {
                if val.chars().all(|c| c.is_ascii_digit()) {
                    self.interval_input = val;
                }
            }
            Message::PointCaptured(x, y) => {
                if !self.waiting_for_click {
                    return Task::none();
                }

                self.waiting_for_click = false;

                let offset_x = self.x_input.parse::<usize>().unwrap_or(0);
                let offset_y = self.y_input.parse::<usize>().unwrap_or(0);
                let id = self.points.len();

                self.points.push(Point {
                    id,
                    x: x as usize + offset_x,
                    y: y as usize + offset_y,
                });
            },
            Message::KeyPressed(key) => {
                self.pressed_buttons.insert(key);
                if let Some(msg) = self.check_hotkeys() {
                    return self.update(msg);
                }
            }
            Message::KeyReleased(key) => {
                self.pressed_buttons.remove(&key);
            }
            Message::IcedKeyPressed(key) => {
                if let iced::keyboard::Key::Named(named) = &key {
                    match named {
                        iced::keyboard::key::Named::Control => { self.pressed_buttons.insert(rdev::Key::ControlLeft); }
                        iced::keyboard::key::Named::Shift => { self.pressed_buttons.insert(rdev::Key::ShiftLeft); }
                        _ => {}
                    }
                }
                if let iced::keyboard::Key::Character(c) = &key {
                    if c.as_str() == "c" { self.pressed_buttons.insert(rdev::Key::KeyC); }
                }

                if let Some(msg) = self.check_hotkeys() {
                    return self.update(msg);
                }
            }
            Message::IcedKeyReleased(key) => {
                if let iced::keyboard::Key::Named(named) = &key {
                    match named {
                        iced::keyboard::key::Named::Control => { self.pressed_buttons.remove(&rdev::Key::ControlLeft); }
                        iced::keyboard::key::Named::Shift => { self.pressed_buttons.remove(&rdev::Key::ShiftLeft); }
                        _ => {}
                    }
                }
                if let iced::keyboard::Key::Character(c) = &key {
                    if c.as_str() == "c" { self.pressed_buttons.remove(&rdev::Key::KeyC); }
                }
            }
            Message::SaveAndClose => {
                save(&AppData {
                    points: self.points.clone(),
                    x_axis_shift: self.x_input.parse().unwrap_or(0),
                    y_axis_shift: self.y_input.parse().unwrap_or(0),
                    interval: self.interval_input.parse().unwrap_or(DEFAULT_INTERVAL),
                });

                std::process::exit(0);
            }
        }
        Task::none()
    }

    // keyboard and mouse input listener
    fn input_subscription(&self) -> iced::Subscription<Message> {
        let rdev_sub = iced::Subscription::run_with(0u8, |_data| {
            iced::stream::channel(100, async |mut sender| {
                thread::spawn(move || {
                    let mut pos_x = 0.0f64;
                    let mut pos_y = 0.0f64;

                    listen(move |event: Event| {
                        match event.event_type {
                            EventType::MouseMove { x, y } => {
                                pos_x = x;
                                pos_y = y;
                            },
                            EventType::ButtonRelease(rdev::Button::Left) => {
                                sender.try_send(Message::PointCaptured(pos_x, pos_y)).ok();
                            },
                            EventType::KeyPress(key) => {
                                sender.try_send(Message::KeyPressed(key)).ok();
                            },
                            EventType::KeyRelease(key) => {
                                sender.try_send(Message::KeyReleased(key)).ok();
                            }
                            _ => {}
                        }
                    }).ok();
                });

                loop {
                    iced::futures::future::pending::<()>().await;
                }
            })
        });

        let iced_key_sub = iced::keyboard::listen().map(|event| match event {
            iced::keyboard::Event::KeyPressed { key, .. } => Message::IcedKeyPressed(key),
            iced::keyboard::Event::KeyReleased { key, .. } => Message::IcedKeyReleased(key),
            _ => { Message::IcedKeyReleased(iced::keyboard::Key::Unidentified) },
        });

        let close_sub = window::close_requests().map(|_| Message::SaveAndClose);

        iced::Subscription::batch([rdev_sub, iced_key_sub, close_sub])
    }

    fn start_clicker(&self) {
        let flag = self.clicker_running.clone();
        flag.store(true, Ordering::Relaxed);

        let points = self.points.clone();
        let interval_ms = self.interval_input.parse::<u64>().unwrap_or(DEFAULT_INTERVAL);

        let x_axis_shift = self.x_input.parse::<usize>().unwrap_or(0);
        let y_axis_shift = self.y_input.parse::<usize>().unwrap_or(0);

        fn make_click(x: f64, y: f64, rng: &mut impl Rng, x_axis_shift: usize, y_axis_shift: usize) {
            let dx = rng.random_range(-(x_axis_shift as i64)..=(x_axis_shift as i64));
            let dy = rng.random_range(-(y_axis_shift as i64)..=(y_axis_shift as i64));
            rdev::simulate(&EventType::MouseMove { x: x + dx as f64, y: y + dy as f64 }).ok();
            rdev::simulate(&EventType::ButtonPress(rdev::Button::Left)).ok();
            rdev::simulate(&EventType::ButtonRelease(rdev::Button::Left)).ok();
        }

        thread::spawn(move || {
            let mut rng = rand::rng();

            while flag.load(Ordering::Relaxed) {
                points.iter().for_each(|point| {
                    make_click(point.x as f64, point.y as f64, &mut rng, x_axis_shift, y_axis_shift);
                    thread::sleep(Duration::from_millis(interval_ms));
                })
            }
        });
    }

    fn view(&self) -> Element<'_, Message> {
        let rows = self.points.iter().map(|point| {
            let x = point.x;
            let y = point.y;
            let id = point.id;

            let bg = if self.list_element_hovered == Some(point.id) {
                iced::Background::Color(iced::Color::from_rgb8(27, 38, 59))
            } else {
                iced::Background::Color(iced::Color::TRANSPARENT)
            };

            mouse_area(
                container(text(format!("{}. {}, {}", id + 1, x, y)))
                    .padding(5)
                    .width(Length::Fill)
                    .style(move |_| container::Style {
                        background: Some(bg),
                        border: Border {
                            color: iced::Color::TRANSPARENT,
                            width: 0.0,
                            radius: Radius::from(8.0),
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
            container(
                row![
                    text(format!("Points list: {} points", self.points.len())),
                    Space::default().width(Length::Fill),
                    text("Start/Stop - Ctrl + Shift + C")
                ]
            ),
            container(scrollable(column(rows).spacing(1)).width(Length::Fill),)
                .style(move |_| container::Style {
                    border: Border {
                        color: iced::Color::from_rgb8(74, 123, 196),
                        width: 1.0,
                        radius: Radius::from(8.0)
                    },
                    ..Default::default()
                })
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(5),
        ]
        .height(Length::Fill)
        .spacing(10);

        let axis_shift_row = column![
            row![
                text("X-axis shift"),
                Space::default().width(Length::Fill),
                text("Y-axis shift"),
            ],
            row![
                text_input("X axis (px)", &self.x_input).on_input(Message::XAxisShiftChanged),
                Space::default().width(Length::Fill),
                text_input("Y axis (px)", &self.y_input).on_input(Message::YAxisShiftChanged),
            ]
        ];

        let app_controls = column![
            text_input("Interval (ms)", &self.interval_input).on_input(Message::IntervalChanged),
            custom_button(if self.clicker_running.load(Ordering::Relaxed) {
                "Stop"
            } else {
                "Start"
            })
            .on_press_maybe(if self.points.is_empty() {
                None
            } else {
                Some(Message::ToggleClicker)
            })
            .width(Length::Fill),
            custom_button("Add point")
                .on_press(Message::AddPoint)
                .width(Length::Fill),
        ]
        .spacing(5);

        container(column![points_list, axis_shift_row, app_controls].spacing(10))
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgb8(13, 27, 42))),
                text_color: Some(iced::Color::from_rgb8(224, 225, 221)),
                ..Default::default()
            })
            .padding(5)
            .into()
    }
}
