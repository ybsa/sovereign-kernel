//! The Soul — identity, persona, and continuity.
//!
//! Parses SOUL.md files (inspired by OpenClaw's identity system) and injects
//! persona directives into the agent's system prompt. The Soul is what makes
//! the kernel *someone* rather than just *something*.

pub mod continuity;
pub mod identity;
pub mod persona;

pub use identity::SoulIdentity;
pub use persona::Persona;
