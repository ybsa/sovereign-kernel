//! Loop guard — detect tool call loops (from Sovereign Kernel's loop_guard.rs).
//!
//! SHA256-based detection of repetitive tool call patterns.

use sha2::{Digest, Sha256};
use std::collections::VecDeque;

const WINDOW_SIZE: usize = 10;
const MAX_REPEATS: usize = 3;

/// Detects repetitive tool call patterns to prevent infinite loops.
pub struct LoopGuard {
    history: VecDeque<String>,
}

impl LoopGuard {
    pub fn new() -> Self {
        Self {
            history: VecDeque::with_capacity(WINDOW_SIZE * 2),
        }
    }

    /// Record a tool call and check for loops.
    /// Returns true if a loop is detected.
    pub fn check(&mut self, tool_name: &str, arguments: &str) -> bool {
        let hash = self.hash_call(tool_name, arguments);
        self.history.push_back(hash.clone());

        if self.history.len() > WINDOW_SIZE * 2 {
            self.history.pop_front();
        }

        // Check for repeated patterns of length 1 to WINDOW_SIZE
        for pattern_len in 1..=WINDOW_SIZE.min(self.history.len() / 2) {
            if self.detect_pattern(pattern_len) >= MAX_REPEATS {
                return true;
            }
        }
        false
    }

    fn hash_call(&self, tool_name: &str, arguments: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(tool_name.as_bytes());
        hasher.update(b"|");
        hasher.update(arguments.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn detect_pattern(&self, pattern_len: usize) -> usize {
        let len = self.history.len();
        if len < pattern_len * 2 {
            return 0;
        }
        let pattern: Vec<&String> = self.history.iter().rev().take(pattern_len).collect();
        let mut repeats = 1;
        let mut pos = len - pattern_len;
        while pos >= pattern_len {
            pos -= pattern_len;
            let window: Vec<&String> = self.history.iter().skip(pos).take(pattern_len).collect();
            let window_rev: Vec<&String> = window.into_iter().rev().collect();
            if window_rev == pattern {
                repeats += 1;
            } else {
                break;
            }
        }
        repeats
    }

    pub fn reset(&mut self) {
        self.history.clear();
    }
}

impl Default for LoopGuard {
    fn default() -> Self {
        Self::new()
    }
}
