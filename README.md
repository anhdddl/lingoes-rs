# Lingoes RS

A clean-room, Rust-only dictionary and translation desktop application inspired by the useful workflows of classic Lingoes.

## Principles

- Rust for the application, platform integration, parsers, search, and UI.
- Windows-first desktop delivery; portable packaging is a primary target.
- Clean-room reconstruction: observe public behavior and file formats, but do not copy proprietary source code, branding, dictionaries, or assets.
- Offline-first dictionary lookup, with optional online translation providers.
- Accessibility APIs first for text capture; OCR is a fallback.

## Current milestone

Milestone 1 provides a native Slint desktop interface backed by the SQLite dictionary core. It includes live prefix search, a result/detail view, TSV import by path, visible status/error feedback, and a portable database stored at `data/lingoes.db` beside the executable.

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

Running without arguments opens the GUI. The search field has initial keyboard focus and searches as you type; Enter also submits search and import paths. Import files are UTF-8 TSV rows in the form `headword<TAB>definition`; blank lines and `#` comments are ignored.

The existing automation interface remains available:

```bash
cargo run -p lingoes-desktop -- import path/to/dictionary.db entries.tsv
cargo run -p lingoes-desktop -- lookup path/to/dictionary.db query
```
