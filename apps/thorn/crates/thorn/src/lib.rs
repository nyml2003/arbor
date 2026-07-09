mod facade;

pub mod prelude {
    pub use crate::facade::{
        app, AppBuilder, AppBuilderWithUpdate, ClosureApp, DefaultIntentMapper,
    };
    pub use thorn_core::{
        clip, column, layer, row, scroll_view, text, view, AppContext, Axis, BackendCapabilities,
        BackendError, BackendEventSource, BackendFeature, BackendInputEvent, BackendKey,
        BackendKeyEvent, BackendPresenter, BoundedInputQueue, Cell, CellAttrs, CellPatch, Color,
        ControlKeyAction, DefaultKeyMap, Direction, DirtyKind, DirtyRegion, Element,
        EmacsTextKeyMap, FocusedControlKind, FrameInvalidation, HostKind, HostNode, HostNodeId,
        InputShutdownSignal, InputThreadDriver, InputThreadStep, IntentContext, IntentMapper,
        IntentResolver, Key, KeyAction, KeyEvent, KeyEventKind, KeyIntent, KeyMap, KeyMapError,
        KeyMapLayer, KeyMapLayerKind, KeyMapResult, KeyModifiers, LayeredKeyMap,
        LayeredKeyMapResolution, LayoutNode, PaintAttrs, PaintColor, PaintPrimitive, PaintStyle,
        PlatformFallbackKeyMap, PresentedFrame, ReadOnlyNavigationKeyMap, Rect, RuntimeInput,
        Screen, ScreenPatch, Size, TextInputKeyMap, Theme, ThornApp, UnsupportedBackendFeature,
        VimNavigationKeyMap, WideCell,
    };
    pub use thorn_headless::{PaintSnapshot, ScreenSnapshot, TestRuntime};
    pub use thorn_runtime::{AppRuntime, FrameStats, NoopPerfSink, PerfSink};
    pub use thorn_terminal::TerminalRuntime;
}

pub use facade::{app, AppBuilder, AppBuilderWithUpdate, ClosureApp, DefaultIntentMapper};
pub use thorn_core::{
    clip, column, layer, row, scroll_view, text, view, AppContext, DirtyKind, FrameInvalidation,
    Theme,
};

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use thorn_core::{
        render_to_screen, AppContext, KeyAction, KeyEvent, KeyIntent, KeyMap, KeyMapLayer,
        KeyMapLayerKind, LayeredKeyMap,
    };
    use thorn_runtime::AppRuntime;
    use toml::Value;

    fn manifest(path: &str) -> &'static str {
        match path {
            "workspace" => include_str!("../../../Cargo.toml"),
            "thorn-core" => include_str!("../../thorn-core/Cargo.toml"),
            "thorn-runtime" => include_str!("../../thorn-runtime/Cargo.toml"),
            "thorn-headless" => include_str!("../../thorn-headless/Cargo.toml"),
            "thorn-terminal" => include_str!("../../thorn-terminal/Cargo.toml"),
            "thorn-win32" => include_str!("../../thorn-win32/Cargo.toml"),
            "thorn" => include_str!("../Cargo.toml"),
            _ => "",
        }
    }

    fn manifest_value(path: &str) -> Value {
        toml::from_str(manifest(path)).unwrap_or_else(|err| {
            panic!("failed to parse manifest {path}: {err}");
        })
    }

    fn dependency_names(path: &str) -> BTreeSet<String> {
        fn collect_dependency_names(
            table: Option<&toml::map::Map<String, Value>>,
        ) -> BTreeSet<String> {
            table
                .into_iter()
                .flat_map(|entries| entries.keys().cloned())
                .filter(|name| name.starts_with("thorn"))
                .collect()
        }

        fn extend_target_dependencies(
            names: &mut BTreeSet<String>,
            target_table: Option<&toml::map::Map<String, Value>>,
        ) {
            for target in target_table
                .into_iter()
                .flat_map(|entries| entries.values())
            {
                let Some(target_entries) = target.as_table() else {
                    continue;
                };

                names.extend(collect_dependency_names(
                    target_entries.get("dependencies").and_then(Value::as_table),
                ));
                names.extend(collect_dependency_names(
                    target_entries
                        .get("dev-dependencies")
                        .and_then(Value::as_table),
                ));
                names.extend(collect_dependency_names(
                    target_entries
                        .get("build-dependencies")
                        .and_then(Value::as_table),
                ));
            }
        }

        let manifest = manifest_value(path);
        let mut names = BTreeSet::new();
        let Some(table) = manifest.as_table() else {
            panic!("manifest {path} is not a TOML table");
        };

        names.extend(collect_dependency_names(
            table.get("dependencies").and_then(Value::as_table),
        ));
        names.extend(collect_dependency_names(
            table.get("dev-dependencies").and_then(Value::as_table),
        ));
        names.extend(collect_dependency_names(
            table.get("build-dependencies").and_then(Value::as_table),
        ));
        extend_target_dependencies(&mut names, table.get("target").and_then(Value::as_table));

        names
    }

    fn workspace_members() -> Vec<String> {
        let manifest = manifest_value("workspace");
        let Some(workspace) = manifest.get("workspace").and_then(Value::as_table) else {
            panic!("workspace manifest is missing [workspace]");
        };
        let Some(members) = workspace.get("members").and_then(Value::as_array) else {
            panic!("workspace manifest is missing workspace.members");
        };

        members
            .iter()
            .map(|member| {
                member
                    .as_str()
                    .unwrap_or_else(|| panic!("workspace member is not a string: {member:?}"))
                    .to_owned()
            })
            .collect()
    }

    #[test]
    fn workspace_includes_thep_0013_mvp_crates() {
        let members = workspace_members();
        let expected_mvp_members = BTreeSet::from([
            "crates/thorn-core".to_owned(),
            "crates/thorn-runtime".to_owned(),
            "crates/thorn-headless".to_owned(),
            "crates/thorn-terminal".to_owned(),
            "crates/thorn".to_owned(),
        ]);
        let member_set = members.iter().cloned().collect::<BTreeSet<_>>();

        assert!(
            expected_mvp_members.is_subset(&member_set),
            "workspace members must include THEP-0013 MVP crates {expected_mvp_members:?}, got {members:?}"
        );
    }

    #[test]
    fn thorn_core_manifest_has_no_runtime_adapter_or_facade_dependencies() {
        let deps = dependency_names("thorn-core");

        assert!(deps.is_empty(), "thorn-core dependencies were {deps:?}");
    }

    #[test]
    fn thorn_runtime_manifest_depends_only_on_core() {
        let deps = dependency_names("thorn-runtime");

        assert_eq!(
            deps,
            BTreeSet::from(["thorn-core".to_owned()]),
            "thorn-runtime dependencies were {deps:?}"
        );
        assert!(!deps.contains("thorn"));
        assert!(!deps.contains("thorn-headless"));
        assert!(!deps.contains("thorn-terminal"));
        assert!(!deps.contains("thorn-win32"));
    }

    #[test]
    fn thep_0013_mvp_adapter_manifests_only_depend_on_allowed_lower_layers() {
        let allowed = BTreeSet::from(["thorn-core".to_owned(), "thorn-runtime".to_owned()]);
        let headless = dependency_names("thorn-headless");
        let terminal = dependency_names("thorn-terminal");

        assert_eq!(
            headless, allowed,
            "thorn-headless dependencies were {headless:?}"
        );
        assert_eq!(
            terminal, allowed,
            "thorn-terminal dependencies were {terminal:?}"
        );

        for (crate_name, deps) in [("thorn-headless", &headless), ("thorn-terminal", &terminal)] {
            assert!(
                !deps.contains("thorn"),
                "{crate_name} must not depend on facade crate thorn"
            );
        }
        assert!(!headless.contains("thorn-terminal"));
        assert!(!headless.contains("thorn-win32"));
        assert!(!terminal.contains("thorn-headless"));
        assert!(!terminal.contains("thorn-win32"));
    }

    #[test]
    fn extra_adapter_win32_manifest_stays_below_facade_if_present() {
        let win32 = dependency_names("thorn-win32");

        assert_eq!(
            win32,
            BTreeSet::from(["thorn-core".to_owned()]),
            "thorn-win32 dependencies were {win32:?}"
        );
        assert!(!win32.contains("thorn"));
        assert!(!win32.contains("thorn-headless"));
        assert!(!win32.contains("thorn-terminal"));
    }

    #[test]
    fn facade_manifest_can_depend_on_core_runtime_and_adapters_only() {
        let deps = dependency_names("thorn");
        let allowed = BTreeSet::from([
            "thorn-core".to_owned(),
            "thorn-headless".to_owned(),
            "thorn-runtime".to_owned(),
            "thorn-terminal".to_owned(),
            "thorn-win32".to_owned(),
        ]);

        assert_eq!(
            deps.intersection(&allowed)
                .cloned()
                .collect::<BTreeSet<_>>(),
            deps,
            "thorn facade dependencies were {deps:?}"
        );
        assert!(deps.contains("thorn-core"));
        assert!(deps.contains("thorn-runtime"));
        assert!(deps.contains("thorn-headless"));
        assert!(deps.contains("thorn-terminal"));
    }

    #[test]
    fn builder_style_app_implements_runtime_contract_and_renders() {
        struct IncrementIntentMapper;

        impl thorn_core::IntentMapper<i32> for IncrementIntentMapper {
            fn map_intent(&self, intent: KeyIntent) -> Option<KeyAction<i32>> {
                match intent {
                    KeyIntent::RequestQuit => Some(KeyAction::RuntimeQuit),
                    KeyIntent::App("increment") => Some(KeyAction::App(1)),
                    _ => None,
                }
            }
        }

        let mut app = app(0)
            .update(|state, action: i32, ctx: &mut AppContext<i32>| {
                *state += action;
                ctx.request_render();
            })
            .view(|state| text(format!("count: {state}")));

        let mut ctx = AppContext::new();
        thorn_core::ThornApp::update(&mut app, 2, &mut ctx);
        assert_eq!(
            ctx.take_invalidation(),
            Some(FrameInvalidation::new(DirtyKind::Render))
        );

        let screen = render_to_screen(
            &thorn_core::ThornApp::view(&app),
            thorn_core::Size::new(12, 1),
        );
        assert_eq!(screen.to_plain_text(), "count: 2");

        let mut runtime = AppRuntime::with_resolver(app, IncrementIntentMapper)
            .keymap(KeyMap::new().bind(KeyEvent::char('n'), KeyIntent::App("increment")))
            .size(12, 1);
        runtime.render_frame();
        runtime.send_key('n');

        assert!(runtime.render_frame().to_plain_text().contains("count: 3"));
    }

    #[test]
    fn builder_run_with_io_uses_terminal_runtime_without_custom_loop() {
        let mut output = Vec::new();

        app(0u8)
            .update(|_state, _action: (), _ctx: &mut AppContext<()>| {})
            .view(|_state| text("builder"))
            .run_with_io(&b"q\n"[..], &mut output)
            .unwrap();

        assert!(String::from_utf8(output).unwrap().contains("builder"));
    }

    struct BuilderIntentMapper;

    impl thorn_core::IntentMapper<&'static str> for BuilderIntentMapper {
        fn map_intent(&self, intent: KeyIntent) -> Option<KeyAction<&'static str>> {
            match intent {
                KeyIntent::RequestQuit => Some(KeyAction::RuntimeQuit),
                KeyIntent::App(action) => Some(KeyAction::App(action)),
                _ => None,
            }
        }
    }

    #[test]
    fn facade_builder_keymap_can_dispatch_app_action() {
        let mut runtime = app(0)
            .update(
                |state, action: &'static str, ctx: &mut AppContext<&'static str>| {
                    if action == "increment" {
                        *state += 1;
                        ctx.request_render();
                    }
                },
            )
            .view(|state| text(format!("count: {state}")))
            .keymap(KeyMap::new().bind(KeyEvent::char('n'), KeyIntent::App("increment")))
            .into_test_runtime_with_mapper(BuilderIntentMapper)
            .size(12, 1);

        runtime.render_frame();
        runtime.send_key('n');
        runtime.render_frame();

        assert!(runtime.screen().to_plain_text().contains("count: 1"));
    }

    #[test]
    fn facade_builder_mode_keymap_sets_runtime_mode_context() {
        let runtime = app("idle")
            .update(|_state, _action: &'static str, _ctx: &mut AppContext<&'static str>| {})
            .view(|state| text(*state))
            .mode_keymap(
                "game",
                KeyMap::new().bind(KeyEvent::char('q'), KeyIntent::App("cast_ultimate")),
            )
            .into_runtime_with_mapper(BuilderIntentMapper);

        assert_eq!(runtime.intent_context().active_mode, Some("game"));
    }

    #[test]
    fn facade_builder_layered_keymap_can_override_default_quit_binding() {
        let mut runtime = app("idle")
            .update(
                |state, action: &'static str, ctx: &mut AppContext<&'static str>| match action {
                    "cast_ultimate" => {
                        *state = "ultimate";
                        ctx.request_render();
                    }
                    _ => {}
                },
            )
            .view(|state| text(*state))
            .layered_keymap(LayeredKeyMap::default().with_layer(KeyMapLayer::with_kind(
                "app:game",
                KeyMapLayerKind::App,
                KeyMap::new().bind(KeyEvent::char('q'), KeyIntent::App("cast_ultimate")),
            )))
            .into_test_runtime_with_mapper(BuilderIntentMapper)
            .size(12, 1);

        runtime.render_frame();
        runtime.send_key('q');
        runtime.render_frame();

        assert!(runtime.is_running());
        assert!(runtime.screen().to_plain_text().contains("ultimate"));
    }
}
