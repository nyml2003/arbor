mod clipboard;
mod diagnostics;
mod dpi;
mod error;
mod host;
mod input;

pub use error::PlatformResult;

pub fn run() -> PlatformResult<()> {
    dpi::set_thread_dpi_awareness();
    let _com = host::ComApartment::init()?;
    let mut window = host::ClipDockWindow::new()?;
    window.run()
}

pub fn report_error(message: &str) {
    diagnostics::report_error(message);
}
