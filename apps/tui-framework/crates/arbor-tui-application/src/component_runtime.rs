use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

use arbor_tui_domain::focus::{mount_tree, unmount_tree};
use arbor_tui_domain::identity::{DirtyKind, NodeIdentity};
use arbor_tui_domain::signal::{ReadSignal, Signal, SignalChange, SignalDep};
use arbor_tui_domain::widget::WidgetNode;
use arbor_tui_domain::PropsRevision;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ComponentStateModel {
    Stateless,
    Controlled,
    Uncontrolled,
    Custom(u64),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ComponentSpec {
    pub identity: NodeIdentity,
    pub component_type: &'static str,
    pub props_revision: PropsRevision,
    pub signal_deps: Vec<SignalDep>,
    pub state_model: ComponentStateModel,
    pub update_dirty: DirtyKind,
    pub children: Vec<ComponentSpec>,
}

impl ComponentSpec {
    pub fn new(identity: NodeIdentity, component_type: &'static str) -> Self {
        Self {
            identity,
            component_type,
            props_revision: PropsRevision::ZERO,
            signal_deps: Vec::new(),
            state_model: ComponentStateModel::Stateless,
            update_dirty: DirtyKind::Full,
            children: Vec::new(),
        }
    }

    pub fn props_revision(mut self, revision: PropsRevision) -> Self {
        self.props_revision = revision;
        self
    }

    pub fn signal_deps(mut self, deps: Vec<SignalDep>) -> Self {
        self.signal_deps = deps;
        self
    }

    pub fn state_model(mut self, state_model: ComponentStateModel) -> Self {
        self.state_model = state_model;
        self
    }

    pub fn update_dirty(mut self, dirty: DirtyKind) -> Self {
        self.update_dirty = dirty;
        self
    }

    pub fn children(mut self, children: Vec<ComponentSpec>) -> Self {
        self.children = children;
        self
    }

    fn is_compatible_with(&self, other: &Self) -> bool {
        self.identity == other.identity
            && self.component_type == other.component_type
            && self.state_model == other.state_model
    }

    fn update_can_skip(&self, other: &Self) -> bool {
        self.props_revision == other.props_revision && self.signal_deps == other.signal_deps
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ComponentCachePolicy {
    RuntimeManaged,
    Disabled,
}

pub trait ComponentRuntime {
    fn mount(&mut self, _spec: &ComponentSpec) {}

    fn update(&mut self, _old: &ComponentSpec, new: &ComponentSpec) -> DirtyKind {
        new.update_dirty
    }

    fn unmount(&mut self, _spec: &ComponentSpec) {}

    fn cache_policy(&self) -> ComponentCachePolicy {
        ComponentCachePolicy::RuntimeManaged
    }
}

#[derive(Default, Debug)]
pub struct ComponentLifecycleReport {
    pub reused: usize,
    pub replaced: usize,
    pub mounted: usize,
    pub updated: usize,
    pub skipped_updates: usize,
    pub unmounted: usize,
    pub dirty: Vec<(NodeIdentity, DirtyKind)>,
}

impl ComponentLifecycleReport {
    fn merge(&mut self, other: ComponentLifecycleReport) {
        self.reused += other.reused;
        self.replaced += other.replaced;
        self.mounted += other.mounted;
        self.updated += other.updated;
        self.skipped_updates += other.skipped_updates;
        self.unmounted += other.unmounted;
        self.dirty.extend(other.dirty);
    }
}

pub struct RetainedComponentTree {
    root: Option<ComponentInstance>,
}

impl Default for RetainedComponentTree {
    fn default() -> Self {
        Self::new()
    }
}

impl RetainedComponentTree {
    pub fn new() -> Self {
        Self { root: None }
    }

    pub fn reconcile(
        &mut self,
        next: ComponentSpec,
        create: &mut impl FnMut(&ComponentSpec) -> Box<dyn ComponentRuntime>,
    ) -> ComponentLifecycleReport {
        match self.root.take() {
            Some(instance) => {
                let (instance, report) = reconcile_instance(instance, next, create);
                self.root = Some(instance);
                report
            }
            None => {
                let (instance, report) = mount_instance(next, create);
                self.root = Some(instance);
                report
            }
        }
    }

    pub fn unmount(&mut self) -> ComponentLifecycleReport {
        let Some(instance) = self.root.take() else {
            return ComponentLifecycleReport::default();
        };
        let mut report = ComponentLifecycleReport::default();
        unmount_instance(instance, &mut report);
        report
    }
}

struct ComponentInstance {
    spec: ComponentSpec,
    runtime: Box<dyn ComponentRuntime>,
    children: Vec<ComponentInstance>,
}

fn mount_instance(
    spec: ComponentSpec,
    create: &mut impl FnMut(&ComponentSpec) -> Box<dyn ComponentRuntime>,
) -> (ComponentInstance, ComponentLifecycleReport) {
    let mut report = ComponentLifecycleReport::default();
    let mut runtime = create(&spec);
    runtime.mount(&spec);
    report.mounted += 1;

    let children = spec
        .children
        .clone()
        .into_iter()
        .map(|child| {
            let (instance, child_report) = mount_instance(child, create);
            report.merge(child_report);
            instance
        })
        .collect();

    (
        ComponentInstance {
            spec,
            runtime,
            children,
        },
        report,
    )
}

fn reconcile_instance(
    mut current: ComponentInstance,
    next: ComponentSpec,
    create: &mut impl FnMut(&ComponentSpec) -> Box<dyn ComponentRuntime>,
) -> (ComponentInstance, ComponentLifecycleReport) {
    if !current.spec.is_compatible_with(&next) {
        let mut report = ComponentLifecycleReport {
            replaced: 1,
            ..Default::default()
        };
        unmount_instance(current, &mut report);
        let (instance, mounted) = mount_instance(next, create);
        report.merge(mounted);
        return (instance, report);
    }

    let mut report = ComponentLifecycleReport {
        reused: 1,
        ..Default::default()
    };

    if current.spec.update_can_skip(&next) {
        report.skipped_updates += 1;
    } else {
        let dirty = current.runtime.update(&current.spec, &next);
        report.updated += 1;
        report.dirty.push((next.identity.clone(), dirty));
    }

    current.children =
        reconcile_children(current.children, next.children.clone(), create, &mut report);
    current.spec = next;
    (current, report)
}

fn reconcile_children(
    current: Vec<ComponentInstance>,
    next: Vec<ComponentSpec>,
    create: &mut impl FnMut(&ComponentSpec) -> Box<dyn ComponentRuntime>,
    report: &mut ComponentLifecycleReport,
) -> Vec<ComponentInstance> {
    let mut by_identity = current
        .into_iter()
        .map(|instance| (instance.spec.identity.clone(), instance))
        .collect::<HashMap<_, _>>();
    let mut reconciled = Vec::with_capacity(next.len());

    for next_child in next {
        if let Some(current_child) = by_identity.remove(&next_child.identity) {
            let (instance, child_report) = reconcile_instance(current_child, next_child, create);
            report.merge(child_report);
            reconciled.push(instance);
        } else {
            let (instance, child_report) = mount_instance(next_child, create);
            report.merge(child_report);
            reconciled.push(instance);
        }
    }

    for (_, removed) in by_identity {
        unmount_instance(removed, report);
    }

    reconciled
}

fn unmount_instance(mut instance: ComponentInstance, report: &mut ComponentLifecycleReport) {
    for child in instance.children.drain(..) {
        unmount_instance(child, report);
    }
    instance.runtime.unmount(&instance.spec);
    report.unmounted += 1;
}

pub struct LegacyWidgetAdapter {
    widget: WidgetNode,
    conservative_dirty: DirtyKind,
}

impl LegacyWidgetAdapter {
    pub fn new(widget: WidgetNode) -> Self {
        Self {
            widget,
            conservative_dirty: DirtyKind::Full,
        }
    }

    pub fn conservative_dirty(mut self, dirty: DirtyKind) -> Self {
        self.conservative_dirty = dirty;
        self
    }
}

impl ComponentRuntime for LegacyWidgetAdapter {
    fn mount(&mut self, _spec: &ComponentSpec) {
        mount_tree(&mut self.widget);
    }

    fn update(&mut self, _old: &ComponentSpec, _new: &ComponentSpec) -> DirtyKind {
        self.conservative_dirty
    }

    fn unmount(&mut self, _spec: &ComponentSpec) {
        unmount_tree(&mut self.widget);
    }

    fn cache_policy(&self) -> ComponentCachePolicy {
        ComponentCachePolicy::Disabled
    }
}

pub struct ComponentOwnedSignal<T: Clone + PartialEq> {
    signal: Signal<T>,
    alive: Rc<Cell<bool>>,
}

impl<T: Clone + PartialEq> ComponentOwnedSignal<T> {
    pub fn new(initial: T) -> Self {
        Self {
            signal: Signal::new(initial),
            alive: Rc::new(Cell::new(true)),
        }
    }

    pub fn read_only(&self) -> ReadSignal<T> {
        self.signal.read_only()
    }

    pub fn write_handle(&self) -> ComponentOwnedSignalWrite<T> {
        ComponentOwnedSignalWrite {
            signal: self.signal.read_only(),
            write_signal: SignalWriteHandle {
                signal: self.signal.clone(),
                alive: self.alive.clone(),
            },
        }
    }
}

impl<T: Clone + PartialEq> Drop for ComponentOwnedSignal<T> {
    fn drop(&mut self) {
        self.alive.set(false);
    }
}

pub struct ComponentOwnedSignalWrite<T: Clone + PartialEq> {
    signal: ReadSignal<T>,
    write_signal: SignalWriteHandle<T>,
}

impl<T: Clone + PartialEq> ComponentOwnedSignalWrite<T> {
    pub fn read_only(&self) -> ReadSignal<T> {
        self.signal.clone()
    }

    pub fn set_collect(&self, value: T) -> Option<SignalChange> {
        self.write_signal.set_collect(value)
    }
}

struct SignalWriteHandle<T: Clone + PartialEq> {
    signal: Signal<T>,
    alive: Rc<Cell<bool>>,
}

impl<T: Clone + PartialEq> SignalWriteHandle<T> {
    fn set_collect(&self, value: T) -> Option<SignalChange> {
        assert!(
            self.alive.get(),
            "component-owned signal was written after its owner was dropped"
        );
        self.signal.set_collect(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arbor_tui_domain::layout::LayoutProps;
    use arbor_tui_domain::widget::{Widget, WidgetId};
    use arbor_tui_domain::WidgetKey;
    use std::cell::RefCell;

    #[derive(Clone)]
    struct Recorder(Rc<RefCell<Vec<String>>>);

    impl Recorder {
        fn push(&self, event: impl Into<String>) {
            self.0.borrow_mut().push(event.into());
        }

        fn events(&self) -> Vec<String> {
            self.0.borrow().clone()
        }
    }

    struct RecordingRuntime {
        recorder: Recorder,
    }

    impl ComponentRuntime for RecordingRuntime {
        fn mount(&mut self, spec: &ComponentSpec) {
            self.recorder.push(format!("mount:{}", spec.component_type));
        }

        fn update(&mut self, old: &ComponentSpec, new: &ComponentSpec) -> DirtyKind {
            self.recorder.push(format!(
                "update:{}:{}->{}",
                new.component_type,
                old.props_revision.get(),
                new.props_revision.get()
            ));
            new.update_dirty
        }

        fn unmount(&mut self, spec: &ComponentSpec) {
            self.recorder
                .push(format!("unmount:{}", spec.component_type));
        }
    }

    fn keyed(key: &str, component_type: &'static str) -> ComponentSpec {
        ComponentSpec::new(NodeIdentity::Keyed(WidgetKey::new(key)), component_type)
    }

    #[test]
    fn lifecycle_mounts_parent_before_children_and_unmounts_children_first() {
        let recorder = Recorder(Rc::new(RefCell::new(Vec::new())));
        let mut tree = RetainedComponentTree::new();
        let root = keyed("root", "root").children(vec![keyed("child", "child")]);
        let mut create = |_: &ComponentSpec| {
            Box::new(RecordingRuntime {
                recorder: recorder.clone(),
            }) as Box<dyn ComponentRuntime>
        };

        tree.reconcile(root, &mut create);
        tree.unmount();

        assert_eq!(
            recorder.events(),
            vec!["mount:root", "mount:child", "unmount:child", "unmount:root"]
        );
    }

    #[test]
    fn same_identity_type_and_state_reuses_runtime_and_skips_same_revision() {
        let recorder = Recorder(Rc::new(RefCell::new(Vec::new())));
        let mut tree = RetainedComponentTree::new();
        let mut create = |_: &ComponentSpec| {
            Box::new(RecordingRuntime {
                recorder: recorder.clone(),
            }) as Box<dyn ComponentRuntime>
        };

        tree.reconcile(keyed("root", "text"), &mut create);
        let report = tree.reconcile(keyed("root", "text"), &mut create);

        assert_eq!(report.reused, 1);
        assert_eq!(report.skipped_updates, 1);
        assert_eq!(recorder.events(), vec!["mount:text"]);
    }

    #[test]
    fn changed_revision_calls_update_and_records_dirty() {
        let recorder = Recorder(Rc::new(RefCell::new(Vec::new())));
        let mut tree = RetainedComponentTree::new();
        let mut create = |_: &ComponentSpec| {
            Box::new(RecordingRuntime {
                recorder: recorder.clone(),
            }) as Box<dyn ComponentRuntime>
        };

        tree.reconcile(keyed("root", "text"), &mut create);
        let report = tree.reconcile(
            keyed("root", "text")
                .props_revision(PropsRevision::new(2))
                .update_dirty(DirtyKind::Layout),
            &mut create,
        );

        assert_eq!(report.updated, 1);
        assert_eq!(
            report.dirty,
            vec![(
                NodeIdentity::Keyed(WidgetKey::new("root")),
                DirtyKind::Layout
            )]
        );
        assert_eq!(recorder.events(), vec!["mount:text", "update:text:0->2"]);
    }

    #[test]
    fn type_change_replaces_instance() {
        let recorder = Recorder(Rc::new(RefCell::new(Vec::new())));
        let mut tree = RetainedComponentTree::new();
        let mut create = |_: &ComponentSpec| {
            Box::new(RecordingRuntime {
                recorder: recorder.clone(),
            }) as Box<dyn ComponentRuntime>
        };

        tree.reconcile(keyed("root", "text"), &mut create);
        let report = tree.reconcile(keyed("root", "button"), &mut create);

        assert_eq!(report.replaced, 1);
        assert_eq!(
            recorder.events(),
            vec!["mount:text", "unmount:text", "mount:button"]
        );
    }

    #[test]
    fn controlled_uncontrolled_switch_replaces_instance() {
        let recorder = Recorder(Rc::new(RefCell::new(Vec::new())));
        let mut tree = RetainedComponentTree::new();
        let mut create = |_: &ComponentSpec| {
            Box::new(RecordingRuntime {
                recorder: recorder.clone(),
            }) as Box<dyn ComponentRuntime>
        };

        tree.reconcile(
            keyed("input", "input").state_model(ComponentStateModel::Uncontrolled),
            &mut create,
        );
        let report = tree.reconcile(
            keyed("input", "input").state_model(ComponentStateModel::Controlled),
            &mut create,
        );

        assert_eq!(report.replaced, 1);
        assert_eq!(
            recorder.events(),
            vec!["mount:input", "unmount:input", "mount:input"]
        );
    }

    struct MountRecordingWidget {
        id: WidgetId,
        props: LayoutProps,
        recorder: Recorder,
    }

    impl Widget for MountRecordingWidget {
        fn id(&self) -> WidgetId {
            self.id
        }

        fn layout_props(&self) -> &LayoutProps {
            &self.props
        }

        fn on_mount(&mut self) {
            self.recorder.push("legacy-mount");
        }

        fn on_unmount(&mut self) {
            self.recorder.push("legacy-unmount");
        }
    }

    #[test]
    fn legacy_adapter_mounts_unmounts_and_disables_cache() {
        let recorder = Recorder(Rc::new(RefCell::new(Vec::new())));
        let widget = WidgetNode::new(MountRecordingWidget {
            id: WidgetId(1),
            props: LayoutProps::default(),
            recorder: recorder.clone(),
        });
        let mut adapter = LegacyWidgetAdapter::new(widget).conservative_dirty(DirtyKind::Full);

        assert_eq!(adapter.cache_policy(), ComponentCachePolicy::Disabled);
        adapter.mount(&keyed("legacy", "legacy"));
        let dirty = adapter.update(&keyed("legacy", "legacy"), &keyed("legacy", "legacy"));
        adapter.unmount(&keyed("legacy", "legacy"));

        assert_eq!(dirty, DirtyKind::Full);
        assert_eq!(recorder.events(), vec!["legacy-mount", "legacy-unmount"]);
    }

    #[test]
    #[should_panic(expected = "component-owned signal was written after its owner was dropped")]
    fn component_owned_signal_panics_when_written_after_owner_drop() {
        let owner = ComponentOwnedSignal::new("before".to_string());
        let writer = owner.write_handle();
        drop(owner);

        let _ = writer.set_collect("after".to_string());
    }
}
