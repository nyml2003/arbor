pub use thorn_core::*;
pub use thorn_terminal as terminal;

use layout::Rect;
use reactive::Scope;
use render::{diff, render_tree, DirtyRegion, Screen};
use terminal::TerminalBackend;
use theme::Theme;
use view::View;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    MissingRoot,
    Terminal(terminal::TerminalError),
}

impl From<terminal::TerminalError> for Error {
    fn from(value: terminal::TerminalError) -> Self {
        Self::Terminal(value)
    }
}

type Root<Action> = Box<dyn FnOnce(&Scope) -> View<Action>>;

pub fn app<Action>(root: impl FnOnce(&Scope) -> View<Action> + 'static) -> App<Action> {
    App {
        root: Some(Box::new(root)),
        theme: Theme::dark(),
    }
}

pub struct App<Action> {
    root: Option<Root<Action>>,
    theme: Theme,
}

impl<Action> App<Action> {
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    pub fn run(mut self) -> Result<()> {
        let root = self.root.take().ok_or(Error::MissingRoot)?;
        let scope = Scope::new();
        let root = scope.enter(|| root(&scope));
        let mut backend = terminal::CrosstermBackend::new();
        let _guard = backend.enter()?;
        let mut previous = None;

        render_frame(&mut backend, &root, &self.theme, &mut previous)?;

        loop {
            match backend.read_event()? {
                terminal::TerminalEvent::Quit => break,
                terminal::TerminalEvent::Enter => {
                    root.press_first_focusable();
                    render_frame(&mut backend, &root, &self.theme, &mut previous)?;
                }
                terminal::TerminalEvent::Resize => {
                    previous = None;
                    render_frame(&mut backend, &root, &self.theme, &mut previous)?;
                }
                terminal::TerminalEvent::Other => {}
            }
        }

        Ok(())
    }
}

fn render_frame<Action>(
    backend: &mut impl TerminalBackend,
    root: &View<Action>,
    theme: &Theme,
    previous: &mut Option<Screen>,
) -> Result<()> {
    let (width, height) = backend.size()?;
    let (next_screen, _) = render_tree(root, width, height, theme);
    let dirty_regions = previous
        .as_ref()
        .map(|old| diff(old, &next_screen))
        .unwrap_or_else(|| full_screen_dirty(width, height));

    backend.emit(&dirty_regions, &next_screen)?;
    backend.flush()?;
    *previous = Some(next_screen);
    Ok(())
}

fn full_screen_dirty(width: u16, height: u16) -> Vec<DirtyRegion> {
    vec![DirtyRegion {
        rect: Rect::new(0, 0, width, height),
    }]
}

pub mod prelude {
    pub use thorn_core::prelude::*;
}

#[cfg(test)]
mod tests {
    use super::{full_screen_dirty, prelude::*};

    enum Action {}

    #[test]
    fn facade_exports_minimal_user_api() {
        fn root(_: &Scope) -> View<Action> {
            text("hello")
        }

        let mut app = TestApp::new(root);
        app.render(20, 4);
        app.assert_text("hello");
    }

    #[test]
    fn initial_frame_dirty_region_covers_full_screen() {
        assert_eq!(full_screen_dirty(4, 2)[0].rect, Rect::new(0, 0, 4, 2));
    }
}
