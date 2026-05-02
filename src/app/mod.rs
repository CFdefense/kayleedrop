//! GUI shell: [`MyApp`], [`boot_with_content`], [`update`], [`view`].

use crate::Message;
use crate::content::Content;
use iced::{
    Element, Task,
    widget::{column, image::Image, text},
};

pub const TITLE: &str = "maybe-malware";

/// MyApp
///
/// A simple application that displays either an image or a peice of text.
///
/// title - The title of the application
/// content - Image + text after remote load
///
pub struct MyApp {
    pub title: String,
    pub content: Content,
}

impl MyApp {
    pub fn with_content(content: Content) -> Self {
        Self {
            title: TITLE.to_string(),
            content,
        }
    }
}

/// Build initial iced state after **successful** prefetch in `main` (no duplicate network fetch).
pub fn boot_with_content(content: Content) -> (MyApp, Task<Message>) {
    (MyApp::with_content(content), Task::none())
}

pub fn update(state: &mut MyApp, message: Message) -> Task<Message> {
    match message {
        Message::UpdateText(text) => {
            state.content.text = text;
            Task::none()
        }
        Message::ContentLoaded(Ok(content)) => {
            state.content = content;
            Task::none()
        }
        Message::ContentLoaded(Err(_e)) => Task::none(),
    }
}

pub fn view(state: &MyApp) -> Element<'_, Message> {
    column![
        Image::new(state.content.image_handle.clone()),
        text(state.content.text.as_str()),
    ]
    .into()
}
