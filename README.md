# Lingoes RS

A clean-room, Rust-only dictionary and translation desktop application inspired by the useful workflows of classic Lingoes.

## Principles

- Rust for the application, platform integration, parsers, search, and UI.
- Windows-first desktop delivery; portable packaging is a primary target.
- Clean-room reconstruction: observe public behavior and file formats, but do not copy proprietary source code, branding, dictionaries, or assets.
- Offline-first dictionary lookup, with optional online translation providers.
- Accessibility APIs first for text capture; OCR is a fallback.

## Current milestone

Milestone 0 establishes a tested dictionary/search core, import pipeline, and runnable desktop shell.

## Workspace

- `crates/lingoes-core`: domain types and search contracts.
- `crates/lingoes-dictionary`: dictionary storage and importers.
- `apps/lingoes-desktop`: desktop executable.
- `docs/plans`: implementation plans and milestones.

## Development

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p lingoes-desktop
```
