mod text;

use std::{
    error::Error,
    sync::Arc,
    time::{Duration, Instant},
};

use game_host::DemoBattle;
use game_ui::{CANVAS_HEIGHT, CANVAS_WIDTH, atlas};
use punctum_gpu::{GpuAtlas, GpuClip, PixelOffset, PixelSize, Rgba8, Viewport, plan_surface};
use punctum_wgpu::{GpuRuntime, PresentOutcome, WinitKeyEventSnapshot, normalize_key_event};
use text::BattleTextRenderer;
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::ModifiersState,
    window::{Window, WindowId},
};

const CLEAR_COLOR: Rgba8 = Rgba8::new(14, 18, 24, 255);

struct CreatureBattleApp {
    battle: DemoBattle,
    atlas: GpuAtlas,
    text_renderer: BattleTextRenderer,
    modifiers: ModifiersState,
    next_playback: Option<Instant>,
    window: Option<Arc<Window>>,
    runtime: Option<GpuRuntime<'static>>,
}

impl CreatureBattleApp {
    fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            battle: DemoBattle::new()
                .map_err(|error| std::io::Error::other(format!("demo battle: {error:?}")))?,
            atlas: atlas(),
            text_renderer: BattleTextRenderer::new(),
            modifiers: ModifiersState::empty(),
            next_playback: None,
            window: None,
            runtime: None,
        })
    }

    fn initialize(&mut self, event_loop: &ActiveEventLoop) -> Result<(), Box<dyn Error>> {
        let window = Arc::new(
            event_loop.create_window(
                Window::default_attributes()
                    .with_title("Arbor 精灵对战")
                    .with_inner_size(LogicalSize::new(960.0, 720.0)),
            )?,
        );
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
        self.window = Some(window);
        self.runtime = Some(runtime);
        Ok(())
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let (Some(window), Some(runtime)) = (&self.window, &mut self.runtime) else {
            return;
        };
        let view = self.battle.view();
        let surface_size = runtime.surface_size();
        let viewport = battle_viewport(surface_size);
        let plan = match plan_surface(
            view.surface(),
            &self.atlas,
            u32::MAX,
            viewport,
            GpuClip::Surface,
        ) {
            Ok(plan) => plan,
            Err(error) => {
                eprintln!("battle GPU planning failed: {error}");
                event_loop.exit();
                return;
            }
        };
        let labels = view.labels();
        let renderer = &mut self.text_renderer;
        let mut text_result = Ok(());
        let result = runtime.present_plan_with_overlay(
            &plan,
            |device, queue, target, encoder, format, size| {
                text_result = renderer.encode(
                    labels, viewport, device, queue, target, encoder, format, size,
                );
            },
        );
        if let Err(error) = text_result {
            eprintln!("battle text rendering failed: {error}");
            event_loop.exit();
            return;
        }
        match result {
            Ok(PresentOutcome::Reconfigured | PresentOutcome::SurfaceLost) => {
                runtime.resize(runtime.surface_size());
                window.request_redraw();
            }
            Ok(
                PresentOutcome::Presented
                | PresentOutcome::PresentedAndReconfigured
                | PresentOutcome::SkippedMinimized
                | PresentOutcome::SkippedTimeout
                | PresentOutcome::SkippedOccluded,
            ) => {}
            Err(error) => {
                eprintln!("battle presentation failed: {error}");
                event_loop.exit();
            }
        }
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        if let Some(runtime) = &mut self.runtime {
            runtime.resize(pixel_size(size));
        }
        self.request_redraw();
    }

    fn handle_key(&mut self, event: winit::event::KeyEvent) {
        let key = normalize_key_event(WinitKeyEventSnapshot::new(
            event.physical_key,
            event.logical_key,
            self.modifiers,
            event.state,
            event.repeat,
        ));
        match self.battle.handle_key(&key) {
            Ok(true) => {
                if self.battle.has_pending_playback() {
                    self.next_playback = Some(Instant::now() + Duration::from_millis(600));
                }
                self.request_redraw();
            }
            Ok(false) => {}
            Err(error) => eprintln!("battle command rejected: {error:?}"),
        }
    }

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

impl ApplicationHandler for CreatureBattleApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none()
            && let Err(error) = self.initialize(event_loop)
        {
            eprintln!("battle initialization failed: {error}");
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
            WindowEvent::KeyboardInput { event, .. } => self.handle_key(event),
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        if self.next_playback.is_some_and(|deadline| now >= deadline) {
            self.battle.advance_playback();
            self.request_redraw();
            self.next_playback = self
                .battle
                .has_pending_playback()
                .then_some(now + Duration::from_millis(600));
        }
        if let Some(deadline) = self.next_playback {
            event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(deadline));
        } else {
            event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
        }
    }
}

fn battle_viewport(target_size: PixelSize) -> Viewport {
    let cell_size = (target_size.width / CANVAS_WIDTH)
        .min(target_size.height / CANVAS_HEIGHT)
        .max(1);
    let width = i64::from(CANVAS_WIDTH) * i64::from(cell_size);
    let height = i64::from(CANVAS_HEIGHT) * i64::from(cell_size);
    Viewport::new(
        target_size,
        PixelOffset::new(
            ((i64::from(target_size.width) - width) / 2) as i32,
            ((i64::from(target_size.height) - height) / 2) as i32,
        ),
        PixelSize::new(cell_size, cell_size),
    )
    .expect("the battle viewport always has a positive integer cell size")
}

fn pixel_size(size: PhysicalSize<u32>) -> PixelSize {
    PixelSize::new(size.width, size.height)
}

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = CreatureBattleApp::new()?;
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use punctum_gpu::{PixelOffset, PixelSize};

    use super::battle_viewport;

    #[test]
    fn battle_viewport_uses_integer_scaling_and_centers_the_canvas() {
        let viewport = battle_viewport(PixelSize::new(960, 720));
        assert_eq!(viewport.cell_size, PixelSize::new(30, 30));
        assert_eq!(viewport.origin, PixelOffset::new(0, 0));

        let wide = battle_viewport(PixelSize::new(1000, 720));
        assert_eq!(wide.cell_size, PixelSize::new(30, 30));
        assert_eq!(wide.origin, PixelOffset::new(20, 0));
    }
}
