use iced::{
    Application, Element, Settings,
    widget::{column, image, text},
};

use std::process::Command;

#[derive(Debug, Clone)]
/// Message
///
/// An iced Message
///
enum Message {}

/// MyApp
///
/// A simple application that displays either an image or a peice of text.
///
struct MyApp<'a> {
    title: String,
    hook: String,
    content: Content<'a>,
}

/// Content
///
/// Contains built iced image and text widgets to render and display
///
struct Content<'a> {
    image: image::Image,
    text: text::Text<'a>,
}

/// Config
///
/// Configuration spec for the application
///
/// src_hub - Remote Content Store URL
/// out_dir - Local temp directory to store the content
///
struct Config {
    src_hub: String,
    out_dir: String,
}

impl<'a> MyApp<'a> {
    /// new()
    ///
    /// To inialize the app
    /// Will initialize the Content and handle the Result
    ///
    fn new() -> Self {
        todo!()
    }
}

impl<'a> Content<'a> {
    /// new()
    ///
    /// To initialize the content, will build the iced widgets
    /// Will attempt the content and store the image
    ///
    fn new() -> Self {
        todo!()
    }
}

impl<'a> Application for MyApp<'a> {
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (Self, Command::none())
    }

    fn title(&self) -> String {
        String::from(self.title)
    }

    fn update(&mut self, _message: Message) -> Command<Message> {
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        column![self.content.image, self.content.image].into()
    }
}

fn main() -> iced::Result {
    MyApp::run(Settings::default())
}
