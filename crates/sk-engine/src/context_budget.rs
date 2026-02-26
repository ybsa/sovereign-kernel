//! Context budget — manage context window token limits.
pub const DEFAULT_CONTEXT_WINDOW: usize = 128_000;
pub fn fits_in_context(token_count: usize, limit: usize) -> bool { token_count <= limit }
