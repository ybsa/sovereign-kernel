//! Security primitives — taint labels and signed manifests.

use serde::{Deserialize, Serialize};

/// Information flow taint label.
///
/// Labels propagate through execution — secrets are tracked from source to sink.
/// Inspired by OpenFang's 16-layer security model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaintLabel {
    /// Data contains API keys or secrets.
    Secret,
    /// Data contains personally identifiable information.
    Pii,
    /// Data comes from an untrusted external source.
    Untrusted,
    /// Data has been sanitized and is safe to output.
    Clean,
}

/// A taint set for tracking data sensitivity.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaintSet {
    labels: Vec<TaintLabel>,
}

impl TaintSet {
    pub fn new() -> Self {
        Self { labels: Vec::new() }
    }

    pub fn add(&mut self, label: TaintLabel) {
        if !self.labels.contains(&label) {
            self.labels.push(label);
        }
    }

    pub fn contains(&self, label: TaintLabel) -> bool {
        self.labels.contains(&label)
    }

    pub fn is_clean(&self) -> bool {
        self.labels.is_empty() || (self.labels.len() == 1 && self.labels[0] == TaintLabel::Clean)
    }

    /// Merge another taint set into this one (union of labels).
    pub fn merge(&mut self, other: &TaintSet) {
        for label in &other.labels {
            self.add(*label);
        }
    }
}

/// Capability gate — what actions an agent is allowed to perform.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    /// Can read files from the filesystem.
    FileRead,
    /// Can write files to the filesystem.
    FileWrite,
    /// Can execute shell commands.
    ShellExec,
    /// Can make HTTP requests (outbound).
    HttpRequest,
    /// Can access MCP servers.
    McpAccess,
    /// Can manage other agents.
    AgentManage,
    /// Can modify kernel configuration.
    ConfigWrite,
    /// Full access (superuser).
    Admin,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taint_set_operations() {
        let mut ts = TaintSet::new();
        assert!(ts.is_clean());

        ts.add(TaintLabel::Secret);
        assert!(!ts.is_clean());
        assert!(ts.contains(TaintLabel::Secret));

        // Adding duplicate is idempotent
        ts.add(TaintLabel::Secret);
        assert_eq!(ts.labels.len(), 1);
    }

    #[test]
    fn taint_set_merge() {
        let mut a = TaintSet::new();
        a.add(TaintLabel::Secret);

        let mut b = TaintSet::new();
        b.add(TaintLabel::Pii);

        a.merge(&b);
        assert!(a.contains(TaintLabel::Secret));
        assert!(a.contains(TaintLabel::Pii));
    }
}
