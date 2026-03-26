# Sovereign Kernel Maintenance Guide

This guide is for developers and maintainers who want to contribute to the Sovereign Kernel or add new capabilities to the "Village".

## 🛠️ Development Workflow

1. **Rust Environment**: Ensure you are using the latest stable Rust.
2. **Feature Branches**: Always branch from `master` for new features or "Hands".
3. **CI Compliance**: Every push must pass the GitHub Actions CI (build, test, fmt, and clippy).
   - Run `cargo fmt --all` before committing.
   - Run `cargo clippy --workspace --all-targets -- -D warnings` to ensure zero-warning state.

## 🤝 Adding New Hands (Autonomous Skills)

1. Create a new directory in `crates/sk-hands/bundled/`.
2. Create a `SKILL.md` file using the established [prompting standards](docs/CONTRIBUTING.md).
3. Register the new hand in `crates/sk-hands/src/registry.rs`.
4. Add any required environment variables to `.env.example`.

## 📦 Release Process

1. **Update CHANGELOG.md**: Record all notable changes in the `[Unreleased]` or new version section.
2. **Bump Version**: Update `version` in the root `Cargo.toml` and potentially individual crates.
3. **Tagging**:
   ```bash
   git tag -a v1.x.x -m "Release v1.x.x"
   git push origin v1.x.x
   ```
4. **Binary Assets**: The CI pipeline automatically builds and attaches binaries to GitHub Releases (once configured).

## 🛡️ Security Policy

- All agents must run in **Sandbox** mode by default.
- Any new "Host Tools" must be risk-classified in `sk-types/src/risk.rs` and gated in `SafetyGate`.
- Report security vulnerabilities via the guidelines in [SECURITY.md](docs/SECURITY.md).

---

*For more details, see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) and [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md).*
