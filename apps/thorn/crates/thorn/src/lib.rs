pub mod prelude {
    pub use thorn_core::{
        column, row, text, view, AppContext, Axis, BackendCapabilities, BackendError,
        BackendEventSource, BackendFeature, BackendInputEvent, BackendKey, BackendKeyEvent,
        BackendPresenter, BoundedInputQueue, Cell, CellAttrs, CellPatch, Color, ControlKeyAction,
        DefaultKeyMap, Direction, DirtyRegion, Element, EmacsTextKeyMap, FocusedControlKind,
        HostKind, HostNode, HostNodeId, InputShutdownSignal, InputThreadDriver, InputThreadStep,
        IntentContext, IntentMapper, IntentResolver, Key, KeyAction, KeyEvent, KeyEventKind,
        KeyIntent, KeyMap, KeyMapError, KeyMapLayer, KeyMapLayerKind, KeyMapResult, KeyModifiers,
        LayeredKeyMap, LayeredKeyMapResolution, LayoutNode, PaintAttrs, PaintColor,
        PaintPrimitive, PaintStyle, PlatformFallbackKeyMap, PresentedFrame, ReadOnlyNavigationKeyMap,
        Rect, RuntimeInput, Screen, ScreenPatch, Size, TextInputKeyMap, ThornApp,
        UnsupportedBackendFeature, VimNavigationKeyMap, WideCell,
    };
    pub use thorn_headless::{PaintSnapshot, ScreenSnapshot, TestRuntime};
    pub use thorn_runtime::{AppRuntime, FrameStats, NoopPerfSink, PerfSink};
    pub use thorn_terminal::TerminalRuntime;
}

pub use thorn_core::{column, row, text, view};

#[cfg(test)]
mod layering_tests {
    fn manifest(path: &str) -> &'static str {
        match path {
            "thorn-core" => include_str!("../../thorn-core/Cargo.toml"),
            "thorn-runtime" => include_str!("../../thorn-runtime/Cargo.toml"),
            "thorn-headless" => include_str!("../../thorn-headless/Cargo.toml"),
            "thorn-terminal" => include_str!("../../thorn-terminal/Cargo.toml"),
            "thorn-win32" => include_str!("../../thorn-win32/Cargo.toml"),
            "thorn" => include_str!("../Cargo.toml"),
            _ => "",
        }
    }

    fn dependency_names(manifest: &str) -> Vec<&str> {
        let mut in_dependencies = false;
        let mut names = Vec::new();
        for line in manifest.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                in_dependencies = trimmed == "[dependencies]";
                continue;
            }
            if in_dependencies {
                if let Some((name, _)) = trimmed.split_once('=') {
                    names.push(name.trim());
                }
            }
        }
        names
    }

    #[test]
    fn thorn_core_has_no_runtime_or_adapter_dependencies() {
        let deps = dependency_names(manifest("thorn-core"));

        assert!(deps.is_empty());
    }

    #[test]
    fn thorn_runtime_depends_only_on_core() {
        let deps = dependency_names(manifest("thorn-runtime"));

        assert_eq!(deps, vec!["thorn-core"]);
    }

    #[test]
    fn adapters_depend_only_through_runtime_or_core() {
        let headless = dependency_names(manifest("thorn-headless"));
        let terminal = dependency_names(manifest("thorn-terminal"));
        let win32 = dependency_names(manifest("thorn-win32"));

        assert_eq!(headless, vec!["thorn-core", "thorn-runtime"]);
        assert_eq!(terminal, vec!["thorn-core", "thorn-runtime"]);
        assert_eq!(win32, vec!["thorn-core"]);
    }

    #[test]
    fn facade_combines_layers_without_defining_core_dependencies() {
        let deps = dependency_names(manifest("thorn"));

        assert_eq!(
            deps,
            vec![
                "thorn-core",
                "thorn-headless",
                "thorn-runtime",
                "thorn-terminal"
            ]
        );
    }
}
