# Lingoes RS full-parity roadmap (Windows first, clean room)

**Date:** 2026-10-14  
**Target reference:** Lingoes 2.9.2 Portable  
**Implementation target:** native Rust application, Windows-first with portable core and platform adapters  
**Repository baseline:** `793bbbcb3aeef98451c6438e135e108eb2fe5d67`

## 1. Scope and claims policy

This plan targets **functional workflow parity**, not source, asset, trademark, or visual duplication. The extracted installation is evidence for externally visible capabilities and persisted settings only. Do not copy its executables, DLLs, fonts, icons, bitmaps, HTML/JavaScript appendices, localization strings, dictionary content, or product name/branding into the new application.

“Parity” in this document means a requirement has a recorded reference observation and the Rust implementation passes its corresponding automated and Windows-native acceptance tests. Static files alone cannot establish exact behavior. Therefore:

- Do not claim exact pixel parity, timing parity, dictionary-format coverage, or capture success until the relevant reference and implementation scenarios have run on the same Windows test matrix.
- Mark every row in the parity ledger as `inferred`, `observed-static`, `observed-runtime`, `implemented`, `verified`, `deferred`, or `not-applicable`.
- A feature is release-complete only at `verified`; code presence or a unit test is only `implemented`.
- Reference screenshots, event traces, output hashes, and test reports may be retained as internal test evidence, but original copyrighted assets must not enter source control or release packages.
- The compatibility name in UI and packaging must be independently branded. “Lingoes” may appear only in compatibility documentation such as “imports Lingoes LD2,” subject to legal review.

## 2. Evidence baseline

### 2.1 Static reference evidence

The following requirements are grounded in the extracted reference tree at `/home/longpc/lingoes-original-audit/extracted`:

- `app_data/data/def_config.ini` exposes appearance, proxy/update, history length, start-minimized, global hotkeys, TTS voice/pitch/rate/volume and auto-pronunciation, capture delay and script/number filters, matching mode, and first-result-only behavior.
- `app_data/data/def_user.ini` exposes independent clipboard, cursor-grab, and selection modes; main/mini window geometry; mini-window roll-up/always-visible behavior; sidebar visibility; search engine; and translation languages.
- `app_data/lang/lang-en.ini` describes tray commands, main/mini window actions, history, web search, index/guide/appendix views, copy/print/save/find/select-all, dictionary query/index/capture groups and ordering, LD2/LDP install, LDB backup/import, old-version migration, file associations, capture filters, speech controls, and configurable hotkeys.
- `app_data/lang/lang-{ar,ar-SA,fa,he,he-IL,ug}.ini` declares right-to-left UI direction. The language directory contains many additional locales; those files are evidence of locale breadth, not permission to copy their strings.
- `app_data/image/*` names show distinct regular/RTL controls and popup/window actions. They establish control roles only; they are not reusable assets.
- `app_data/dict/*.ldx` and `app_data/dict/pre_install/*.ldx` are binary format specimens. There are no `.ld2` specimens in the extracted tree currently inspected, so LD2 behavior remains an explicit evidence gap.
- `app_data/webe/webengine.xml` shows language-aware configurable web-search templates.
- `app_data/appe/**/setup.xml` identifies 13 appendix families: calculator, weights/measures, time zones, dialing codes, exchange rates, periodic table, IPA, simplified/traditional Chinese conversion, number-to-Chinese, verb reference, dictionary abbreviations, hashes, and Base64.
- `app_data/data/def_zwfilter.txt` shows process/window-class filtering for selection capture.

These observations do **not** establish event ordering, popup focus rules, precise activation masks, binary format semantics, rendering compatibility, or behavior under modern Windows security boundaries. Those require controlled runtime observation.

### 2.2 Current Rust baseline

At the pinned baseline, the repository contains:

- `lingoes-core`: Unicode NFC/lowercase query normalization and dictionary entry types.
- `lingoes-dictionary`: SQLite/FTS5, dictionary enable/disable/delete, atomic TSV import, StarDict import, and a limited MDict v2 import through `opendict-rs`.
- `lingoes-translator`: Google-compatible HTTP translation and OpenAI-compatible translation traits/clients.
- `lingoes-tts`: command-based system TTS; Windows currently shells to PowerShell rather than using native SAPI/WinRT APIs.
- `lingoes-capture`: polling clipboard abstraction with debounce and length bounds only.
- `lingoes-desktop`: Slint main UI, lookup/full-text search, dictionary management, translation, TTS, clipboard-driven in-window quick card, and a data path beside the executable.

Known baseline gaps include Windows hotkeys/tray/single-instance lifecycle, a true independent popup window, UI Automation and OCR capture, selection mode, durable settings/history/favorites, dictionary groups/order, LD2/LDX, rich records/resources, native Windows TTS, printing/saving, localization/RTL, appendices, backup/migration, hardened portable packaging, and Windows runtime evidence.

## 3. Architecture target

Preserve a cross-platform dependency direction: domain crates never depend on Slint, Win32, WebView, or OCR engines.

```text
apps/lingoes-desktop
  -> lingoes-application       commands, view models, async jobs, policy
  -> lingoes-ui-slint          main/popup/settings/history/tool views
  -> lingoes-platform          portable platform traits
       -> lingoes-platform-windows   windows-rs adapters
       -> future platform adapters   macOS/Linux capability subsets
  -> lingoes-core              terms, results, settings, events
  -> lingoes-storage           SQLite, migrations, history/favorites/cache
  -> lingoes-dictionary        parsers, package model, indexing, export
  -> lingoes-document          safe rich-document AST and resource resolver
  -> lingoes-capture           capture arbitration and filtering
  -> lingoes-translator
  -> lingoes-tts
  -> lingoes-appendices
```

Use capability traits with explicit unsupported states rather than `cfg` branches in view models:

- `GlobalHotkeyService`, `TrayService`, `WindowLocator`, `TextCaptureProvider`, `RegionCaptureProvider`, `OcrProvider`, `PrintService`, `SecretStore`, `AutostartService`, `FileAssociationService`, and `NativeTtsService`.
- `CaptureCoordinator` receives candidates from clipboard, selected-text, UIA-under-cursor, and OCR. It applies activation, script filters, timeout, confidence, deduplication, and cancellation policy before lookup.
- All potentially slow operations are cancellable jobs. Never block the Slint event loop on import, HTTP, OCR, SQLite maintenance, or TTS.
- Settings use versioned `serde` models and atomic writes (`tempfile` + flush + rename). Durable user data remains in SQLite with numbered migrations and backup-before-upgrade.
- Event payloads are plain owned Rust data. UI updates return through `slint::invoke_from_event_loop`.

### Recommended crates and APIs

Pin exact versions when implementation starts and record license/SBOM results; names below are choices, not an instruction to track floating latest versions.

| Concern | Rust crate/API |
|---|---|
| Win32/WinRT | `windows` (`windows-rs`) with narrowly selected features |
| Async/runtime | existing `tokio`; `tokio-util::sync::CancellationToken` |
| Settings/config | `serde`, `toml`, `directories` (installed mode only), `tempfile` |
| Database | existing `rusqlite` bundled/FTS5, explicit numbered migrations |
| Localization | `fluent-bundle`, `fluent-langneg`, `unic-langid`, `sys-locale` |
| Unicode | existing `unicode-normalization`; add `unicode-segmentation`, `unicode-script`, `unicode-bidi` where needed |
| HTML/document parsing | `html5ever`/`markup5ever_rcdom`, `cssparser`, `url`; allowlist sanitizer implemented over the parsed tree or audited `ammonia` policy |
| Images/audio | `image` with decode limits; `symphonia`/`rodio` only if embedded pronunciation audio is required by evidence |
| Compression/checksums | `flate2`, `zstd`, `zip` with limits, `crc32fast`, `sha2` |
| Secrets | Windows Credential Manager via `CredWriteW`/`CredReadW`; optional cross-platform `keyring` adapter |
| Testing | `proptest`, `insta` for semantic snapshots, `tempfile`, `wiremock`; Windows UI harness using `windows-rs` UIA |
| Packaging | `cargo-wix` or hand-maintained WiX for installed build; deterministic ZIP for portable build; `cargo-about`/`cargo-deny`; `syft` if available |

Windows feature sets should include only what an adapter uses. Expected modules include:

- `Win32::UI::WindowsAndMessaging`: `RegisterHotKey`, `UnregisterHotKey`, `WM_HOTKEY`, message-only window, cursor/window APIs.
- `Win32::UI::Shell`: `Shell_NotifyIconW`, `NOTIFYICONDATAW`, `NIM_ADD/MODIFY/DELETE`, `NIM_SETVERSION`.
- `Win32::UI::Accessibility`: `CUIAutomation8`, `IUIAutomation`, `IUIAutomationElement`, text patterns, `SetWinEventHook`; use `AccessibleObjectFromPoint` only as a bounded legacy fallback.
- `Win32::System::Com`: apartment initialization and COM marshaling rules.
- `Win32::System::Threading`: named mutex for single instance.
- `Win32::Graphics::Gdi` or `Windows::Graphics::Capture`: bounded screen-region acquisition for OCR; prefer Windows Graphics Capture when supported and use GDI only for compatible desktop targets.
- `Media::Ocr` and `Graphics::Imaging`: `OcrEngine` and `SoftwareBitmap` WinRT OCR path.
- `Media::SpeechSynthesis`: native voice enumeration and synthesis, or SAPI through COM if runtime tests show better parity.
- `Win32::Graphics::Printing`/document package APIs or a print-dialog-backed rendered document path, selected by a prototype milestone.
- `Win32::Security::Credentials`: credential storage for API keys/proxy credentials.

Every unsafe Windows wrapper must be small, documented with ownership/threading invariants, and covered by a safe mockable interface.

## 4. Verifiable parity ledger

Create `docs/parity/parity-ledger.toml` in Milestone 0 and update it in every parity PR. Each row must contain: stable ID, user workflow, evidence path(s), reference status, implementation status, automated test IDs, Windows scenarios, supported platforms, deviation/unknown, and evidence artifact hashes.

Initial ledger:

| ID | Capability | Evidence | Baseline | Verification gate |
|---|---|---|---|---|
| APP-01 | Main and mini lookup windows, geometry restore | `def_user.ini` | Main Slint window; no independent mini window | Runtime focus/geometry scenarios at 100/150/200% DPI |
| APP-02 | Single instance, start minimized, tray lifecycle | `def_config.ini`, `lang-en.ini` | Missing | Relaunch routes command to first process; tray survives window close per chosen policy |
| APP-03 | Configurable global hotkeys | `[Hotkey]` | Missing | Registration/conflict/rebind/restart tests on Windows |
| LOOK-01 | Exact/prefix/index/full-text lookup | locale resources; current implementation | Partial | Corpus oracle for ordering, grouping, disabled dictionaries, Unicode |
| LOOK-02 | Back/forward/history/sidebar/index behavior | `lang-en.ini` | Missing | Runtime sequence traces plus persistent history tests |
| DICT-01 | Install/remove/order/query/index/capture groups | `lang-en.ini` | Partial enable/delete only | Group/order tests and cross-restart UI tests |
| DICT-02 | LD2/LDP install and LDX runtime dictionary | locale/resources | Unknown format; missing | Golden corpus and differential reference tests for each accepted dialect |
| DICT-03 | Backup/import | `.ldb` filter in locale | Missing | Round-trip and failure atomicity tests; proprietary LDB only if independently specified |
| RENDER-01 | Rich definitions, links, images, audio, resource lookup | dictionary files/images and UI controls | Plain text only | Sanitized fixture gallery semantic/screenshot tests |
| CAP-01 | Clipboard translation | configs | Polling prototype | Windows clipboard event, filtering, no feedback loop, popup tests |
| CAP-02 | Word under cursor | configs and capture resources | Missing | UIA fixture apps plus browser/Office/PDF matrix |
| CAP-03 | Selected text/double-click translation | configs/filter file | Missing | Selection events and per-process/window deny rules |
| CAP-04 | OCR fallback | parity roadmap requirement; runtime behavior unconfirmed | Missing | Image corpus CER thresholds and on-screen fallback scenarios |
| POP-01 | Non-activating capture popup, pin/roll-up/open/speak/copy | locale/image names, geometry config | In-window card only | Focus invariants and monitor-edge placement tests |
| TTS-01 | Voice/rate/pitch/volume/stop and auto-pronounce | `[Speech]`, locale | Basic command invocation | Native voice tests and cancellation |
| TRAN-01 | Text translation and language/engine settings | configs/resources | Partial | mocked provider contract and opt-in live smoke tests |
| DATA-01 | Search history and cache clearing/limits | config/locale | Missing | persistence, cap, dedupe, clear, private-mode tests |
| DATA-02 | Favorites | requested target; not located in inspected locale/static files | Missing/evidence gap | Define product behavior; do not label reference parity without runtime evidence |
| DOC-01 | Copy/find/select-all/save HTML/print | `lang-en.ini` | Copy plain text only | saved standalone document and printed/PDF semantic comparison |
| I18N-01 | Localized UI and RTL | language files | Hard-coded Vietnamese UI | locale fallback and mirrored RTL UI tests |
| APPE-01 | 13 appendix workflows | `app_data/appe` inventory | Missing | Independent fixtures and calculation reference vectors |
| PORT-01 | Portable beside-executable data layout | portable target/current path | Partial | clean Windows VM/no-admin/read-only/removable-drive tests |
| MIG-01 | Old-version import | locale string | Missing | fixture migration report, backup, rollback, idempotency |
| SEC-01 | Safe untrusted dictionary/resource handling | required for new implementation | Partial size limits | parser fuzzing, traversal/SSRF/script/resource bombs, secret redaction |

If runtime observation disproves an inferred row, change the requirement and preserve the old observation in the ledger history rather than silently redefining parity.

## 5. Runtime evidence protocol

Before implementing ambiguous behavior, run the reference application only in a disposable, offline-first Windows VM. Never instrument or copy proprietary code. Observe inputs and outputs:

1. Snapshot the VM and hash the reference package.
2. Use purpose-built, redistributable fixture applications: Win32 edit controls, WPF, WinUI 3, Chromium contenteditable, a PDF viewer, console, elevated/non-elevated windows, and mixed-DPI monitors.
3. Record user input scripts, window rectangles/DPI, focus before/after, UIA properties, visible output, timing ranges, filesystem/registry deltas, and network destinations. Redact personal data.
4. For dictionary formats, create dictionaries from independently authored content using documented/public tooling where legally permitted; compare entry count, metadata, lookup output, resource resolution, synonyms, and malformed-file behavior.
5. Store scenario manifests and hashes, not original application assets. A reviewer who did not inspect implementation internals approves the behavioral specification before implementers consume it.
6. Keep researcher notes/specification and implementation commits attributable to separate roles when practical. Implementation code must cite the behavioral spec, not decompiled output.

## 6. Milestones and acceptance tests

Each milestone follows red-green-refactor and ends with:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --workspace --doc
cargo deny check
```

Windows-specific jobs additionally build/test `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`. Do not make GNU ABI the release reference.

### Milestone 0 — Evidence, ledger, and test harness

**Deliverables**

- Add the machine-readable parity ledger and schema validator.
- Inventory current/reference features with evidence classifications and explicit unknowns.
- Create scenario IDs and an evidence artifact manifest with SHA-256 hashes.
- Build independently authored test dictionaries and UI fixture applications; no extracted asset becomes a fixture.
- Establish reproducible Windows VM images and CI labels.

**Acceptance tests**

- Ledger validation fails if a capability lacks evidence status, acceptance test, owner, or deviation field.
- A script verifies every referenced evidence artifact hash and rejects paths under release/package directories.
- Clean-room audit confirms no hashes of reference assets occur in tracked product/package files.
- Baseline Windows scenarios produce reports; unrun cells are visibly `NOT RUN`, never pass.

### Milestone 1 — Application shell, settings, single instance, tray, and hotkeys

**Implementation**

- Split view models from Slint components and introduce platform traits.
- Implement versioned settings and two data roots: portable root beside executable when `portable.toml` exists; installed root under `%LOCALAPPDATA%/<independent-brand>/` otherwise.
- Use a per-user named mutex plus a restricted local IPC endpoint to enforce one instance and route lookup/open commands.
- Implement a hidden/message-only HWND host. Register configurable hotkeys using `RegisterHotKey`; process `WM_HOTKEY`; unregister deterministically. Surface conflicts and support “disable all hotkeys.”
- Implement tray icon/menu with `Shell_NotifyIconW`; recreate after Explorer restart (`TaskbarCreated`), expose capture toggles, main/mini window, settings/dictionaries, and exit.
- Draw new icons from independently created source; do not derive from the reference artwork.

**Acceptance tests**

- A second launch does not create a second database writer or tray icon and forwards a lookup within 2 seconds.
- Default and rebound hotkeys work while the app is unfocused, survive restart, report `ERROR_HOTKEY_ALREADY_REGISTERED`, and release on exit/crash recovery.
- Explorer restart restores one tray icon and menu states match capture states.
- Closing the main window, explicit Exit, start-minimized, and shutdown follow documented policy with no orphan process.
- Portable mode performs no writes to registry/AppData except OS-managed caches unavoidable and documented by test evidence.

### Milestone 2 — Durable storage, lookup semantics, history, favorites, and dictionary groups

**Implementation**

- Introduce migrations for settings metadata, search/translation history, favorites, dictionary order, query/index/capture group membership, online cache metadata, and audit-safe import jobs.
- Preserve multiple entries/records/synonyms rather than flattening all records into one plain string.
- Define normalization by language/script; retain original text and use normalized keys for lookup. Add fuzzy/spelling suggestions only after reference ordering is measured.
- Implement bounded history (reference default shows 20), dedupe policy, back/forward cursor, clear history/cache, private-history toggle, and export.
- Implement favorites as an independent product capability with folders/tags/notes and stable entry references plus fallback snapshot when a dictionary is removed. Keep ledger status as a requested extension until reference runtime evidence exists.
- Implement dictionary order and separate query/index/capture groups.

**Acceptance tests**

- Migration from every previous schema fixture is transactional, idempotent, backup-producing, and rollback-safe.
- History cap/dedupe/order survive restart; clearing history does not clear favorites; private mode writes nothing.
- Favorites survive dictionary disable/removal and indicate stale source without losing the saved snapshot.
- Search result ordering and group membership pass a multilingual oracle corpus and disabled dictionaries never leak through FTS joins.
- Property tests cover Unicode normalization, grapheme truncation, and SQL/FTS special characters.

### Milestone 3 — LD2/LDX clean-room format program and export

Treat `.ld2`, `.ldx`, `.ldp`, and `.ldb` as distinct until evidence proves relationships. The inspected tree has LDX specimens but no LD2 specimen. Do not rename or guess one format as another.

**Phase A: specification and read-only parser**

- Produce a byte-level format note solely from lawful public documentation, independently generated files, metadata observation, and input/output experiments.
- Identify magic/version, endianness, offsets, compression/encryption flags, string encodings, entry index, redirects/synonyms, record blocks, resource tables, metadata, and checksums.
- Implement `Read + Seek` parsers with checked arithmetic, bounded allocation/decompression, non-recursive resource traversal, structured diagnostics, and no unsafe code.
- Quarantine unsupported encrypted/protected variants with a precise error; do not bypass access controls.

**Phase B: importer**

- Stream entries/resources into staging tables; validate references/checksums; atomically publish after complete success.
- Retain raw metadata needed for faithful semantic export only when licensing allows; never retain arbitrary executable content.
- Expose dry-run inspection and an import report listing dropped/unsupported constructs.

**Phase C: exporter**

- First define and ship a documented, open Lingoes RS exchange package (ZIP with canonical UTF-8 manifest, records, resources, hashes) and TSV/StarDict export.
- Implement LD2/LDX export only for dialects proven by independently authored golden files and reference round-trip tests. If no lawful complete specification is obtained, leave proprietary export `deferred` and report that clearly; import compatibility does not imply writable compatibility.
- Export deterministically: stable ordering, normalized paths, fixed metadata policy, collision checks, and reproducible hashes.

**Acceptance tests**

- Golden files cover every supported version/encoding/compression/resource mode and compare entry/redirect/resource semantics, not proprietary bytes.
- `import -> open exchange export -> import` preserves canonical metadata, entry order where meaningful, rich document AST, synonyms, and resource hashes.
- A proven writable LD2/LDX dialect opens in the reference application and matches expected lookups/resources; evidence records the exact reference build and platform.
- Truncation at every structural boundary, offset overflow, zip bombs, invalid UTF, duplicate paths, traversal, and randomized fuzz input never panic, escape staging, or partially publish.
- Parser fuzz targets run in CI smoke mode and scheduled long mode with a retained regression corpus.

### Milestone 4 — Rich dictionary document model and resources

**Implementation**

- Replace `definition: String` as the storage/rendering boundary with a typed document: paragraphs, spans, headings, lists, tables, phonetics, links, images, audio actions, cross-references, and unknown-safe nodes.
- Parse HTML/XML-like records into a DOM, sanitize to an explicit element/attribute/style allowlist, then lower into the document AST. Strip scripts, event handlers, plugins, forms, active SVG, remote CSS, and unsafe URL schemes.
- Resolve `dictres://<dictionary-id>/<normalized-path>` through an immutable resource store. Enforce MIME sniffing, maximum decoded dimensions/bytes, and no filesystem path exposure.
- Render the safe common subset natively in Slint. Prototype a sandboxed embedded web renderer only for constructs the native renderer cannot represent and only if acceptance/security results justify it. If used on Windows, prefer WebView2 with a locked virtual host, CSP, disabled devtools/context menus/downloads, blocked external navigation, and an explicit host allowlist; use a `RichRenderer` trait so other platforms can choose a different adapter.
- Cross-reference clicks go through the application router; external links require confirmation and the OS browser.

**Acceptance tests**

- A fixture gallery covers nested formatting, tables, RTL runs, ruby/phonetics, images, missing resources, pronunciation audio, links, and redirects.
- Semantic snapshots of the document AST are stable across platforms; Windows screenshots use tolerance masks and are evidence of regressions, not proof of reference pixel identity.
- XSS/SSRF/path traversal/resource bomb suites show no script execution, loopback/file fetch, filesystem read, or unbounded decode.
- A 100-entry rich result renders without UI-thread I/O and scrolling/input remain responsive under the agreed performance budget.

### Milestone 5 — Windows capture stack: clipboard, UI Automation, selection, OCR

**Capture order**

1. Explicit user selection/clipboard event when that mode is enabled.
2. UI Automation text under cursor.
3. Legacy accessibility fallback for compatible controls.
4. OCR of a bounded cursor-centered region only when enabled and higher layers return no usable text.

**Implementation**

- Replace clipboard polling on Windows with `AddClipboardFormatListener` and `WM_CLIPBOARDUPDATE`; tag self-writes to prevent feedback loops. Keep the polling adapter for other platforms.
- Run UIA on a dedicated COM MTA worker. Get cursor point and target element, request `TextPattern2`/`TextPattern`, obtain a degenerate/range-at-point where supported, expand to word/phrase boundaries, and return text plus bounding rectangle and source metadata. Apply strict per-call timeout/cancellation because providers can hang.
- Use `SetWinEventHook` for selection-related accessibility events where reliable. Do not inject hooks or DLLs into foreign processes. A user-triggered “translate selection” hotkey may use an explicit, reversible copy gesture only as an opt-in fallback: preserve clipboard formats, synthesize copy with `SendInput`, wait boundedly, restore clipboard, and never do this for password/secure fields.
- Implement process executable and window-class allow/deny rules corresponding to the observed filter concept; default-deny password controls, secure desktops, hidden windows, and the app itself.
- OCR fallback captures only a configurable region around the pointer. Convert to `SoftwareBitmap`, select an installed `OcrEngine` language, run OCR, choose the word/line intersecting the pointer, and emit confidence/provenance. No screenshot is persisted by default.
- Use script filters for Arabic/Hebrew, CJK, Cyrillic, Latin, and numbers, matching configured intent without copying old activation encodings.

**Acceptance tests**

- Fixture matrix: Win32 EDIT/RichEdit, WPF TextBox/FlowDocument, WinUI 3, Chromium input/contenteditable/canvas, Firefox, Office if licensed, common PDF viewer, terminal, remote desktop, elevated target, password field, mixed-script text, and app-owned windows.
- For UIA-capable fixtures, expected word and bounding rectangle are returned at 100/150/200% DPI with per-scenario success thresholds recorded in the ledger.
- A hung UIA provider times out without freezing UI; rapid pointer movement cancels stale lookups; 1,000 captures show no COM/GDI handle leak.
- OCR corpus records character/word error rates by language and rejects results below configured confidence. The UI labels OCR-derived text.
- Clipboard contents and all formats are preserved after selection fallback, including failure paths; secure/password targets are never copied or OCR’d.
- Unelevated-to-elevated limitations are reported, not bypassed. No feature requires administrator rights.

### Milestone 6 — Popup and mini-window behavior

**Implementation**

- Build a separate popup window, not an overlay inside the main window. On Windows apply a non-activating tool-window policy (`WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW`, with topmost only while visible) through a small window adapter after Slint creates the native window.
- Place against the captured text rectangle first, cursor second; select the monitor work area; flip above/below and clamp to edges with per-monitor DPI conversion.
- State machine: `Hidden -> Pending -> VisibleTransient -> Pinned/Expanded -> Hidden`. Newer capture tokens supersede stale jobs.
- Support open full result, dictionary sections, previous/next result, copy, speak/stop, pin/keep-visible, close, and optional auto-pronounce. Mini window supports stored size, roll-up, and always-visible policies if runtime evidence confirms behavior.
- Never steal focus for hover capture. Explicit keyboard invocation may focus the query field by documented policy. Suppress popup over the app and during fullscreen/presentation according to settings.

**Acceptance tests**

- Foreground HWND and caret remain unchanged for passive popup display and dismissal across fixture apps.
- Popup never renders off the usable work area on negative-coordinate, portrait, mixed-DPI, taskbar-edge, and monitor-removal scenarios.
- Stale lookup/OCR results cannot replace newer content; Esc, outside click, pin, and open-main transitions match the recorded state table.
- Popup supports keyboard-only operation, screen-reader names, high contrast, reduced motion, and 200% text scaling.
- Side-by-side reference comparisons record differences; no pixel-parity claim is made without same-machine measured evidence.

### Milestone 7 — Native TTS, translation, web search, print, and save

**Implementation**

- Replace PowerShell Windows TTS with a native adapter. Enumerate voices; map BCP-47 language, voice, rate, pitch, and volume; support stop/cancel and auto-pronunciation policies. Keep portable `TtsEngine` traits and non-Windows adapters.
- Move all translation calls off the UI thread. Add timeouts, cancellation, response-size limits, retry policy only for idempotent/transient failures, language discovery, proxy settings, and clear provider attribution.
- Replace legacy search templates with user-configurable HTTPS templates validated with `url`; URL-encode terms and open in the default browser. Do not ship obsolete HTTP endpoints from extracted config.
- Implement find/select-all/copy on the rich document model.
- “Save” writes UTF-8 plain text, sanitized self-contained HTML, or PDF. Self-contained HTML inlines or copies only permitted resources and includes source dictionary attribution.
- Implement a Windows print prototype using a system print dialog and paginated rich-document layout. Select the lowest-complexity API that passes printer and Microsoft Print to PDF tests; isolate it behind `PrintService`.

**Acceptance tests**

- Native TTS voice listing, language choice, rate/pitch/volume boundaries, stop, device failure, and 100 repeated speak/cancel cycles pass without shelling out.
- Translation mock tests cover cancellation, proxy auth, 401/429/5xx, malformed/oversized response, offline mode, and secret redaction. Live tests are opt-in and never gate deterministic CI.
- Saved HTML opens offline with formatting/resources intact and contains no script, remote fetch, absolute local path, or secret.
- Print preview/PDF has correct pagination, Unicode/RTL text, images, headers, and no clipped content at Letter/A4 and portrait/landscape.

### Milestone 8 — Localization, RTL, accessibility, and input methods

**Implementation**

- Extract every user-facing string into Fluent resources with English as the source locale. Existing hard-coded Vietnamese becomes a translation, not the source contract.
- Generate independent translations; extracted localization files may guide feature inventory but no strings are copied.
- Negotiate BCP-47 locales with fallback chains and allow runtime language switching where Slint permits safe rebind.
- Expose layout direction as application state. Mirror navigation/order/icons that carry direction, but do not mirror universal symbols. Use Unicode bidi isolation around user/dictionary content and keep mixed-direction content in the rich document model.
- Format dates, numbers, and plural messages by locale. Verify CJK fonts, grapheme-aware selection, IME composition, and screen-reader labels.

**Acceptance tests**

- Pseudo-locales catch clipping and unlocalized strings; a key-consistency test rejects missing/unused keys.
- Arabic, Hebrew, Persian, and Uyghur smoke suites verify mirrored layout, tab order, caret/navigation, mixed LTR/RTL headwords, digits, punctuation, and popup placement.
- Japanese/Chinese/Korean and Vietnamese IME composition does not trigger lookup on incomplete composition unless explicitly configured.
- Windows Narrator can discover actionable controls, state, errors, result counts, and popup content; keyboard-only navigation has no traps.

### Milestone 9 — Independent appendices replacement

Implement appendices as signed/manifested internal tools over typed Rust APIs, not arbitrary local HTML/JavaScript packages. A future third-party extension format requires a separate sandbox design.

- Calculator: expression parser (`pest` or `nom`) plus `rust_decimal`; explicit angle/base modes and no `eval`.
- Measures: dimension-checked conversion with exact rational/decimal constants and documented standards.
- Time zones: `chrono` + `chrono-tz`, including DST ambiguity/nonexistence.
- Dialing codes and dictionary abbreviations: independently sourced, versioned, redistributable datasets with provenance.
- Exchange rates: provider trait, HTTPS, timestamp/source display, cache, stale/offline state; no claim of financial advice.
- Periodic table and IPA: independently licensed datasets; accessible tables/search.
- Simplified/traditional Chinese: independently licensed OpenCC-compatible data through a Rust adapter; variant choice is explicit.
- Number-to-Chinese: locale-specific deterministic rules and range limits.
- Verb reference: rules/reference data with source attribution; do not imply exhaustive conjugation without tests.
- Hashes: RustCrypto `md-5`, `sha1`, `sha2`; clearly label MD5/SHA-1 as non-secure.
- Base64: `base64` crate with standard/URL-safe modes and strict/forgiving decode options.

**Acceptance tests**

- Each tool has published test vectors and provenance/license metadata.
- Conversion round-trips and boundary/property tests pass; time-zone DST and decimal precision cases are explicit.
- Network tools show source time and stale state, obey proxy/offline mode, and never silently substitute cached data as current.
- Appendix launcher, ordering, enable/disable, search, and keyboard access persist across restart.
- No extracted HTML, JS, images, data, names, or translations ship in the replacement.

### Milestone 10 — Migration, backup, security hardening, and recovery

**Implementation**

- Create a standalone migration assistant with scan/dry-run/report/apply/rollback phases. It recognizes known legacy directories and settings encodings, imports only supported user-created/configuration data and lawful dictionaries, and never modifies the source installation.
- Map legacy settings semantically; do not preserve obfuscated credentials or opaque activation/update state. Re-prompt for secrets and store through `SecretStore`.
- Add native backup/restore using the open exchange format: manifest version, SQLite consistent backup, dictionaries by reference or embedded choice, resource hashes, settings excluding secrets by default.
- Harden importers and renderers: limits for file count, entry count, nesting, compressed ratio, decoded image pixels, text/record size, and total temporary space.
- Enforce TLS, redirect/host policy, proxy handling, log redaction, crash-report opt-in, and offline mode. No local API server is enabled by default; if added, bind loopback, require a random token, restrict origins, and threat-model it separately.
- Publish a threat model covering untrusted dictionaries, HTML/resources, OCR privacy, clipboard/selection sensitivity, network providers, IPC, updater/package supply chain, and portable removable media.

**Acceptance tests**

- Migration fixtures cover UTF-16 INI, missing/corrupt files, duplicate dictionaries, unsupported formats, partial disk, interrupted process, and repeat runs. Source hashes remain unchanged.
- Restore into an empty profile recreates non-secret settings, history/favorites choices, dictionary order/groups, and resource hashes; corrupt/tampered backups fail atomically.
- Security regression corpus and fuzzers produce no panic, path escape, active content execution, credential disclosure, or partial database state.
- Logs and crash bundles pass secret/PII scans; clipboard/OCR text is absent unless diagnostic consent explicitly includes it.

### Milestone 11 — Portable Windows packaging and release qualification

**Portable ZIP layout**

```text
<independent-brand>/
  app.exe
  portable.toml
  data/                 writable databases and user state
  dictionaries/         user-managed packages
  resources/            independently created app resources/locales
  licenses/             dependency and dataset notices
  README.txt
  SHA256SUMS
```

**Implementation**

- Build MSVC x64 as primary and ARM64 when all dependencies pass. Decide x86 only from user demand and verified dependencies.
- Produce deterministic portable ZIP and optional signed installer. Embed a new icon/version manifest, `asInvoker` execution level, long-path/DPI awareness, and supported OS metadata.
- Portable mode never registers file associations, autostart, updater tasks, or uninstall records automatically. Installed mode offers those as explicit options with reversible operations.
- Sign executables/packages with Authenticode in release infrastructure, timestamp signatures, publish SHA-256 sums, SBOM, dependency licenses, provenance/attestation, and reproducible-build notes.
- Support in-place portable upgrades by replacing immutable app files while preserving `data/`; make pre-migration backup and downgrade refusal explicit.

**Acceptance tests**

- Unzip and run as a standard user on clean Windows 10 and 11 VMs with no Rust, PowerShell-script dependency, WebView assumption (unless packaged/detected), or admin rights.
- Paths containing spaces, non-ASCII, long paths, read-only media, network shares, FAT/exFAT/NTFS, and removable drives yield documented behavior and no data loss.
- No unexpected writes occur outside the portable tree; ProcMon evidence is attached with OS-generated exceptions documented.
- Signature, timestamp, checksums, SBOM, licenses, malware scan, and archive reproducibility checks pass.
- Upgrade from the previous two releases preserves data; simulated interruption restores from backup or leaves the old version usable.

## 7. Windows-native test matrix

A row can be marked passed only with an artifact from that exact cell. Manual exploratory results do not replace automation, and unavailable proprietary applications are marked `NOT RUN`.

### Operating systems and architectures

| Axis | Required cells |
|---|---|
| Windows | Windows 10 22H2 x64; Windows 11 current stable x64; Windows 11 ARM64 for ARM release |
| Session | Local desktop; Remote Desktop; locked/unlocked transition; standard user |
| Integrity | App and target same medium integrity; unelevated app vs elevated target (expected limitations); never require elevation |
| Display | 96/144/192 DPI; mixed-DPI dual monitor; negative monitor coordinates; portrait; high contrast; text scaling 200% |
| Locale/input | English, Vietnamese, Simplified Chinese, Japanese IME, Arabic RTL, Hebrew RTL |
| Graphics | Hardware acceleration on/off; WARP/VM adapter |
| Packaging | Portable NTFS, removable FAT/exFAT, read-only directory, installed mode |

### Target application capture matrix

| Target/control | Clipboard | selection | UIA cursor | OCR fallback | Popup focus |
|---|---:|---:|---:|---:|---:|
| Native Win32 Edit/RichEdit | required | required | required | fallback test | required |
| WPF | required | required | required | fallback test | required |
| WinUI 3 | required | required | required | fallback test | required |
| Chromium browser | required | required | required where exposed | required for canvas | required |
| Firefox | required | required | required where exposed | fallback | required |
| PDF viewer | required | scenario-based | scenario-based | required | required |
| Office desktop (licensed runner) | required | required | scenario-based | fallback | required |
| Terminal/console | required | selected-text hotkey | capability recorded | fallback | required |
| Password/secure control | prohibited capture | prohibited | prohibited | prohibited | no popup |
| Elevated target | capability recorded | capability recorded | capability recorded | user-consented only | no focus theft |
| Remote Desktop content | capability recorded | capability recorded | capability recorded | policy-controlled | required |

### Reliability/performance budgets

Set final thresholds from reference/runtime evidence and hardware class; until then use provisional engineering gates:

- Main window warm show: p95 under 250 ms on reference CI hardware.
- Prefix lookup on a 1-million-entry indexed corpus: p95 under 50 ms after warm-up.
- UIA capture to popup content: p95 under 350 ms for responsive providers; hard timeout 1 second.
- OCR fallback: p95 under 1.5 seconds for the bounded region; always cancellable.
- Idle tray mode: no continuous high-frequency polling on Windows and less than 1% average CPU on the reference runner.
- Import cancellation/failure leaves no published partial dictionary.
- 8-hour soak: stable thread, USER, GDI, COM, file-handle, and memory trends within documented bounds.

These are release budgets, not claims about the reference application.

## 8. CI and test strategy

- **Every PR:** format, clippy, unit/integration/doc tests, ledger validation, schema migration matrix, parser regression corpus, sanitizer/security tests, dependency policy.
- **Windows PR gate:** x64 build/tests, native shell tests, headless view-model tests, smoke launch, package layout test.
- **Nightly:** UIA fixture matrix, OCR language corpus, screenshot regression, fuzz smoke, million-entry benchmark, print-to-PDF.
- **Weekly/release:** Windows OS/DPI/locale matrix, Office/PDF licensed runners, RDP, ARM64, 8-hour soak, portable ProcMon audit, migration/upgrade/downgrade, signatures/SBOM/reproducibility.
- Use semantic assertions before screenshots: window role/state/text, focus HWND, rectangle bounds, document AST, result ordering, and resource hashes. Screenshot thresholds catch regressions but cannot establish functional correctness alone.
- Network tests use local mock servers. Live providers and exchange-rate feeds are opt-in and their changing output is never compared byte-for-byte.

## 9. Security and privacy requirements

- Captured clipboard, selection, UIA, and OCR text is ephemeral unless history is enabled. Private mode bypasses persistence and online translation.
- OCR is disabled by default until onboarding explains screen-region capture. Never run on secure desktop or password controls; never retain captures by default.
- Dictionary inputs are untrusted. Parsing and decompression are bounded; imports stage transactionally; resources cannot access filesystem/network by default.
- Rich content has no script/plugin execution. Remote resources are blocked unless the user explicitly opens an external link.
- API/proxy credentials never enter TOML/SQLite/logs/backup by default. Store Windows secrets in Credential Manager; portable users may choose session-only credentials or an encrypted vault with a user-provided passphrase. Do not silently weaken storage to plaintext for portability.
- IPC authenticates the same user and validates message size/schema. Named objects use per-user names and restrictive security descriptors.
- Update checks are opt-in for portable mode, HTTPS-only, signed-manifest based, and never execute downloaded content without signature verification.
- Publish dependency/data licenses and a vulnerability response process. Reject dependencies or datasets incompatible with project licensing.

## 10. Release gates and definition of parity

A “full parity candidate” requires all of the following:

1. Every in-scope ledger row is `verified` on the required Windows matrix or has an approved, user-visible deviation. No `inferred` row is counted as complete.
2. LD2/LDX claims list exact supported dialects and fixtures. Unsupported encryption/version/resource forms are disclosed.
3. Capture success rates and limitations are published by target application/control; OCR provenance is visible.
4. Popup focus/placement, tray/hotkeys, history/favorites, print/save, localization/RTL, migration, and portable packaging pass their milestone tests.
5. Security tests, parser fuzzing, threat-model review, license review, SBOM, signing, backup/restore, and upgrade tests pass.
6. Side-by-side runtime scenarios have been executed against Lingoes 2.9.2 Portable on the same Windows configurations for behaviors claimed equivalent.
7. Visual comparisons are described as measured regressions. “Exact,” “identical,” or “pixel-perfect” is prohibited unless a published scenario and tolerance proves it; otherwise use “functionally compatible within the recorded scenarios.”

## 11. Recommended delivery order and dependencies

1. **M0 evidence/ledger** prevents unsupported parity claims.
2. **M1 shell/platform boundary** unlocks Windows lifecycle work without contaminating core crates.
3. **M2 data model** must precede richer import/render/history features.
4. **M3 format research/import** and **M4 document rendering** iterate together on independent fixtures.
5. **M5 capture** depends on platform shell and feeds **M6 popup**.
6. **M7 output/services** depends on the document model and settings/security.
7. **M8 localization/RTL** should begin string extraction during M1, then receive a dedicated qualification pass.
8. **M9 appendices** can proceed in parallel after settings/navigation contracts stabilize.
9. **M10 migration/security** starts threat modeling in M0 but completes after schemas/formats settle.
10. **M11 packaging** runs continuously in CI, with final qualification after all prior gates.

Avoid a single “big bang” parity release. Ship preview builds with ledger-derived labels such as `lookup verified`, `UIA capture preview`, and `LDX dialect X read-only`; this keeps claims aligned with evidence while converging on the full candidate.

## 12. Immediate next actions

1. Create the ledger/schema and populate the initial rows above with owners and scenario IDs.
2. Build the Windows reference-observation VM and independently authored control fixture suite.
3. Prototype the `windows-rs` message-only window, tray, and one hotkey behind traits without changing core domain crates.
4. Redesign the entry schema around records/resources/document AST before expanding importers.
5. Acquire or independently generate lawful LD2 fixtures; until then keep LD2 status `evidence gap` and do not promise export.
6. Add Windows x64 CI and deterministic portable ZIP smoke testing before merging additional Windows-only functionality.
