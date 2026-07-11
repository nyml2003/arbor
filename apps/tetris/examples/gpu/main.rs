mod view;

use std::{
    error::Error,
    sync::Arc,
    time::{Duration, Instant},
};

use punctum_gpu::{GpuAtlas, GpuCell, GpuClip, PixelSize, Rgba8};
use punctum_grid::Surface;
use punctum_tetris::{PieceKind, TetrisCommand, TetrisState, transition};
use punctum_wgpu::{GpuRuntime, PresentOutcome, WinitKeyEventSnapshot, normalize_key_event};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::ModifiersState,
    window::{Window, WindowId},
};

use view::{apply_key, atlas, project_frame, viewport};

const TICK_INTERVAL: Duration = Duration::from_millis(450);
const CLEAR_COLOR: Rgba8 = Rgba8::new(18, 20, 24, 255);

struct TetrisGpu {
    atlas: GpuAtlas,
    state: TetrisState,
    previous: Option<Surface<GpuCell>>,
    modifiers: ModifiersState,
    next_tick: Instant,
    window: Option<Arc<Window>>,
    runtime: Option<GpuRuntime<'static>>,
}

impl TetrisGpu {
    fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            atlas: atlas(),
            state: TetrisState::new(PieceKind::ALL.to_vec())?,
            previous: None,
            modifiers: ModifiersState::empty(),
            next_tick: Instant::now() + TICK_INTERVAL,
            window: None,
            runtime: None,
        })
    }

    fn initialize(&mut self, event_loop: &ActiveEventLoop) -> Result<(), Box<dyn Error>> {
        let attributes = Window::default_attributes()
            .with_title("Punctum Tetris GPU")
            .with_inner_size(LogicalSize::new(480.0, 704.0));
        let window = Arc::new(event_loop.create_window(attributes)?);
        let size = pixel_size(window.inner_size());
        let instance = wgpu::Instance::default();
        let runtime = pollster::block_on(GpuRuntime::new(
            &instance,
            window.clone(),
            size,
            &self.atlas,
            CLEAR_COLOR,
        ))?;

        window.request_redraw();
        self.runtime = Some(runtime);
        self.window = Some(window);
        Ok(())
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let (Some(window), Some(runtime)) = (&self.window, &mut self.runtime) else {
            return;
        };
        let frame = project_frame(self.previous.as_ref(), &self.state);
        let viewport = viewport(runtime.surface_size());
        let result = if let Some(patch) = frame.patch() {
            runtime.present_patch(patch, &self.atlas, viewport, GpuClip::Surface)
        } else {
            runtime.present_surface(frame.surface(), &self.atlas, viewport, GpuClip::Surface)
        };

        match result {
            Ok(outcome) => {
                self.previous = Some(frame.into_surface());
                match outcome {
                    PresentOutcome::Reconfigured => window.request_redraw(),
                    PresentOutcome::SurfaceLost => {
                        runtime.resize(runtime.surface_size());
                        window.request_redraw();
                    }
                    PresentOutcome::Presented
                    | PresentOutcome::PresentedAndReconfigured
                    | PresentOutcome::SkippedMinimized
                    | PresentOutcome::SkippedTimeout
                    | PresentOutcome::SkippedOccluded => {}
                }
            }
            Err(error) => {
                eprintln!("GPU presentation failed: {error}");
                event_loop.exit();
            }
        }
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        if let Some(runtime) = &mut self.runtime {
            runtime.resize(pixel_size(size));
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn handle_tick(&mut self) {
        let now = Instant::now();
        let mut changed = false;
        while now >= self.next_tick {
            let next = transition(&self.state, TetrisCommand::Tick);
            changed |= next != self.state;
            self.state = next;
            self.next_tick += TICK_INTERVAL;
        }
        if changed && let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

impl ApplicationHandler for TetrisGpu {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none()
            && let Err(error) = self.initialize(event_loop)
        {
            eprintln!("GPU initialization failed: {error}");
            event_loop.exit();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.window.as_ref().map(|window| window.id()) != Some(window_id) {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => self.resize(size),
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(window) = &self.window {
                    self.resize(window.inner_size());
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers.state(),
            WindowEvent::KeyboardInput { event, .. } => {
                let key = normalize_key_event(WinitKeyEventSnapshot::new(
                    event.physical_key,
                    event.logical_key,
                    self.modifiers,
                    event.state,
                    event.repeat,
                ));
                let next = apply_key(&self.state, &key);
                if next != self.state {
                    self.state = next;
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.handle_tick();
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_tick));
    }
}

fn pixel_size(size: PhysicalSize<u32>) -> PixelSize {
    PixelSize::new(size.width, size.height)
}

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = TetrisGpu::new()?;
    event_loop.run_app(&mut app)?;
    Ok(())
}
