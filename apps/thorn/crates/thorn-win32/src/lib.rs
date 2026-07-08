//! Win32 adapter contract for Thorn.
//!
//! This crate intentionally starts with a dry-run presenter instead of real
//! HWND/message-loop code. It proves the adapter boundary while keeping
//! `thorn-core` backend-independent.

use thorn_core::{
    BackendCapabilities, BackendError, BackendFeature, BackendPresenter, PresentedFrame,
    ScreenPatch,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Win32BackendConfig {
    pub no_activate: bool,
    pub topmost: bool,
    pub tool_window: bool,
}

impl Default for Win32BackendConfig {
    fn default() -> Self {
        Self {
            no_activate: true,
            topmost: true,
            tool_window: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Win32BackendPlan {
    pub config: Win32BackendConfig,
    pub capabilities: BackendCapabilities,
}

pub fn plan_win32_backend(config: Win32BackendConfig) -> Win32BackendPlan {
    Win32BackendPlan {
        config,
        capabilities: BackendCapabilities::new(vec![
            BackendFeature::Text,
            BackendFeature::FillRect,
            BackendFeature::Border,
            BackendFeature::Clip,
        ]),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Win32BackendKind {
    DryRunContract,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Win32DryRunPresenter {
    kind: Win32BackendKind,
    plan: Win32BackendPlan,
    presented_frames: usize,
}

impl Win32DryRunPresenter {
    pub fn new(config: Win32BackendConfig) -> Self {
        Self {
            kind: Win32BackendKind::DryRunContract,
            plan: plan_win32_backend(config),
            presented_frames: 0,
        }
    }

    pub fn kind(&self) -> Win32BackendKind {
        self.kind.clone()
    }

    pub fn plan(&self) -> &Win32BackendPlan {
        &self.plan
    }

    pub fn presented_frames(&self) -> usize {
        self.presented_frames
    }
}

impl BackendPresenter for Win32DryRunPresenter {
    fn capabilities(&self) -> &BackendCapabilities {
        &self.plan.capabilities
    }

    fn present(&mut self, patch: &ScreenPatch) -> Result<PresentedFrame, BackendError> {
        self.capabilities().require(BackendFeature::Text)?;
        self.presented_frames += 1;
        Ok(PresentedFrame {
            size: patch.size,
            full: patch.full,
            changed_cells: patch.cells.len(),
            output_summary: format!(
                "win32-dry-run frame={} cells={} full={}",
                self.presented_frames,
                patch.cells.len(),
                patch.full
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use thorn_core::{render_to_screen, text, BackendFeature, Size};

    #[test]
    fn default_plan_matches_keydock_window_policy() {
        let plan = plan_win32_backend(Win32BackendConfig::default());

        assert!(plan.config.no_activate);
        assert!(plan.config.topmost);
        assert!(plan.config.tool_window);
        assert!(plan.capabilities.supports(BackendFeature::Text));
        assert!(plan.capabilities.supports(BackendFeature::FillRect));
        assert!(!plan.capabilities.supports(BackendFeature::TextInput));
    }

    #[test]
    fn dry_run_presenter_accepts_core_screen_patch_without_window() {
        let screen = render_to_screen(&text::<()>("ready"), Size::new(8, 1));
        let patch = screen.full_patch();
        let mut presenter = Win32DryRunPresenter::new(Win32BackendConfig::default());

        let frame = presenter.present(&patch).unwrap();

        assert_eq!(presenter.kind(), Win32BackendKind::DryRunContract);
        assert_eq!(presenter.presented_frames(), 1);
        assert_eq!(frame.size, Size::new(8, 1));
        assert!(frame.full);
        assert_eq!(frame.changed_cells, 8);
        assert_eq!(
            frame.output_summary,
            "win32-dry-run frame=1 cells=8 full=true"
        );
    }

    #[test]
    fn unsupported_control_features_remain_structured_errors() {
        let presenter = Win32DryRunPresenter::new(Win32BackendConfig::default());

        assert_eq!(
            presenter.capabilities().require(BackendFeature::TextInput),
            Err(BackendError::UnsupportedFeature(
                thorn_core::UnsupportedBackendFeature {
                    feature: BackendFeature::TextInput,
                }
            ))
        );
    }
}
