use crate::{ScreenPatch, Size};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendFeature {
    Text,
    FillRect,
    Border,
    Cursor,
    Clip,
    Layer,
    TextInput,
    ScrollView,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendCapabilities {
    features: Vec<BackendFeature>,
}

impl BackendCapabilities {
    pub fn new(features: impl Into<Vec<BackendFeature>>) -> Self {
        Self {
            features: features.into(),
        }
    }

    pub fn text_only() -> Self {
        Self::new(vec![BackendFeature::Text])
    }

    pub fn supports(&self, feature: BackendFeature) -> bool {
        self.features.contains(&feature)
    }

    pub fn require(&self, feature: BackendFeature) -> Result<(), BackendError> {
        self.supports(feature)
            .then_some(())
            .ok_or(BackendError::UnsupportedFeature(
                UnsupportedBackendFeature { feature },
            ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedBackendFeature {
    pub feature: BackendFeature,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendError {
    UnsupportedFeature(UnsupportedBackendFeature),
    PresentationFailed { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentedFrame {
    pub size: Size,
    pub full: bool,
    pub changed_cells: usize,
    pub output_summary: String,
}

pub trait BackendPresenter {
    fn capabilities(&self) -> &BackendCapabilities;
    fn present(&mut self, patch: &ScreenPatch) -> Result<PresentedFrame, BackendError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Cell, CellPatch, DirtyRegion, Rect};

    struct MemoryPresenter {
        capabilities: BackendCapabilities,
    }

    impl BackendPresenter for MemoryPresenter {
        fn capabilities(&self) -> &BackendCapabilities {
            &self.capabilities
        }

        fn present(&mut self, patch: &ScreenPatch) -> Result<PresentedFrame, BackendError> {
            self.capabilities.require(BackendFeature::Text)?;
            Ok(PresentedFrame {
                size: patch.size,
                full: patch.full,
                changed_cells: patch.cells.len(),
                output_summary: format!("cells={}", patch.cells.len()),
            })
        }
    }

    #[test]
    fn backend_capabilities_report_supported_features() {
        let capabilities = BackendCapabilities::text_only();

        assert!(capabilities.supports(BackendFeature::Text));
        assert!(!capabilities.supports(BackendFeature::TextInput));
    }

    #[test]
    fn unsupported_backend_feature_is_structured() {
        let capabilities = BackendCapabilities::text_only();

        assert_eq!(
            capabilities.require(BackendFeature::TextInput),
            Err(BackendError::UnsupportedFeature(
                UnsupportedBackendFeature {
                    feature: BackendFeature::TextInput,
                }
            ))
        );
    }

    #[test]
    fn backend_presenter_returns_presented_frame_summary() {
        let mut presenter = MemoryPresenter {
            capabilities: BackendCapabilities::text_only(),
        };
        let patch = ScreenPatch {
            size: Size::new(2, 1),
            full: false,
            regions: vec![DirtyRegion {
                rect: Rect::new(0, 0, 1, 1),
            }],
            cells: vec![CellPatch {
                x: 0,
                y: 0,
                cell: Cell::new('x'),
            }],
        };

        let frame = presenter.present(&patch).unwrap();

        assert_eq!(frame.changed_cells, 1);
        assert_eq!(frame.output_summary, "cells=1");
    }
}
