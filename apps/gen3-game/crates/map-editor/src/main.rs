mod assets;
mod controller;
mod layout;
mod model;
mod text;
mod view;

use std::{collections::BTreeSet, error::Error, fs, path::PathBuf, sync::Arc};

use assets::{default_project_path, load_assets, load_project};
use controller::{EditorController, PointerButton};
use map_project::{Collision, MapEventKind};
use map_render::AtomicTileCatalog;
use model::{EditorEffect, EditorIntent, EditorModel, EditorTool};
use punctum_gpu::{GpuAtlas, GpuClip, PixelSize, Rgba8, Viewport, plan_composite};
use punctum_wgpu::{GpuRuntime, PresentOutcome};
use text::EditorTextRenderer;
use view::{editor_viewport, project};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, ModifiersState, NamedKey},
    window::{Window, WindowId},
};

const CLEAR_COLOR: Rgba8 = Rgba8::new(17, 19, 22, 255);

struct MapEditorApp {
    project_path: PathBuf,
    model: EditorModel,
    controller: EditorController,
    atlas: GpuAtlas,
    catalog: AtomicTileCatalog,
    text_renderer: EditorTextRenderer,
    modifiers: ModifiersState,
    viewport: Viewport,
    window: Option<Arc<Window>>,
    runtime: Option<GpuRuntime<'static>>,
}

impl MapEditorApp {
    fn new() -> Result<Self, Box<dyn Error>> {
        let assets = load_assets()?;
        let project_path = std::env::args_os()
            .nth(1)
            .map(PathBuf::from)
            .unwrap_or_else(default_project_path);
        let project = load_project(&project_path, &assets.ids)?;
        Ok(Self {
            project_path,
            model: EditorModel::new(project, assets.ids),
            controller: EditorController::default(),
            atlas: assets.atlas,
            catalog: assets.catalog,
            text_renderer: EditorTextRenderer::new(),
            modifiers: ModifiersState::empty(),
            viewport: editor_viewport(PixelSize::new(1600, 950)),
            window: None,
            runtime: None,
        })
    }

    fn initialize(&mut self, event_loop: &ActiveEventLoop) -> Result<(), Box<dyn Error>> {
        let window = Arc::new(
            event_loop.create_window(
                Window::default_attributes()
                    .with_title("Gen3 地图编辑器")
                    .with_inner_size(LogicalSize::new(1600.0, 950.0)),
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
        self.viewport = editor_viewport(size);
        self.window = Some(window);
        self.runtime = Some(runtime);
        self.update_title();
        self.request_redraw();
        Ok(())
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let Some(target_size) = self.runtime.as_ref().map(GpuRuntime::surface_size) else {
            return;
        };
        let frame = match project(
            &self.model,
            &self.catalog,
            self.controller.hover,
            target_size,
        ) {
            Ok(frame) => frame,
            Err(error) => {
                self.model.report_error(error);
                self.update_title();
                return;
            }
        };
        self.viewport = frame.viewport;
        let plan = match plan_composite(
            &frame.surface,
            &frame.images,
            &self.atlas,
            u32::MAX,
            frame.viewport,
            GpuClip::Surface,
        ) {
            Ok(plan) => plan,
            Err(error) => {
                self.model.report_error(error);
                self.update_title();
                return;
            }
        };
        let (Some(window), Some(runtime)) = (&self.window, &mut self.runtime) else {
            return;
        };
        let mut text_result = Ok(());
        let renderer = &mut self.text_renderer;
        let result = runtime.present_plan_with_overlay(
            &plan,
            |device, queue, target, encoder, format, size| {
                text_result = renderer.encode(
                    &frame.labels,
                    frame.viewport,
                    device,
                    queue,
                    target,
                    encoder,
                    format,
                    size,
                );
            },
        );
        if let Err(error) = text_result {
            eprintln!("map editor text rendering failed: {error}");
            event_loop.exit();
            return;
        }
        match result {
            Ok(PresentOutcome::Reconfigured | PresentOutcome::SurfaceLost) => {
                runtime.resize(runtime.surface_size());
                window.request_redraw();
            }
            Ok(_) => {}
            Err(error) => {
                eprintln!("map editor presentation failed: {error}");
                event_loop.exit();
            }
        }
    }

    fn dispatch(&mut self, intent: EditorIntent) {
        match self.model.apply(intent) {
            Ok(EditorEffect::None) => {}
            Ok(EditorEffect::SaveRequested) => self.save(),
            Err(error) => self.model.report_error(error),
        }
        self.update_title();
        self.request_redraw();
    }

    fn save(&mut self) {
        let known = self
            .model
            .atomic_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let result = self
            .model
            .project
            .to_json_pretty(&known)
            .map_err(|error| Box::new(error) as Box<dyn Error>)
            .and_then(|json| {
                if let Some(parent) = self.project_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&self.project_path, json)?;
                Ok(())
            });
        match result {
            Ok(()) => self.model.mark_saved(),
            Err(error) => self.model.report_error(error),
        }
    }

    fn handle_key(&mut self, event: winit::event::KeyEvent) {
        if event.state != ElementState::Pressed {
            return;
        }
        let control = self.modifiers.control_key();
        let intent = match &event.logical_key {
            Key::Character(value) if control && value.eq_ignore_ascii_case("s") => {
                Some(EditorIntent::Save)
            }
            Key::Character(value) if control && value.eq_ignore_ascii_case("z") => {
                Some(EditorIntent::Undo)
            }
            Key::Character(value) if control && value.eq_ignore_ascii_case("y") => {
                Some(EditorIntent::Redo)
            }
            Key::Character(value) if value.eq_ignore_ascii_case("v") => {
                Some(EditorIntent::SelectTool(EditorTool::Visual))
            }
            Key::Character(value) if value.eq_ignore_ascii_case("a") => {
                Some(EditorIntent::AddLayer)
            }
            Key::Character(value) if value.eq_ignore_ascii_case("d") => {
                Some(EditorIntent::RemoveLayer)
            }
            Key::Named(NamedKey::Delete) => Some(EditorIntent::DeleteMaterial),
            Key::Character(value) if value == "1" => Some(EditorIntent::SelectTool(
                EditorTool::Collision(Collision::Walkable),
            )),
            Key::Character(value) if value == "2" => Some(EditorIntent::SelectTool(
                EditorTool::Collision(Collision::Blocked),
            )),
            Key::Character(value) if value == "3" => Some(EditorIntent::SelectTool(
                EditorTool::Event(Some(MapEventKind::Encounter)),
            )),
            Key::Character(value) if value == "4" => {
                Some(EditorIntent::SelectTool(EditorTool::Event(None)))
            }
            Key::Named(NamedKey::PageUp) => Some(EditorIntent::SelectAtomic(
                self.model.selected_atomic.saturating_sub(1),
            )),
            Key::Named(NamedKey::PageDown) => Some(EditorIntent::SelectAtomic(
                (self.model.selected_atomic + 1).min(self.model.atomic_ids.len() - 1),
            )),
            Key::Named(NamedKey::F1) => Some(EditorIntent::ToggleHelp),
            _ => None,
        };
        if let Some(intent) = intent {
            self.dispatch(intent);
        }
    }

    fn handle_cursor(&mut self, position: winit::dpi::PhysicalPosition<f64>) {
        if let Some(intent) =
            self.controller
                .move_cursor(position.x, position.y, self.viewport, &self.model)
        {
            self.dispatch(intent);
        } else {
            self.request_redraw();
        }
    }

    fn handle_mouse(&mut self, state: ElementState, button: MouseButton) {
        let button = match button {
            MouseButton::Left => PointerButton::Primary,
            MouseButton::Right => PointerButton::Secondary,
            _ => return,
        };
        match state {
            ElementState::Pressed => {
                if let Some(intent) = self.controller.press(button, &self.model) {
                    self.dispatch(intent);
                }
            }
            ElementState::Released => self.controller.release(button),
        }
    }

    fn handle_wheel(&mut self, delta: MouseScrollDelta) {
        let direction = match delta {
            MouseScrollDelta::LineDelta(_, y) => y.signum(),
            MouseScrollDelta::PixelDelta(position) => position.y.signum() as f32,
        };
        if direction == 0.0 || self.model.atomic_ids.is_empty() {
            return;
        }
        let page = self.model.selected_atomic / layout::ASSET_PAGE_SIZE;
        let maximum_page = self.model.atomic_ids.len().saturating_sub(1) / layout::ASSET_PAGE_SIZE;
        let next_page = if direction > 0.0 {
            page.saturating_sub(1)
        } else {
            (page + 1).min(maximum_page)
        };
        self.dispatch(EditorIntent::SelectAtomic(
            next_page * layout::ASSET_PAGE_SIZE,
        ));
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        if let Some(runtime) = &mut self.runtime {
            runtime.resize(pixel_size(size));
        }
        self.viewport = editor_viewport(pixel_size(size));
        self.request_redraw();
    }

    fn update_title(&self) {
        if let Some(window) = &self.window {
            let dirty = if self.model.dirty { " *" } else { "" };
            window.set_title(&format!(
                "Gen3 地图编辑器 - {}{}",
                self.model.project.id, dirty
            ));
        }
    }

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

impl ApplicationHandler for MapEditorApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none()
            && let Err(error) = self.initialize(event_loop)
        {
            eprintln!("map editor initialization failed: {error}");
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
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            WindowEvent::Resized(size) => self.resize(size),
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(window) = &self.window {
                    self.resize(window.inner_size());
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers.state(),
            WindowEvent::KeyboardInput { event, .. } => self.handle_key(event),
            WindowEvent::CursorMoved { position, .. } => self.handle_cursor(position),
            WindowEvent::CursorLeft { .. } => {
                self.controller.leave();
                self.request_redraw();
            }
            WindowEvent::MouseInput { state, button, .. } => self.handle_mouse(state, button),
            WindowEvent::MouseWheel { delta, .. } => self.handle_wheel(delta),
            _ => {}
        }
    }
}

fn pixel_size(size: PhysicalSize<u32>) -> PixelSize {
    PixelSize::new(size.width, size.height)
}

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = MapEditorApp::new()?;
    event_loop.run_app(&mut app)?;
    Ok(())
}
