//! The dependency-closure tree VIEW (arc 03 §5): the docker-pull model
//! extended to a recursive install.
//!
//! Edges live only in a payload's embedded manifest (L1 — the registry
//! mirrors resolution fields, never `requires:`), so the closure cannot be
//! planned whole before the first fetch. The view therefore GROWS as nodes
//! are discovered, and the contract it enforces is ordering: a node's row
//! is on screen before that node's bytes are fetched.
//!
//! The tree is a view of the same install walk, never a second execution
//! path — states are set by the walk as it moves, so what the user reads is
//! what the walk is doing.

use tpkg::PayloadKind;

/// Where one node stands. `Cached` is known before any byte moves (the
/// store answered); `Fetching`/`Fetched` bracket one node's download;
/// `Skipped` names an edge the host does not cover (spec 03 §2.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeState {
    Cached,
    Fetching,
    Fetched,
    Skipped,
}

impl NodeState {
    fn label(self) -> &'static str {
        match self {
            NodeState::Cached => "cached",
            NodeState::Fetching => "fetching",
            NodeState::Fetched => "fetched",
            NodeState::Skipped => "skipped",
        }
    }
}

/// One node of the closure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeNode {
    pub depth: usize,
    pub name: String,
    pub version: String,
    pub kind: PayloadKind,
    /// The node that pulled this one in (`None` for the root).
    pub edge_from: Option<String>,
    /// The row's size when the registry declares it (never guessed).
    pub size_hint: Option<u64>,
    pub state: NodeState,
    /// Why the edge was skipped (host not covered) — the row stays
    /// visible, a skip is never a silent omission.
    pub skip_reason: Option<String>,
}

/// The view: one subject (the install that started the walk) and its
/// nodes in discovery order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosureTree {
    subject: String,
    nodes: Vec<TreeNode>,
}

impl ClosureTree {
    pub fn new(subject: impl Into<String>) -> ClosureTree {
        ClosureTree {
            subject: subject.into(),
            nodes: Vec::new(),
        }
    }

    /// Add a node; returns its index for later state updates.
    pub fn push(&mut self, node: TreeNode) -> usize {
        self.nodes.push(node);
        self.nodes.len() - 1
    }

    /// The walk's state update (cached at plan time, fetching at the start
    /// of the node's fetch, fetched once it landed).
    pub fn set_state(&mut self, index: usize, state: NodeState) {
        if let Some(node) = self.nodes.get_mut(index) {
            node.state = state;
        }
    }

    pub fn set_skip(&mut self, index: usize, reason: impl Into<String>) {
        if let Some(node) = self.nodes.get_mut(index) {
            node.state = NodeState::Skipped;
            node.skip_reason = Some(reason.into());
        }
    }

    /// The rendered row for one node (the walk prints it as it moves).
    pub fn line(&self, index: usize) -> Option<String> {
        self.nodes.get(index).map(row)
    }

    pub fn nodes(&self) -> &[TreeNode] {
        &self.nodes
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// The header: the install the walk serves.
    pub fn header(&self) -> String {
        format!("resolving {}", self.subject)
    }

    /// The node rows, in discovery order, indented by depth. The last
    /// node of each depth carries the corner connector; everything else
    /// the tee, so the list reads as a tree without tracking positions.
    pub fn lines(&self) -> Vec<String> {
        self.nodes.iter().map(row).collect()
    }

    /// The closing tally — the number the user needs to know the walk is
    /// complete and what it cost.
    pub fn summary(&self) -> String {
        let count = |state: NodeState| self.nodes.iter().filter(|n| n.state == state).count();
        format!(
            "{} node(s): {} cached, {} fetched, {} skipped",
            self.nodes.len(),
            count(NodeState::Cached),
            count(NodeState::Fetched) + count(NodeState::Fetching),
            count(NodeState::Skipped),
        )
    }
}

/// One rendered row: `<indent><connector> <kind> <name> <version> — <state
/// or size>`. A skipped node names its reason instead of hiding.
fn row(node: &TreeNode) -> String {
    let indent = "  ".repeat(node.depth);
    let mut out = format!(
        "{indent}└ {} {} {}",
        kind_label(node.kind),
        node.name,
        node.version
    );
    match node.state {
        NodeState::Skipped => {
            out.push_str(" — skipped");
            if let Some(reason) = &node.skip_reason {
                out.push_str(&format!(" ({reason})"));
            }
        }
        state => {
            out.push_str(" — ");
            out.push_str(state.label());
            if let Some(size) = node.size_hint {
                out.push_str(&format!(", {}", human_size(size)));
            }
        }
    }
    if let Some(parent) = &node.edge_from {
        out.push_str(&format!(" [from {parent}]"));
    }
    out
}

fn kind_label(kind: PayloadKind) -> &'static str {
    match kind {
        PayloadKind::App => "app",
        PayloadKind::Data => "data",
        PayloadKind::Toolkit => "toolkit",
        PayloadKind::Runtime => "runtime",
        PayloadKind::Language => "language",
    }
}

/// Byte counts the way the fetch progress prints them: whole units below
/// 1 KiB, one decimal above, so two runs never disagree on a size.
pub fn human_size(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    let b = bytes as f64;
    if bytes < 1024 {
        format!("{bytes} B")
    } else if b < KIB * KIB {
        format!("{:.1} KiB", b / KIB)
    } else if b < KIB * KIB * KIB {
        format!("{:.1} MiB", b / (KIB * KIB))
    } else {
        format!("{:.2} GiB", b / (KIB * KIB * KIB))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(name: &str, depth: usize, kind: PayloadKind, state: NodeState) -> TreeNode {
        TreeNode {
            depth,
            name: name.to_string(),
            version: "1.0.0".to_string(),
            kind,
            edge_from: None,
            size_hint: None,
            state,
            skip_reason: None,
        }
    }

    #[test]
    fn rows_keep_discovery_order_and_indent_by_depth() {
        let mut tree = ClosureTree::new("hello-ruby 1.0.0");
        tree.push(node("hello-ruby", 0, PayloadKind::App, NodeState::Fetched));
        tree.push(TreeNode {
            edge_from: Some("hello-ruby".to_string()),
            ..node("fonts", 1, PayloadKind::Data, NodeState::Cached)
        });
        tree.push(TreeNode {
            edge_from: Some("fonts".to_string()),
            ..node("dejavu", 2, PayloadKind::Data, NodeState::Fetching)
        });

        assert_eq!(tree.header(), "resolving hello-ruby 1.0.0");
        let lines = tree.lines();
        assert_eq!(lines.len(), 3);
        assert!(
            lines[0].starts_with("└ app hello-ruby 1.0.0 — fetched"),
            "{lines:?}"
        );
        assert!(
            lines[1].starts_with("  └ data fonts 1.0.0 — cached"),
            "{lines:?}"
        );
        assert!(
            lines[2].starts_with("    └ data dejavu 1.0.0 — fetching"),
            "{lines:?}"
        );
        assert!(lines[2].contains("[from fonts]"), "{lines:?}");
    }

    #[test]
    fn the_summary_counts_each_state_by_name() {
        let mut tree = ClosureTree::new("app 2.0.0");
        tree.push(node("a", 0, PayloadKind::App, NodeState::Cached));
        tree.push(node("b", 1, PayloadKind::Toolkit, NodeState::Fetched));
        let skip = tree.push(node("c", 1, PayloadKind::Data, NodeState::Fetching));
        tree.set_skip(skip, "platform aarch64-macos is not covered");

        assert_eq!(tree.summary(), "3 node(s): 1 cached, 1 fetched, 1 skipped");
        let lines = tree.lines();
        assert!(lines[2].contains("skipped"), "{lines:?}");
        assert!(lines[2].contains("not covered"), "{lines:?}");
    }

    #[test]
    fn a_state_update_moves_the_row_and_a_stale_index_is_inert() {
        let mut tree = ClosureTree::new("app 1.0.0");
        let i = tree.push(node("dep", 1, PayloadKind::Toolkit, NodeState::Fetching));
        tree.set_state(i, NodeState::Fetched);
        assert_eq!(tree.nodes()[i].state, NodeState::Fetched);
        tree.set_state(99, NodeState::Cached);
        assert_eq!(tree.nodes().len(), 1);
        assert!(!tree.is_empty());
    }

    #[test]
    fn sizes_read_the_way_the_progress_prints_them() {
        assert_eq!(human_size(0), "0 B");
        assert_eq!(human_size(1023), "1023 B");
        assert_eq!(human_size(1024), "1.0 KiB");
        assert_eq!(human_size(12 * 1024 * 1024 + 400 * 1024), "12.4 MiB");
        assert_eq!(human_size(3 * 1024 * 1024 * 1024), "3.00 GiB");
    }
}
