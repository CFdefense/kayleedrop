//! Iced GUI shell: [`MyApp`], bootstrap, [`update`], [`view`].

use crate::Message;
use crate::content::Content;
use iced::{
    Element, Length, Task,
    alignment::{Horizontal, Vertical},
    widget::{column, container, image::Image, text},
};

/// Default window title surfaced in iced window chrome.
///
/// Duplicated verbatim into [`MyApp::title`] during construction.
pub const TITLE: &str = "kayleedrop";

/// Holds UI state fetched before the iced runtime spins up.
///
/// Fields are public so `main` can wire [`iced::application`] title hooks.
pub struct MyApp {
    pub title: String,
    pub content: Content,
}

impl MyApp {
    /// Constructs UI state ahead of spawning iced.
    ///
    /// Copies the shared [`TITLE`] into `title` and stores the decrypted [`crate::content::Content`]
    /// bundle for rendering. Does not fetch network data or resize windows.
    pub fn with_content(content: Content) -> Self {
        Self {
            title: TITLE.to_string(),
            content,
        }
    }
}

/// Produces iced’s initial `(state, task)` tuple once `main` has finished prefetching remote content.
///
/// Returns [`iced::task::Task::none`] for the boot task chain; companion window resizing (if any) is
/// owned by crate-level bootstrap rather than nested here.
pub fn boot_with_content(content: Content) -> (MyApp, Task<Message>) {
    (MyApp::with_content(content), Task::none())
}

/// Mutates [`MyApp::content`] in response to iced messages.
///
/// Handles typed caption updates plus successful [`crate::Message::ContentLoaded`] payloads by
/// replacing the entire [`crate::content::Content`]. Errors from async loads produce no-op tasks so
/// the UI keeps the previous bundle.
///
/// Returned tasks are always idle (`Task::none`) in current builds—reserved for future side effects.
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

/// Renders stacked image + text inside a padded, centered iced container tree.
///
/// Uses fill lengths so the widgets stay centered regardless of fractional window chrome; callers
/// are expected to size the outer window tightly around the raster when desired.
pub fn view(state: &MyApp) -> Element<'_, Message> {
    container(
        column![
            Image::new(state.content.image_handle.clone()),
            text(state.content.text.as_str()),
        ]
        .spacing(8)
        .align_x(Horizontal::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(Horizontal::Center)
    .align_y(Vertical::Center)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Message;
    use iced::widget::image;

    #[test]
    fn with_content_sets_title_constant() {
        let c = Content::default();
        let app = MyApp::with_content(c.clone());
        assert_eq!(app.title, TITLE);
        assert_eq!(app.content.text, c.text);
    }

    #[test]
    fn update_text_changes_caption_only() {
        let mut app = MyApp::with_content(Content::default());
        let _task = update(&mut app, Message::UpdateText("lebron".into()));
        assert_eq!(app.content.text, "lebron");
    }

    #[test]
    fn content_loaded_replaces_bundle() {
        let mut app = MyApp::with_content(Content::default());
        let next = Content::new(image::Handle::from_bytes(vec![1]), (2, 3), "x".into());
        let _task = update(&mut app, Message::ContentLoaded(Ok(next.clone())));
        assert_eq!(app.content.image_size, (2, 3));
        assert_eq!(app.content.text, "x");
    }
}
