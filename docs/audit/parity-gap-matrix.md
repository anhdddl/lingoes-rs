# Lingoes 2.9.2 parity gap matrix

**Rust baseline:** `793bbbcb3aeef98451c6438e135e108eb2fe5d67` (`main`)  
**Original baseline:** extracted official Lingoes 2.9.2 ZIP, MD5 `9A88E062A1C5B489C59BA36B59350B06`  
**Audit date:** 2026-10-14

## Scope and method

This is a clean-room, static audit of the Rust source, tests, UI definition, and documentation against features observable in the extracted Lingoes package. The original Windows executables and plug-ins were **not executed**. Original behavior is therefore established only where package resources make it observable: English UI strings, default configuration, XML/HTML resources, file inventory, and binary file identity. A label or packaged component proves that Lingoes exposes or ships a capability; it does not prove every runtime detail.

Rust citations refer to repository-relative paths. Original citations are relative to `/home/longpc/lingoes-original-audit/extracted/` and are prefixed **Original:**.

### Classification

- **Complete** — the observable user workflow is implemented end to end in Rust at materially equivalent scope. It need not reuse Lingoes internals.
- **Partial** — a related working path exists, but important original behavior, formats, presentation, controls, or integration is absent.
- **Missing** — no product implementation was found after bounded searches of all eight Rust sources, the Slint UI, manifests, README, and plan.
- **Unverifiable** — package evidence indicates a feature/component, but static evidence cannot establish original runtime behavior closely enough to compare it honestly. This is also used where native Windows behavior cannot be proved on the Linux audit host.

“Complete” is intentionally strict. A unit-tested clipboard watcher, for example, is not equivalent to cursor capture.

## Executive result

The Rust product is a functional cross-platform dictionary MVP, not a feature-identical Lingoes 2.9.2 reconstruction. Its strongest implemented slices are SQLite-backed lookup, TSV/StarDict/MDict ingestion, basic dictionary enable/delete management, Google/OpenAI-compatible text translation, system-command TTS, clipboard copy, and clipboard-triggered lookup. Most desktop-shell, native capture, rich dictionary rendering, history/notebook, configuration, appendix, packaging, and browser/PDF integration remains absent.

| Status | Count |
|---|---:|
| Complete | 5 |
| Partial | 26 |
| Missing | 72 |
| Unverifiable | 5 |
| **Total rows** | **108** |

The counts describe matrix rows, not weighted completion. Several rows aggregate a tightly related original workflow.

## 1. Desktop UI, navigation, and lookup workflows

| ID | Observable Lingoes 2.9.2 feature | Rust status | Evidence and gap |
|---|---|---|---|
| UI-01 | Native main window with close/maximize/minimize/restore and main menu | **Partial** | **Original:** `app_data/lang/lang-en.ini:11-40` defines the frame and menu. **Rust:** `apps/lingoes-desktop/ui/app-window.slint:18,81-164` defines one Slint window with custom tab header, but no Lingoes-style main menu or explicit window command surface. Normal window-manager controls may be supplied by Slint, but exact Windows behavior is not established. |
| UI-02 | Main menu workflows: open, capture toggles, configuration, dictionaries, appendices, activation, update, help, support, exit | **Missing** | **Original:** `app_data/lang/lang-en.ini:27-41`. **Rust:** the only top-level navigation is four tabs (`apps/lingoes-desktop/ui/app-window.slint:96-164`); no menu callbacks exist among `apps/lingoes-desktop/ui/app-window.slint:65-75`. |
| UI-03 | Back/forward navigation and history list in the search view | **Missing** | **Original:** `app_data/lang/lang-en.ini:43-52`. **Rust:** lookup UI has query, search/full-text buttons, result list, and detail pane (`apps/lingoes-desktop/ui/app-window.slint:180-306`) but no back, forward, or history callback/state in `DesktopApp` (`apps/lingoes-desktop/src/lib.rs:50-63`). |
| UI-04 | Sidebar/index with Index, Guide, Appendix; show/hide sidebar | **Missing** | **Original:** `app_data/lang/lang-en.ini:54-70,127-129`; `app_data/data/def_user.ini:6,12` persists index width/visibility. **Rust:** lookup uses a fixed result/detail split (`apps/lingoes-desktop/ui/app-window.slint:200-316`) and has no guide/appendix modes or persisted sidebar state. |
| UI-05 | Prefix/index lookup with automatic closest-match suggestions | **Partial** | **Original:** `app_data/lang/lang-en.ini:47-59,197-203,260-263` describes auto-match and a dedicated Index Group. **Rust:** `SqliteDictionary::lookup_prefix` does normalized prefix matching and exact-first ordering (`crates/lingoes-dictionary/src/lib.rs:399-418`), called by `DesktopApp::search` (`apps/lingoes-desktop/src/lib.rs:106-139`). There is no separate Index Group or configurable auto-match behavior. |
| UI-06 | Detailed explanation search and full-text/find-in-result workflows | **Partial** | **Original:** `app_data/lang/lang-en.ini:48,120-126,166,172,517` exposes detailed search and Find. **Rust:** SQLite FTS5 definition search exists (`SqliteDictionary::lookup_full_text`, `crates/lingoes-dictionary/src/lib.rs:421-439`) and a GUI button invokes it (`apps/lingoes-desktop/ui/app-window.slint:190-194`), but there is no in-result Find dialog or Lingoes detailed-result navigation. |
| UI-07 | Search result list and dictionary result pane | **Partial** | **Original:** `app_data/lang/lang-en.ini:488-495,512-531` describes spelling, multiple results, show/minimize result, dictionary menu, and result arrangement. **Rust:** result list and selected definition are present (`apps/lingoes-desktop/ui/app-window.slint:200-316`; `DesktopApp::select`, `apps/lingoes-desktop/src/lib.rs:142-176`). No spelling alternatives, collapse/minimize, per-result dictionary menu, or arrangement controls exist. |
| UI-08 | Correct source-dictionary attribution in result presentation | **Partial** | **Original:** dictionary-specific menus and ordered results are exposed at `app_data/lang/lang-en.ini:512-531`. **Rust:** domain entries preserve `dictionary_id/name` (`crates/lingoes-core/src/lib.rs:37-70`), but `update_window` discards it by assigning `dictionary_name: ""` and the constant badge `Lingoes DB` (`apps/lingoes-desktop/src/lib.rs:619-651`). |
| UI-09 | Spelling alternatives/spell checker | **Missing** | **Original:** `app_data/lang/lang-en.ini:62-64,488-490`. No spell-check model, UI, or dependency appears in Rust source/manifests. Prefix matching is not a spell checker. |
| UI-10 | Web search from the query workflow with selectable engine | **Missing** | **Original:** `app_data/lang/lang-en.ini:49,52,296-297`; `app_data/webe/webengine.xml` enumerates locale-specific Google, Bing, Yahoo, dictionary, encyclopedia, and other engines; `app_data/data/def_user.ini:15-16` stores the engine. **Rust:** no web-search model, browser launch callback, or parsing of `webengine.xml` exists. Online translation is a separate function. |
| UI-11 | Start page/guide and online support links | **Missing** | **Original:** `app_data/lang/lang-en.ini:61-71,127,497-501` and `app_data/update/update.xml` contain start-page/guide markup. **Rust:** no start page or support/home/community link surface exists. |
| UI-12 | Mini window with keep-on-top, keep-size, roll-up, auto-roll-up, back/forward, copy and speech | **Missing** | **Original:** `app_data/lang/lang-en.ini:111-116,165-178`; `app_data/data/def_user.ini:7-10` persists mini-window settings. **Rust:** the quick-popup card (`apps/lingoes-desktop/ui/app-window.slint:678-737`) is drawn inside the main Slint window and has only close, detail, and speech actions; no independent mini-window lifecycle or controls exist. |
| UI-13 | Text-capture popup/quick result | **Partial** | **Original:** `app_data/lang/lang-en.ini:165-178,330-335` describes a configurable capture window. **Rust:** `DesktopApp::handle_captured_text` populates a quick card (`apps/lingoes-desktop/src/lib.rs:353-381`; `apps/lingoes-desktop/ui/app-window.slint:678-737`). It is driven only by changed clipboard text, lacks native cursor/selection sources and most original popup behavior, and is not an independent OS popup. |
| UI-14 | Keyboard workflow shortcuts in query box (Enter, Ctrl+Enter, Shift+Enter, Alt+Enter) | **Missing** | **Original:** `app_data/lang/lang-en.ini:172` explicitly documents four actions. **Rust:** buttons exist, but no keyboard event handlers are defined in `apps/lingoes-desktop/ui/app-window.slint`; no global or query-specific accelerator subsystem exists. |
| UI-15 | Vietnamese-only modern tab layout versus localized Lingoes visual structure | **Partial** | **Original:** 44 `app_data/lang/lang-*.ini` files, direction metadata (`lang-en.ini:1-6`), and configurable style/fonts (`lang-en.ini:318-339`). **Rust:** all visible strings are hard-coded Vietnamese in `apps/lingoes-desktop/ui/app-window.slint:18,87-748`, with four modern tabs. Functional areas overlap, but visual hierarchy, menus, index/guide pane, compact mini window, styling, and language adaptability are not parity. |

## 2. Dictionary formats, management, lookup, and presentation

| ID | Observable Lingoes 2.9.2 feature | Rust status | Evidence and gap |
|---|---|---|---|
| DICT-01 | Native Lingoes dictionary install formats `.ld2` and `.ldp` | **Missing** | **Original:** `app_data/lang/lang-en.ini:180-219,211` specifies `.ld2/.ldp`; the package includes eight `.ldx` dictionary payloads under `app_data/dict/`, including five preinstalled resources. **Rust:** `DictionaryFormat` accepts only TSV, StarDict, and MDict (`crates/lingoes-dictionary/src/lib.rs:41-84`). No LD2/LDP/LDX parser exists; the project plan still lists LD2 as a next milestone (`docs/plans/2026-10-13-mvp-foundation.md:60`). |
| DICT-02 | Import UTF-8 TSV dictionaries | **Complete** | This is an added Rust format rather than an original native format. `import_tsv_with_progress_and_name` parses tab-separated records transactionally (`crates/lingoes-dictionary/src/lib.rs:190-255`), with malformed/atomicity tests at `:729-795`. GUI and CLI expose import (`apps/lingoes-desktop/src/lib.rs:217-250,449-474,682-705`; `apps/lingoes-desktop/ui/app-window.slint:440-461`). |
| DICT-03 | Import StarDict (`.ifo/.idx/.dict/.dict.dz`) | **Complete** | Rust detects these extensions and opens `StarDictDictionary` (`crates/lingoes-dictionary/src/lib.rs:64-84,307-317`) with fixture-backed auto-detect/import and malformed-input tests (`:820-876`). This expands format support but does not replace native Lingoes-format parity. |
| DICT-04 | Import MDict MDX v2 | **Partial** | Rust detects `.mdx`, opens an MDict directory through `opendict`, and has a fixture-backed v2 test (`crates/lingoes-dictionary/src/lib.rs:78-80,318-321,884-972`). It flattens each returned record to trimmed UTF-8 text (`:345-370`); MDD resources, non-UTF-8 records, scripts/styles/audio, and faithful rich rendering are not implemented. |
| DICT-05 | Install/download/uninstall dictionaries | **Partial** | **Original:** `app_data/lang/lang-en.ini:180-219,254-274` supports disk install, online download, uninstall, add/remove, and backup import/export. **Rust:** GUI import, list, enable/disable, and delete work (`DesktopApp::{import_path_with_name,list_dictionaries,toggle_dictionary,delete_dictionary}`, `apps/lingoes-desktop/src/lib.rs:185-253`; manager UI `apps/lingoes-desktop/ui/app-window.slint:430-533`). There is no online catalog/download, add/remove distinction, package installer, file association, or backup workflow. |
| DICT-06 | Dictionary metadata/details (title, language, publisher, website, copyright, description) | **Missing** | **Original:** `app_data/lang/lang-en.ini:230-253` exposes a details dialog. **Rust:** `DictionaryMeta` stores only id, name, format, source path, enabled, entry count, and creation time (`crates/lingoes-dictionary/src/lib.rs:14-23`); UI displays the smaller set (`apps/lingoes-desktop/ui/app-window.slint:471-531`). |
| DICT-07 | Query, Index, and Text Capture dictionary groups | **Missing** | **Original:** `app_data/lang/lang-en.ini:195-203,260-263,529-531` provides distinct ordered groups. **Rust:** one global `enabled` boolean governs all searches (`crates/lingoes-dictionary/src/lib.rs:154-162,385-439`); no group schema or group-specific lookup exists. |
| DICT-08 | Reorder dictionaries / set top result / arrange results | **Missing** | **Original:** `app_data/lang/lang-en.ini:188-191,527-528`. **Rust:** dictionaries are ordered by database id and results by fixed SQL clauses (`crates/lingoes-dictionary/src/lib.rs:130-151,385-439`); no rank/order property or UI callback exists. |
| DICT-09 | Enable/disable dictionaries | **Complete** | **Original:** `app_data/lang/lang-en.ini:523-526`. **Rust:** `toggle_dictionary` persists `enabled`, and all lookup SQL filters it (`crates/lingoes-dictionary/src/lib.rs:154-162,385-439`); GUI offers per-dictionary toggle (`apps/lingoes-desktop/ui/app-window.slint:523-527`). |
| DICT-10 | Exact/prefix and Unicode-normalized local lookup | **Complete** | Rust provides exact, exact-first prefix, Unicode normalization, and persistent SQLite storage (`crates/lingoes-core/src/lib.rs:76-96`; `crates/lingoes-dictionary/src/lib.rs:385-419`). Tests cover case, Unicode composition/case expansion, ordering, and persistence (`crates/lingoes-core/src/lib.rs:105-154`; `crates/lingoes-dictionary/src/lib.rs:668-779,1006-1034`). |
| DICT-11 | Multi-dictionary result aggregation | **Partial** | **Original:** result arrangement and dictionary groups imply aggregated per-dictionary output (`app_data/lang/lang-en.ini:195-203,512-531`). **Rust:** SQL returns entries across all enabled dictionaries with attribution (`crates/lingoes-dictionary/src/lib.rs:385-454`) but the GUI discards attribution and displays one flat definition (`apps/lingoes-desktop/src/lib.rs:619-651`). |
| DICT-12 | Rich HTML dictionary presentation, phonetics, images, links, audio, scripts, and expandable sections | **Missing** | **Original:** `app_data/dict/layout2.cfc` is a 25,884-byte presentation resource; `app_data/dict/image/` contains audio, phonetic, link, license, expand, and other assets; `lang-en.ini:512-531` defines dictionary layout actions. **Rust:** definitions are plain strings displayed in a Slint `Text`/scroll area (`apps/lingoes-desktop/ui/app-window.slint:300-313`). Import concatenates UTF-8 record bytes without a renderer/resource resolver (`crates/lingoes-dictionary/src/lib.rs:345-370`). |
| DICT-13 | Online dictionaries and local cache | **Missing** | **Original:** dictionary manager marks online types (`app_data/lang/lang-en.ini:204-209`), package contains online `.ldx` resources, and configuration exposes result cache (`:299-305`). **Rust:** all dictionary lookup uses local SQLite; network use is limited to translation clients. |
| DICT-14 | Dictionary backup export/import (`.ldb`) | **Missing** | **Original:** `app_data/lang/lang-en.ini:217-219`. Rust has no `.ldb` support or backup command/UI. |
| DICT-15 | Dictionary file associations (`.ld2`, `.ldp`, `.laf`) and migration from 2.8 | **Missing** | **Original:** `app_data/lang/lang-en.ini:311-316`. Rust has no OS association registration or legacy-data migration. SQLite schema migration (`migrate_or_init_schema`, `crates/lingoes-dictionary/src/lib.rs:458-565`) is internal and not equivalent. |
| DICT-16 | Bundled Concise English, synonym/antonym, English-German, Jukuu, WordNet, instant/text-translation dictionaries | **Missing** | **Original:** named `.ldx` files are shipped in `app_data/dict/` and `app_data/dict/pre_install/`. Rust repository has no bundled dictionaries or licensed/online dictionary bootstrap assets; startup creates an empty SQLite store (`DesktopApp::open`, `apps/lingoes-desktop/src/lib.rs:65-103`). |
| DICT-17 | Chinese simplified/traditional conversion databases and Unicode data | **Missing** | **Original:** `app_data/dict/chs2t.db`, `cht2s.db`, and `ucd.db`; appendix `app_data/appe/convzh/`. Rust normalizes Unicode for search but has no Chinese conversion service or appendix (`crates/lingoes-core/src/lib.rs:76-96`). |
| DICT-18 | Licensed/trial dictionary activation and purchase state | **Missing** | **Original:** `app_data/lang/lang-en.ini:133-143,207-209,518,587-624` exposes authorization, activation, purchase, and license import. Rust contains no licensing model or UI. |

## 3. Capture, selection, hotkeys, popup, and tray

| ID | Observable Lingoes 2.9.2 feature | Rust status | Evidence and gap |
|---|---|---|---|
| CAP-01 | Capture the word under the mouse cursor from arbitrary Windows applications | **Missing** | **Original:** `app_data/lang/lang-en.ini:21-31,351-368` and `app_data/data/def_user.ini:3-5` expose cursor capture independently of clipboard/selection. **Rust:** `lingoes-capture` only reads the clipboard (`SystemClipboardReader`, `crates/lingoes-capture/src/lib.rs:63-97`) and polls it (`ClipboardWatcher::spawn`, `:168-192`). Clipboard polling must not be represented as cursor capture. The plan names Windows UI Automation capture as future work (`docs/plans/2026-10-13-mvp-foundation.md:60`). |
| CAP-02 | Configurable mouse activation buttons and capture delay | **Missing** | **Original:** `app_data/lang/lang-en.ini:353-362`; defaults include `GrabActivation` and `GrabTimeDelay=200` (`app_data/data/def_config.ini:65-68`). Rust exposes only fixed 200 ms clipboard polling (`ClipboardWatcherConfig::default`, `crates/lingoes-capture/src/lib.rs:100-114`), not mouse activation or cursor delay. |
| CAP-03 | Phrase matching and script/number ignore filters | **Missing** | **Original:** `app_data/lang/lang-en.ini:363-368`; defaults at `app_data/data/def_config.ini:69-76`. Rust only trims and enforces generic character-count bounds (`ClipboardWatcher::poll_step`, `crates/lingoes-capture/src/lib.rs:144-165`). |
| CAP-04 | Translate selected text and double-click selection | **Missing** | **Original:** `app_data/lang/lang-en.ini:369-372`; selection mode defaults in `def_user.ini:3-5`. Rust has no OS selection monitor, accessibility/UI Automation integration, or double-click hook. |
| CAP-05 | Per-program/per-window selection filter rules and window-class capture tool | **Missing** | **Original:** `app_data/lang/lang-en.ini:380-403`. No process/window identity model or capture-filter UI exists in Rust. |
| CAP-06 | Clipboard text translation/lookup | **Partial** | **Original:** clipboard translator is independently toggleable (`app_data/lang/lang-en.ini:21-31,443-445`) and has activation settings (`def_config.ini:65-76`). **Rust:** GUI can start `ClipboardWatcher` and look up changed clipboard content (`apps/lingoes-desktop/src/lib.rs:579-612`; `handle_captured_text`, `:353-365`). It performs dictionary lookup, not the original complete translation/capture workflow, and settings are not persisted. |
| CAP-07 | Native/global hotkeys for window, capture modes, pronunciation, copy, selected-text speech, stop, mini-window roll-up | **Missing** | **Original:** complete action list in `app_data/lang/lang-en.ini:440-452`, with encoded defaults in `app_data/data/def_config.ini:33-44`. No hotkey dependency, registration code, or settings UI appears in Rust manifests or source. |
| CAP-08 | System tray/notification icon with mode status and commands | **Missing** | **Original:** `app_data/lang/lang-en.ini:19-25` defines notify-icon tooltip/status; main menu exposes mode toggles. Rust has no tray dependency or tray code; the Slint app exits with its main window. |
| CAP-09 | Capture popup translucency/colors, always-on-top, keep size, auto-roll-up | **Missing** | **Original:** `app_data/lang/lang-en.ini:111-116,165-178,318-335`; defaults in `def_config.ini:6-17` and `def_user.ini:7-10`. Rust quick card uses fixed Slint colors/layout and is embedded in the main window (`apps/lingoes-desktop/ui/app-window.slint:678-737`). |
| CAP-10 | Windows native capture behavior across browsers/PDF/native controls | **Unverifiable** | The package exposes plug-in installation and ships browser/PDF components, but static resource evidence cannot establish compatibility breadth. Original binaries were not executed, and the Linux host cannot validate native Windows text extraction. Rust has no equivalent native integration; see PLUG-01 through PLUG-05. |

## 4. Text translation and web services

| ID | Observable Lingoes 2.9.2 feature | Rust status | Evidence and gap |
|---|---|---|---|
| TR-01 | Dedicated multiline Text Translation window/workflow | **Partial** | **Original:** `app_data/lang/lang-en.ini:126,169,491,503-510`; package includes `app_data/dict/Text Translation_*.ldx`. **Rust:** translation is a main tab with multiline source/target editors and controls (`apps/lingoes-desktop/ui/app-window.slint:320-425`), wired by `DesktopApp::translate_with_engine` (`apps/lingoes-desktop/src/lib.rs:277-324,531-565`). It lacks original provider catalog/history/window integration and blocks the UI callback while running. |
| TR-02 | Source/target language selection and source auto-detection | **Partial** | **Original:** defaults store source/destination (`app_data/data/def_user.ini:18-21`) and configuration exposes primary/fallback target languages (`def_config.ini:46-49`; `lang-en.ini:341-349`). **Rust:** editable language-code fields default to `auto`/`vi` (`apps/lingoes-desktop/ui/app-window.slint:36-40,327-357`) and codes are sent to providers. There is no validated language catalog, fallback-target rule, or locale mapping. |
| TR-03 | Google online text translation | **Partial** | **Rust:** `GoogleTranslator` calls Google’s unofficial `translate_a/single` endpoint and parses segments (`crates/lingoes-translator/src/lib.rs:97-186`), with wiremock coverage (`crates/lingoes-translator/src/lib.rs:311-485`). This is a working translator but not evidence of parity with Lingoes’ original selectable engines, provider contracts, limits, or error behavior. Live network success was not part of this static audit. |
| TR-04 | Multiple selectable Lingoes translation engines | **Partial** | **Original:** engine IDs/settings appear in `def_config.ini:46-49`, `def_user.ini:18-21`, and `lang-en.ini:341-349`. **Rust:** supports Google and OpenAI-compatible chat APIs (`OpenAiTranslator`, `crates/lingoes-translator/src/lib.rs:188-309`; engine selection `apps/lingoes-desktop/src/lib.rs:277-306`). These are not the original engine catalog, and settings are not persisted. |
| TR-05 | OpenAI/local-LLM translation | **Partial** | This is a Rust extension. Base URL/model/API key fields and a Chat Completions client exist (`apps/lingoes-desktop/ui/app-window.slint:48-53,586-645`; `crates/lingoes-translator/src/lib.rs:188-309`). It is usable but stores configuration only in live UI properties and provides no secure credential storage, persistence, model discovery, retry, or streaming. |
| TR-06 | Instant translation tied to cursor/selection/clipboard modes | **Missing** | **Original:** `app_data/lang/lang-en.ini:341-378` and `def_config.ini:65-76`. Rust translation requires the translation-tab button; clipboard watcher performs dictionary lookup rather than calling translation. |
| TR-07 | Local language mapping and fallback target when source equals target | **Missing** | **Original:** `app_data/lang/lang-en.ini:343-349`; defaults `SupportLCMap` and `TargetLangCode2` in `def_config.ini:46-49,77`. No equivalent logic exists in Rust. |
| TR-08 | Connection/proxy configuration and authenticated proxy | **Missing** | **Original:** `app_data/lang/lang-en.ini:156-163,426-438`; defaults in `def_config.ini:19-24`. Rust creates default `reqwest::Client` instances (`crates/lingoes-translator/src/lib.rs:103-119,225-233`) and exposes no proxy UI/settings. |
| TR-09 | Embedded local API/dictionary server | **Missing** | **Original:** `app_data/lang/lang-en.ini:454-461`; `def_config.ini:1-4` defaults to loopback port 11111. Rust has CLI commands but no listening API server. |
| TR-10 | Online update service | **Missing** | **Original:** `app_data/lang/lang-en.ini:36,436-438,533-548`; package has `lingoes_update32.exe`, `app_data/update/update.xml`, `app_data/update/upnow.xml`, and `app_data/update/service.xmc`. Rust has no updater, update metadata, or update UI. |

## 5. Speech and pronunciation

| ID | Observable Lingoes 2.9.2 feature | Rust status | Evidence and gap |
|---|---|---|---|
| TTS-01 | Speak selected dictionary result or translated text | **Partial** | **Original:** `app_data/lang/lang-en.ini:119,131-132,166-169`. **Rust:** lookup, translation, and quick-card buttons call `DesktopApp::speak` (`apps/lingoes-desktop/ui/app-window.slint:290-298,405-413,726-730`; `apps/lingoes-desktop/src/lib.rs:326-339`). Backend availability and voice mapping vary by OS. |
| TTS-02 | System TTS on Windows/macOS/Linux | **Partial** | `SystemTtsEngine` tries PowerShell on Windows, `say` on macOS, and speech-dispatcher/eSpeak commands on Linux (`crates/lingoes-tts/src/lib.rs:77-169`). Tests verify command selection through mocks (`:177-294`), not audible output. Windows PowerShell receives only text (`build_args`, `:125-131`), so `lang` and an explicit SAPI voice are not honored. Native Windows runtime behavior is unverified on Linux. |
| TTS-03 | Voice, engine, volume, pitch, speed, and audio-format controls | **Missing** | **Original:** `app_data/lang/lang-en.ini:405-424`; defaults at `def_config.ini:51-63`. Rust UI passes a language code only and has no speech-settings page or backend setters. |
| TTS-04 | Real-time pronunciation during query, cursor capture, and selected-text translation | **Missing** | **Original:** `app_data/lang/lang-en.ini:418-421`; defaults in `def_config.ini:55-57`. Rust speech is manual button/CLI invocation. |
| TTS-05 | Natural Voice packages/index and download | **Missing** | **Original:** `app_data/lang/lang-en.ini:407-410,424`; package contains `app_data/speech/sample/*.mp3`, `sample.dic`, and `data.xml`. Rust has no voice-package/index format or downloader. |
| TTS-06 | Speak selected text and stop speaking via hotkeys | **Missing** | **Original:** `app_data/lang/lang-en.ini:448-449`; hotkey defaults in `def_config.ini:43-44`. `TtsEngine` exposes only `speak` (`crates/lingoes-tts/src/lib.rs:18-20`), with no stop/cancel method or selection capture. |

## 6. History, notebook/favorites, and persistence

| ID | Observable Lingoes 2.9.2 feature | Rust status | Evidence and gap |
|---|---|---|---|
| HIST-01 | Search history list | **Missing** | **Original:** `app_data/lang/lang-en.ini:47,299-302`; default capacity `SearchHistory=20` in `app_data/data/def_config.ini:26-31`. `DesktopApp` stores only current results and selection (`apps/lingoes-desktop/src/lib.rs:50-63`); no history table/list exists. |
| HIST-02 | Clear history | **Missing** | **Original:** `app_data/lang/lang-en.ini:299-302`. No history model or clear action exists. |
| HIST-03 | Local online-dictionary result cache and clear-cache action | **Missing** | **Original:** `app_data/lang/lang-en.ini:304-305`. Rust has no online dictionary subsystem or result-cache schema. |
| HIST-04 | Notebook/favorites (“Add the word to notebook”) | **Missing** | **Original:** `app_data/lang/lang-en.ini:519`; `app_data/dict/image/remberword.png` supplies the action asset. No favorite/notebook model, database table, or UI callback exists in Rust. |
| HIST-05 | Persist installed dictionaries and entries | **Partial** | Rust persists dictionary metadata and entries in SQLite (`SqliteDictionary::open`, `crates/lingoes-dictionary/src/lib.rs:115-127`; schema initialization `crates/lingoes-dictionary/src/lib.rs:458-565`) beside the executable (`portable_database_path`, `apps/lingoes-desktop/src/lib.rs:397-408`). It does not persist the broad user workflow/settings state Lingoes stores. |

## 7. Copy, save, print, and content actions

| ID | Observable Lingoes 2.9.2 feature | Rust status | Evidence and gap |
|---|---|---|---|
| IO-01 | Copy displayed interpretation/translation | **Complete** | **Original:** `app_data/lang/lang-en.ini:121,168,447`. **Rust:** `DesktopApp::copy_to_clipboard` uses `arboard` (`apps/lingoes-desktop/src/lib.rs:341-351`) and lookup/translation UI buttons invoke it (`apps/lingoes-desktop/ui/app-window.slint:294-298,409-413`). The original global copy hotkey remains missing under CAP-07. |
| IO-02 | Save search result as HTML | **Missing** | **Original:** `app_data/lang/lang-en.ini:123,130,566,576`. Rust has no save callback, file chooser, exporter, or HTML presentation model. |
| IO-03 | Print displayed result | **Missing** | **Original:** `app_data/lang/lang-en.ini:122`. No print API or callback exists in Rust. |
| IO-04 | Find and Select All in result content | **Missing** | **Original:** `app_data/lang/lang-en.ini:124-125`. No rich document selection/find commands exist in the Slint definition pane. |
| IO-05 | Copy/speech/text-translation actions from mini/capture window | **Partial** | **Original:** `app_data/lang/lang-en.ini:165-178`. Rust quick card provides speech and “Detail,” but no copy or direct text-translation action (`apps/lingoes-desktop/ui/app-window.slint:678-737`). |

## 8. Settings, appearance, startup, localization, and licensing

| ID | Observable Lingoes 2.9.2 feature | Rust status | Evidence and gap |
|---|---|---|---|
| SET-01 | Persistent configuration dialog with General, Appearance, Connection, Speech, Hotkeys, Translation, Licenses, Capture, Other | **Missing** | **Original:** `app_data/lang/lang-en.ini:275-289` plus UTF-16 defaults in `app_data/data/def_config.ini`. **Rust:** the settings tab exposes clipboard toggle and in-memory translation fields only (`apps/lingoes-desktop/ui/app-window.slint:538-670`); no config read/write layer exists. |
| SET-02 | UI localization and runtime language selection | **Missing** | **Original:** 44 `app_data/lang/lang-*.ini` files and `lang-en.ini:291-295`; each language declares code/name/direction. **Rust:** visible UI/status text is hard-coded Vietnamese (`apps/lingoes-desktop/ui/app-window.slint`; `apps/lingoes-desktop/src/lib.rs:95,118-135,199-213,266,315,330,348`). No catalog loading or locale selection exists. |
| SET-03 | Right-to-left UI support | **Missing** | **Original:** language metadata supports `Dir="ltr"` or `rtl` (`app_data/lang/lang-en.ini:1-6`) and Arabic/Hebrew language files ship. Rust has no direction/localization property or RTL layout branch. |
| SET-04 | Window/result/phonetic fonts and sizes | **Missing** | **Original:** `app_data/lang/lang-en.ini:318-335`; defaults in `def_config.ini:6-17`. Rust uses fixed Slint typography; no user controls or phonetic font role exist. |
| SET-05 | Window style and capture-window colors/translucency | **Missing** | **Original:** `app_data/lang/lang-en.ini:318-339`; defaults in `def_config.ini:6-17`. Rust has fixed styling in `apps/lingoes-desktop/ui/app-window.slint` and no theme/style persistence. |
| SET-06 | Restore main/mini window position, size, sidebar, and last configuration page | **Missing** | **Original:** `app_data/data/def_user.ini:1-13`. Rust does not load/save window geometry or active tab. |
| SET-07 | Run at Windows logon; minimize after startup; hide tip window | **Missing** | **Original:** `app_data/lang/lang-en.ini:307-309`; defaults in `def_config.ini:26-31`. No startup-registration, start-minimized, or tip preference implementation exists. |
| SET-08 | Desktop shortcut and file-association creation | **Missing** | **Original:** `app_data/lang/lang-en.ini:315-316`. No installer/integration code exists. |
| SET-09 | Product activation, registration, license import, commercial/evaluation status | **Missing** | **Original:** `app_data/lang/lang-en.ini:35,41,133-143,472-485,587-624`. Rust is MIT/Apache-2.0 workspace software (`Cargo.toml:12-16`) and intentionally has no corresponding commercial-license workflow. |
| SET-10 | Error/status reporting | **Partial** | Both products expose status/errors. Rust maintains `status`/`has_error` and a status bar (`apps/lingoes-desktop/src/lib.rs:57-58,165-183`; `apps/lingoes-desktop/ui/app-window.slint:740-753`). It lacks Lingoes’ structured dialogs, retry/auth/network/update flows, and some callbacks discard errors after setting only status. |

## 9. Appendices and bundled tools

| ID | Observable Lingoes 2.9.2 feature | Rust status | Evidence and gap |
|---|---|---|---|
| APPX-01 | Appendix browser/groups integrated into sidebar | **Missing** | **Original:** `app_data/lang/lang-en.ini:54-70,73-90`; `app_data/appe/def_appendix.xml` defines groups, display templates, and actions. Rust has no appendix model, loader, renderer, or tab. |
| APPX-02 | Install/delete/modify/reorder/download `.laf` appendices | **Missing** | **Original:** `app_data/lang/lang-en.ini:73-90`. No `.laf` parser or appendix manager exists. |
| APPX-03 | Appendix Builder: create HTML project and build package | **Missing** | **Original:** `app_data/lang/lang-en.ini:92-109`; package contains `app_data/appe/def_project.zip`. Rust has no builder. |
| APPX-04 | Currency converter | **Missing** | **Original:** `app_data/appe/def_appendix.xml` item `rate`; assets and localized HTML under `app_data/appe/rate/`. No equivalent Rust tool exists. |
| APPX-05 | Weights & measures converter | **Missing** | **Original:** `def_appendix.xml` item `measures`; `app_data/appe/measures/`. No equivalent exists. |
| APPX-06 | International dialing codes | **Missing** | **Original:** `def_appendix.xml` item `dialcode`; `app_data/appe/dialcode/`. No equivalent exists. |
| APPX-07 | Time-zone converter | **Missing** | **Original:** `def_appendix.xml` item `timezone`; `app_data/appe/timezone/`. No equivalent exists. |
| APPX-08 | IPA 2005 chart | **Missing** | **Original:** `def_appendix.xml` item `ipa`; `app_data/appe/ipa/` including `vowel.png`. No equivalent exists. |
| APPX-09 | Periodic table | **Missing** | **Original:** `def_appendix.xml` item `ptoe`; `app_data/appe/ptoe/`. No equivalent exists. |
| APPX-10 | Scientific calculator | **Missing** | **Original:** `def_appendix.xml` item `calc`; `app_data/appe/calc/`. No equivalent exists. |
| APPX-11 | Simplified/traditional Chinese converter | **Missing** | **Original:** `def_appendix.xml` item `convzh`; `app_data/appe/convzh/` and conversion DBs. No equivalent exists. |
| APPX-12 | Number-to-text, Base64, MD5/SHA tools | **Missing** | **Original:** `def_appendix.xml` items `ntoc`, `base64`, `md5`; source assets under `app_data/appe/pre_install/`. No equivalent exists. |
| APPX-13 | Dictionary abbreviations and irregular-verbs references | **Missing** | **Original:** `def_appendix.xml` items `dictabbr`, `verb`; corresponding preinstall directories. No equivalent exists. |
| APPX-14 | HTML/JavaScript appendix execution semantics and `app://` actions | **Unverifiable** | XML/HTML resources prove a web-content appendix framework (`def_appendix.xml`; `app_data/appe/*/index*.html`), but original host security, navigation, scripting, and packaging behavior cannot be established without executing or deeper binary analysis. Rust has no webview/appendix host. |

## 10. Portable packaging, installation, and deployment

| ID | Observable Lingoes 2.9.2 feature | Rust status | Evidence and gap |
|---|---|---|---|
| PKG-01 | Portable mode | **Partial** | **Original:** root `install.ini:1-3` sets `PortableMode=y`; about UI labels Portable Edition (`app_data/lang/lang-en.ini:472-477`). **Rust:** `portable_database_path` always selects `<executable>/data/lingoes.db` (`apps/lingoes-desktop/src/lib.rs:397-408`) with a unit test (`:1015-1023`). It provides portable data placement but no explicit mode selection, full setting migration, bundled assets, or permission fallback. |
| PKG-02 | Self-contained Windows distribution with resources, updater, capture/browser/PDF components | **Missing** | **Original:** root ships `Lingoes.exe`, `LGui32.dll`, `OpenText32.dll`, updater, Chrome host, Acrobat APIs, resources and data. Rust repository has Cargo build outputs but no release manifest, installer, portable archive recipe, resource bundle, or integration helper scripts. |
| PKG-03 | Linux and Windows cross-platform GUI design | **Partial** | **Rust:** Slint/winit and conditional TTS backends target multiple OSes (`README.md:7-12,52-68`; `crates/lingoes-tts/src/lib.rs:110-122`). The audit host is Linux; native Windows GUI, clipboard, PowerShell TTS, hotkey/tray, packaging, and capture behavior were not verified. |
| PKG-04 | Online updater execution and upgrade lifecycle | **Missing** | **Original:** `lingoes_update32.exe`, update XML/XMC resources, and localized stages (`app_data/lang/lang-en.ini:533-548`). Rust has no updater or signed-release lifecycle. |
| PKG-05 | Exact original binary/runtime behavior | **Unverifiable** | Original PE binaries were intentionally not executed. File identity and static resources are known, but timing, rendering, Windows shell interactions, online service availability, and undocumented behavior cannot be claimed from this audit. |

## 11. Plug-ins, browsers, and PDF

| ID | Observable Lingoes 2.9.2 feature | Rust status | Evidence and gap |
|---|---|---|---|
| PLUG-01 | Generic text-capture plug-in installation | **Missing** | **Original:** `app_data/lang/lang-en.ini:374`. Rust has no plug-in ABI, loader, installer, or discovery path. |
| PLUG-02 | Chrome extension/native-messaging integration | **Missing** | **Original:** root `lingoes_chrome_extension.json:1-9` declares native host `com.lingoes.translator`, executable `lingoes_chrome_extension32.exe`, stdio transport, and one allowed Chrome extension origin. Rust provides no native-messaging manifest/host or browser extension. |
| PLUG-03 | Firefox plug-in | **Unverifiable** | **Original:** install action is exposed at `app_data/lang/lang-en.ini:376`, but no clearly named Firefox payload appears in the extracted root. Static resources alone do not establish whether Lingoes downloaded or generated it at runtime. Rust has no Firefox integration. |
| PLUG-04 | 360 Chrome plug-in | **Unverifiable** | **Original:** install action at `app_data/lang/lang-en.ini:378`; no separately identifiable static payload establishes its mechanism. Rust has no integration. |
| PLUG-05 | Adobe Acrobat/PDF capture plug-in | **Missing** | **Original:** install action at `app_data/lang/lang-en.ini:375`; root ships PE plug-ins `lingoes_acrobat32.api` and `lingoes_acrobat32_v7.api`. Rust has no Acrobat API plug-in, PDF text extraction, or PDF viewer integration. |
| PLUG-06 | Browser/PDF text capture parity on Windows | **Missing** | Rust’s only capture backend is clipboard polling (`crates/lingoes-capture/src/lib.rs:63-97,100-192`), so it cannot be considered equivalent to the packaged native/browser/PDF integration even though copying selected browser/PDF text may incidentally trigger it. |

## 12. Test and evidence coverage

These rows qualify confidence rather than adding product parity credit.

| ID | Audit concern | Status | Evidence and implication |
|---|---|---|---|
| VER-01 | Core/dictionary unit coverage | **Partial** | Tests cover Unicode normalization, exact/prefix/FTS lookup, transaction atomicity, StarDict, fixture-backed MDX v2, persistence, migration, and enable filtering (`crates/lingoes-core/src/lib.rs:105-154`; `crates/lingoes-dictionary/src/lib.rs:668-1237`). They do not compare output against original `.ldx/.ld2` dictionaries or rich rendering. |
| VER-02 | Translation coverage | **Partial** | Mock/wiremock tests cover request shape, parsing, and failures (`crates/lingoes-translator/src/lib.rs:311-485`; desktop mock flow `apps/lingoes-desktop/src/lib.rs:1056-1073,1181-1204`). No original-engine corpus or live provider compatibility baseline exists. |
| VER-03 | TTS/capture coverage | **Partial** | Command-runner mocks test TTS fallback (`crates/lingoes-tts/src/lib.rs:172-295`); clipboard mocks test bounds/debounce/thread lifecycle (`crates/lingoes-capture/src/lib.rs:195-336`). These tests do not verify audible speech, global Windows selection/cursor capture, tray, or hotkeys. |
| VER-04 | GUI end-to-end and visual parity coverage | **Missing** | Desktop tests exercise application logic/CLI (`apps/lingoes-desktop/src/lib.rs:875-1220`), but no Slint interaction tests, screenshots, golden layouts, accessibility tests, or Windows runs are present. |

## Highest-priority parity gaps

1. **Native Windows capture foundation:** cursor-under-pointer and selected-text capture, activation/filter rules, global hotkeys, tray, and a real popup/mini window. Clipboard polling is useful but not equivalent.
2. **Lingoes dictionary compatibility:** LD2/LDP/LDX ingestion, metadata, resources, pronunciation/audio, HTML layout, dictionary groups/order, and accurate per-result attribution.
3. **Desktop workflow parity:** main menu, history/back/forward, sidebar/index/guide/appendix, web search, spelling suggestions, mini window, save/print/find/select-all.
4. **Persistence and configuration:** localized settings store, 44-language UI model including RTL, appearance, proxy, startup, speech controls, capture rules, window state, and safe provider-secret handling.
5. **History/notebook:** persisted query history, clear-history/cache, and favorites/notebook.
6. **Appendix framework:** `.laf` management/builder plus the 13 packaged converter/reference tools.
7. **Windows distribution integrations:** portable/release packaging, updater, Chrome native messaging, Acrobat PDF plug-in, and any Firefox/360 integration that can be specified from clean-room evidence.
8. **Verification:** Windows CI/runtime tests, GUI interaction/visual baselines, golden dictionary fixtures, and behavior-level comparison derived without executing or copying protected implementation internals.

## Bounded unknowns and non-equivalence warnings

- No original PE (`.exe`, `.dll`, `.api`) was executed. Menu labels and packaged files are strong evidence of exposed features, not proof of all runtime outcomes.
- The original online endpoints are old HTTP resources; current service availability is unknown and is not credited or debited as parity.
- Chrome’s native-host manifest is directly observable, while Firefox and 360 mechanisms are not identifiable from filenames alone; those rows remain **Unverifiable** for original mechanics.
- Native Windows behavior of the Rust build was not tested on this Linux host. Conditional source proves intent, not runtime parity.
- Clipboard polling can react after the user copies text. It does **not** capture a word under the cursor, capture selection without copying, or provide browser/PDF plug-in parity.
- MDict import support does not imply MDict presentation parity: current code flattens UTF-8 records and has no MDD/resource/HTML host.
- The README’s “3 main tabs” statement conflicts with the four tabs actually defined in `apps/lingoes-desktop/ui/app-window.slint`; this audit follows implementation.
