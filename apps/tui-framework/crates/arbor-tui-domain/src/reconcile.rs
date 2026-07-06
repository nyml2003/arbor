use std::collections::HashMap;

use crate::identity::{DirtyKind, NodeIdentity, ReconcileReport};
use crate::widget::WidgetNode;
use crate::widget_id::WidgetId;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ReconcileNode {
    pub id: WidgetId,
    pub identity: NodeIdentity,
    pub widget_type: &'static str,
    pub children: Vec<ReconcileNode>,
}

impl ReconcileNode {
    fn from_widget(node: &WidgetNode) -> Self {
        Self {
            id: node.id(),
            identity: node
                .identity()
                .expect("widget tree identity must be assigned before reconcile"),
            widget_type: node.widget_type_name(),
            children: node.children().iter().map(Self::from_widget).collect(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ReconcileDecision {
    Reuse {
        old: WidgetId,
        next: WidgetId,
    },
    Replace {
        old: WidgetId,
        next: WidgetId,
        reason: ReplaceReason,
    },
    Mount {
        next: WidgetId,
    },
    Unmount {
        old: WidgetId,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ReplaceReason {
    IdentityChanged,
    WidgetTypeChanged,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct ReconcilePlan {
    pub decisions: Vec<ReconcileDecision>,
    pub report: ReconcileReport,
}

impl ReconcilePlan {
    fn record_reuse(&mut self, old: WidgetId, next: WidgetId) {
        self.report.reused += 1;
        self.decisions.push(ReconcileDecision::Reuse { old, next });
    }

    fn record_replace(&mut self, old: WidgetId, next: WidgetId, reason: ReplaceReason) {
        self.report.replaced += 1;
        self.report.mounted += 1;
        self.report.unmounted += 1;
        self.report.record_dirty(next, DirtyKind::Structure);
        self.decisions
            .push(ReconcileDecision::Replace { old, next, reason });
    }

    fn record_mount(&mut self, next: WidgetId) {
        self.report.mounted += 1;
        self.report.record_dirty(next, DirtyKind::Structure);
        self.decisions.push(ReconcileDecision::Mount { next });
    }

    fn record_unmount(&mut self, old: WidgetId) {
        self.report.unmounted += 1;
        self.report.focus_dirty = true;
        self.decisions.push(ReconcileDecision::Unmount { old });
    }
}

pub fn plan_reconcile(old: Option<&WidgetNode>, next: &WidgetNode) -> ReconcilePlan {
    match old {
        Some(old) => {
            let old_snapshot = ReconcileNode::from_widget(old);
            let next_snapshot = ReconcileNode::from_widget(next);
            plan_reconcile_nodes(Some(&old_snapshot), &next_snapshot)
        }
        None => {
            let next_snapshot = ReconcileNode::from_widget(next);
            plan_reconcile_nodes(None, &next_snapshot)
        }
    }
}

pub fn plan_reconcile_nodes(old: Option<&ReconcileNode>, next: &ReconcileNode) -> ReconcilePlan {
    let mut plan = ReconcilePlan::default();
    reconcile_node(old, next, &mut plan);
    plan
}

fn reconcile_node(old: Option<&ReconcileNode>, next: &ReconcileNode, plan: &mut ReconcilePlan) {
    let Some(old) = old else {
        mount_subtree(next, plan);
        return;
    };

    if old.identity != next.identity {
        plan.record_replace(old.id, next.id, ReplaceReason::IdentityChanged);
        unmount_children(old, plan);
        mount_children(next, plan);
        return;
    }

    if old.widget_type != next.widget_type {
        plan.record_replace(old.id, next.id, ReplaceReason::WidgetTypeChanged);
        unmount_children(old, plan);
        mount_children(next, plan);
        return;
    }

    plan.record_reuse(old.id, next.id);
    reconcile_children(old, next, plan);
}

fn reconcile_children(old: &ReconcileNode, next: &ReconcileNode, plan: &mut ReconcilePlan) {
    let mut old_by_identity: HashMap<&NodeIdentity, &ReconcileNode> = HashMap::new();
    for child in &old.children {
        old_by_identity.insert(&child.identity, child);
    }

    let mut reused_old = Vec::new();
    for next_child in &next.children {
        let old_child = old_by_identity.get(&next_child.identity).copied();
        if let Some(old_child) = old_child {
            reused_old.push(&old_child.identity);
        }
        reconcile_node(old_child, next_child, plan);
    }

    for old_child in &old.children {
        if !reused_old.contains(&&old_child.identity) {
            unmount_subtree(old_child, plan);
        }
    }
}

fn mount_subtree(node: &ReconcileNode, plan: &mut ReconcilePlan) {
    plan.record_mount(node.id);
    mount_children(node, plan);
}

fn mount_children(node: &ReconcileNode, plan: &mut ReconcilePlan) {
    for child in &node.children {
        mount_subtree(child, plan);
    }
}

fn unmount_subtree(node: &ReconcileNode, plan: &mut ReconcilePlan) {
    plan.record_unmount(node.id);
    unmount_children(node, plan);
}

fn unmount_children(node: &ReconcileNode, plan: &mut ReconcilePlan) {
    for child in &node.children {
        unmount_subtree(child, plan);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::LayoutProps;
    use crate::widget::{assign_tree_identity, Widget};

    struct TestWidget {
        id: WidgetId,
        props: LayoutProps,
        children: Vec<WidgetNode>,
    }

    struct OtherWidget {
        id: WidgetId,
        props: LayoutProps,
    }

    impl TestWidget {
        fn leaf(id: u64) -> WidgetNode {
            WidgetNode::new(Self {
                id: WidgetId(id),
                props: LayoutProps::default(),
                children: Vec::new(),
            })
        }

        fn branch(id: u64, children: Vec<WidgetNode>) -> WidgetNode {
            WidgetNode::new(Self {
                id: WidgetId(id),
                props: LayoutProps::default(),
                children,
            })
        }
    }

    impl Widget for TestWidget {
        fn id(&self) -> WidgetId {
            self.id
        }

        fn layout_props(&self) -> &LayoutProps {
            &self.props
        }

        fn children(&self) -> &[WidgetNode] {
            &self.children
        }

        fn children_mut(&mut self) -> &mut [WidgetNode] {
            &mut self.children
        }
    }

    impl Widget for OtherWidget {
        fn id(&self) -> WidgetId {
            self.id
        }

        fn layout_props(&self) -> &LayoutProps {
            &self.props
        }
    }

    fn with_identity(mut node: WidgetNode) -> WidgetNode {
        assign_tree_identity(&mut node).unwrap();
        node
    }

    #[test]
    fn same_key_and_type_are_reuse_candidates() {
        let old = with_identity(TestWidget::leaf(1).with_key("same"));
        let next = with_identity(TestWidget::leaf(2).with_key("same"));

        let plan = plan_reconcile(Some(&old), &next);

        assert_eq!(plan.report.reused, 1);
        assert_eq!(
            plan.decisions,
            vec![ReconcileDecision::Reuse {
                old: WidgetId(1),
                next: WidgetId(2),
            }]
        );
    }

    #[test]
    fn same_key_and_different_type_are_replace_candidates() {
        let old = with_identity(TestWidget::leaf(1).with_key("same"));
        let next = with_identity(
            WidgetNode::new(OtherWidget {
                id: WidgetId(2),
                props: LayoutProps::default(),
            })
            .with_key("same"),
        );

        let plan = plan_reconcile(Some(&old), &next);

        assert_eq!(plan.report.replaced, 1);
        assert!(plan.report.focus_dirty);
        assert_eq!(
            plan.decisions,
            vec![ReconcileDecision::Replace {
                old: WidgetId(1),
                next: WidgetId(2),
                reason: ReplaceReason::WidgetTypeChanged,
            }]
        );
    }

    #[test]
    fn keyed_reorder_preserves_child_reuse_candidates() {
        let old = with_identity(TestWidget::branch(
            10,
            vec![
                TestWidget::leaf(1).with_key("a"),
                TestWidget::leaf(2).with_key("b"),
            ],
        ));
        let next = with_identity(TestWidget::branch(
            20,
            vec![
                TestWidget::leaf(22).with_key("b"),
                TestWidget::leaf(21).with_key("a"),
            ],
        ));

        let plan = plan_reconcile(Some(&old), &next);

        assert_eq!(plan.report.reused, 3);
        assert!(plan.decisions.contains(&ReconcileDecision::Reuse {
            old: WidgetId(1),
            next: WidgetId(21),
        }));
        assert!(plan.decisions.contains(&ReconcileDecision::Reuse {
            old: WidgetId(2),
            next: WidgetId(22),
        }));
    }

    #[test]
    fn missing_child_is_unmounted_and_marks_focus_dirty() {
        let old = with_identity(TestWidget::branch(
            10,
            vec![
                TestWidget::leaf(1).with_key("a"),
                TestWidget::leaf(2).with_key("b"),
            ],
        ));
        let next = with_identity(TestWidget::branch(
            20,
            vec![TestWidget::leaf(21).with_key("a")],
        ));

        let plan = plan_reconcile(Some(&old), &next);

        assert_eq!(plan.report.unmounted, 1);
        assert!(plan.report.focus_dirty);
        assert!(plan
            .decisions
            .contains(&ReconcileDecision::Unmount { old: WidgetId(2) }));
    }
}
