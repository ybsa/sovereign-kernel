//! Infinite Memory — unified memory substrate for the Sovereign Kernel.
//!
//! Provides a single `MemorySubstrate` that composes multiple storage backends:
//! - **Structured store** (SQLite KV): Agent state, key-value pairs
//! - **Semantic store** (SQLite BLOB vectors): Vector similarity search
//! - **Knowledge graph** (SQLite): Entity-relation triples
//! - **Session store**: Conversation persistence
//! - **BM25 full-text search**: Keyword-based retrieval (from Sovereign Kernel QMD)
//! - **Hybrid ranking**: Combined BM25 + vector + MMR (from Sovereign Kernel)
//! - **Temporal decay**: Time-weighted relevance scoring (from Sovereign Kernel)

pub mod audit;
pub mod bm25;
pub mod checkpoint;
pub mod consolidation;
pub mod embedding;
pub mod hybrid;
pub mod knowledge;
pub mod mmr;
pub mod semantic;
pub mod session;
pub mod shared;
pub mod structured;
pub mod substrate;
pub mod temporal_decay;

pub use substrate::MemorySubstrate;
