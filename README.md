# Lingoes RS

A clean-room, Rust-only dictionary and translation desktop application inspired by the useful workflows of classic Lingoes.

## Principles

- Rust for the application, platform integration, parsers, search, and UI.
- Windows-first desktop delivery; portable packaging is a primary target.
- Clean-room reconstruction: observe public behavior and file formats, but do not copy proprietary source code, branding, dictionaries, or assets.
- Offline-first dictionary lookup, with optional online translation providers.
- Accessibility APIs first for text capture; OCR is a fallback.

## Current milestone

Milestone 2 provides a production dictionary engine and multi-format import pipeline:
- **Unicode-friendly Search**: NFC normalization and lowercase folding for exact match, scalable indexed prefix lookup, and full-text search (SQLite FTS5 with BM25 ranking).
- **Supported Formats**:
  - UTF-8 TSV (`headword<TAB>definition`).
  - StarDict (`.ifo`, `.idx`, `.dict`, and compressed `.dict.dz`).
  - MDict MDX v2.0 (pure-Rust via `opendict-rs`).
- **Atomic Bulk Import**: Transaction-based imports roll back completely on malformed entries or I/O errors.
- **Reporting & Statistics**: Import operations report imported count, newly inserted entries, and updated entries.
- **Desktop GUI & CLI**: Slint GUI exposes both prefix and full-text search, multi-format import auto-detection, and status reporting.

## Workspace

- `crates/lingoes-core`: domain types, unicode normalization, and search contracts.
- `crates/lingoes-dictionary`: dictionary storage, FTS5 indexes, and multi-format importers.
- `apps/lingoes-desktop`: desktop executable and Slint UI.
- `docs/plans`: implementation plans and milestones.

## Development

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p lingoes-desktop
```

Running without arguments opens the GUI. The search field has initial keyboard focus and searches as you type; Enter also submits search and import paths. Use the **Full text** button for content searches across definitions.

The automation interface:

```bash
cargo run -p lingoes-desktop -- import path/to/dictionary.db dictionary.tsv
cargo run -p lingoes-desktop -- import path/to/dictionary.db dictionary.ifo
cargo run -p lingoes-desktop -- import path/to/dictionary.db dictionary.mdx
cargo run -p lingoes-desktop -- lookup path/to/dictionary.db query
cargo run -p lingoes-desktop -- full-text path/to/dictionary.db query
```
