# Original Lingoes 2.9.2 Portable — Static Feature Inventory

## Scope, identity, and method

This inventory describes the extracted original package at:

- Archive: `/home/longpc/lingoes-original-audit/lingoes_portable_2.9.2.zip`
- Supplied archive MD5: `9A88E062A1C5B489C59BA36B59350B06`
- Extracted tree: `/home/longpc/lingoes-original-audit/extracted`
- Main executable: `Lingoes.exe`, 2,682,880 bytes, SHA-256 `6a1c4573739506868e71f819714e7a84b1b21fceb18560b7fd55381982721cfd`
- Static version resource: product/file version `2.9.2.0`, description **“Lingoes Dictionary and Text Translation Tool”**, copyright 2007–2014 Lingoes Project.
- Packaging mode: `install.ini:1-3` sets `PortableMode=y`.

The extracted package contains **369 files / 24,510,585 bytes**. This audit used only static reads: configuration and localization parsing, XML/HTML/JavaScript inspection, archive member listing, PE headers/imports/resources, exported symbols, and printable ASCII/UTF-16 strings. **No original executable, DLL, plug-in, script, or embedded application was executed.**

Evidence notation such as `app_data/lang/lang-en.ini:27-40` gives an exact package-relative file and line range. PE observations identify the artifact and evidence type because they do not have source lines.

### Confidence labels

- **Confirmed:** directly declared by configuration, localization, markup/script, PE import/export, or multiple independent static artifacts.
- **Indicated:** a UI label, image, string, or packaged component establishes intended support, but runtime behavior was not observed.
- **Not established:** absent from the bounded static corpus or hidden by an opaque format; this is not a claim about runtime nonexistence.

## 1. Application shell and GUI commands

### Main window and navigation — Confirmed

- Standard close, maximize, minimize, and restore actions, plus a main menu (`app_data/lang/lang-en.ini:11-17`).
- Main-menu commands: **Open**, three capture/translation toggles, **Configuration…**, **Dictionaries…**, **Appendices…**, product activation/license entry, online update, help, support, about, and exit (`lang-en.ini:27-41`).
- Search/input toolbar: back, forward, main menu, history drop-list, normal/detailed search, web search, auto-match against the index group, index-group options, and “get more web engines” (`lang-en.ini:43-52`).
- Sidebar tabs/pages: **Index**, **Guide**, and **Appendix**; the sidebar can be shown or hidden (`lang-en.ini:54-59`, `118-130`).
- Guide shortcuts expose results, spelling alternatives, appendices, options, dictionary/appendix management, configuration, help, homepage, and community (`lang-en.ini:61-71`).
- Result-view commands: speak selected text, search, copy, print, save as HTML, find, select all, launch text translation, go to start page, and show/hide sidebar (`lang-en.ini:118-132`).
- Standard edit strings for erase, erase all, copy, cut, find, paste, repeat, replace, select all, undo, and redo are present in `Lingoes.exe` RT_STRING resources (PE resource type 6, table ID 3603). Some may be framework-provided rather than exposed in the main UI.

### Search keyboard semantics — Confirmed

The mini/capture search prompt documents:

- `Enter`: search.
- `Ctrl+Enter`: detailed explanations.
- `Shift+Enter`: text translation.
- `Alt+Enter`: web search.

Evidence: `app_data/lang/lang-en.ini:165-178`.

### Main versus mini window — Confirmed

- Persistent defaults record separate main and mini-window rectangles, sidebar visibility/width, and current search list (`app_data/data/def_user.ini:1-13`).
- Mini window commands include back/forward, search, speak, copy, text translation, roll up/restore, keep always on top, keep size, delayed auto-roll-up, and close (`lang-en.ini:165-178`).
- Defaults include `MiniWndAutoRoolUp`, `MiniWndKeepSize`, and `MiniWndKeepVisual` (`def_user.ini:7-10`).
- Status-bar action opens/closes the mini window and can keep the main window always on top (`lang-en.ini:111-116`).
- `Lingoes.exe` contains the class/title strings `Lingoes_MiniWnd` and `Lingoes Mini Window`, corroborating a separate window implementation.

## 2. Lookup, results, spelling, history, cache, and notebook

### Dictionary lookup flow — Confirmed

- Search supports a normal operation and “detailed explanations,” with back/forward result navigation (`lang-en.ini:43-52`, `165-172`).
- The **Index Group** supplies closest-word/spelling suggestions while typing; localization warns that too many index dictionaries slow matching (`lang-en.ini:54-59`, `211-214`, `254-262`).
- Result surfaces include **Spelling Alternatives**, **Spell Checker**, text translation, web search, “More results,” waiting state, and no-match state (`lang-en.ini:488-495`).
- Individual dictionary result menus can show About, enable/disable a dictionary, move it to the top result, and arrange results; labels distinguish Index, Query, and Text Capture group membership (`lang-en.ini:521-531`).
- Result markup supports per-dictionary collapsing/minimizing and in-result find (`lang-en.ini:512-519`).
- Packaged assets such as `app_data/dict/image/entry_p.png`, `entry_ph.png`, `audio.png`, `expand.png`, `menu.png`, and `remberword.png` corroborate pronunciation, expansion, dictionary menus, and notebook affordances.

### History and online-result cache — Confirmed

- The search box has a **History List** (`lang-en.ini:43-48`).
- Configuration controls how many searched words are saved, allows clearing term history, and separately clears the local cache of online dictionary results (`lang-en.ini:299-305`).
- The default history limit is `SearchHistory=20` (`app_data/data/def_config.ini:26-31`).

### Favorites versus notebook — Important boundary

- **Confirmed:** a result action labeled **“Add the word to notebook”** (`RememberWord`) exists (`lang-en.ini:512-520`), with a corresponding `remberword.png` asset.
- **Not established:** no `favorite` or `favourite` token occurs in the inspected text/config/markup corpus. The static evidence supports a word notebook/save-word action, but does not establish a separately named Favorites manager, favorites list UI, or notebook import/export format.

## 3. Dictionary inventory and management

### Shipped dictionary components — Confirmed

The package includes eight `.ldx` artifacts:

- Core/service-looking entries:
  - `app_data/dict/def_EA8BE1CEC6BCAD41A4BAF7705F2AF5E6.ldx`
  - `app_data/dict/instant_93A93E37CB43D14DB05A15ECCCE91634.ldx`
  - `app_data/dict/Text Translation_54BDECB4CEEAB74D9F8BED8AF043163D.ldx`
- Pre-install candidates:
  - `Concise English Dictionary_A3F32E03C1866047BF2A3B9D5AAB4C47.ldx`
  - `Concise English Synonym and Antonym Dictionary_25D62D261B9B6943BE86B7DCF8F9D255.ldx`
  - `English-German Dictionary_44CAB16C0D062A40B40AB8F0F0DED32F.ldx`
  - `Jukuu English Sentence_EE0B95E38BB17847AE5B00465E612602.ldx`
  - `WordNet Online_7284B7DA4F80514D9FEDCCA913280A77.ldx`

Also shipped are `ucd.db`, `chs2t.db`, `cht2s.db`, `layout2.cfc`, and dictionary UI assets. The `.ldx` files were treated as opaque data and not reverse-engineered into source content.

### Dictionary manager — Confirmed

- Install from disk, download from Lingoes, uninstall, add/remove from a list, download for offline use, reorder to top/up/down/bottom, and inspect details (`lang-en.ini:180-203`).
- Separate manager views/lists for **Installed Dictionaries**, **Index Group**, and **Text Capture Group** (`lang-en.ini:195-203`). The localization uses “Installed Dictionaries” for the query list as well; dictionary result-menu labels explicitly identify a **Query Group** (`lang-en.ini:200-203`, `527-531`).
- Dictionary install formats are `.ld2` and `.ldp` (`lang-en.ini:211-214`).
- Install completion can optionally add the dictionary to Index Group and Text Capture Group (`lang-en.ini:254-274`).
- Install pipeline messages establish package load, extraction, setup-file parsing, format/version checks, dictionary install, and package completion (`lang-en.ini:265-274`).
- Duplicate install confirmation, uninstall, group removal, incompatible old/new dictionary version checks, and file-format errors are defined (`lang-en.ini:550-562`).
- Details view exposes ID, MD5, tags, name/title, version, creation/update dates, word count, languages, edition, description, author/publisher/contact/website/copyright, and filename (`lang-en.ini:230-252`).
- Online dictionaries can be marked as requiring an Internet connection, and licensed dictionaries/products can expose activate/purchase links (`lang-en.ini:204-209`).

### Dictionary backup/import/export — Confirmed

- Dictionary backup file extension: `.ldb`.
- Commands: **Export dictionaries to backup file** and **Import dictionaries from backup file**, each with success/failure states.

Evidence: `lang-en.ini:216-219`.

### Opaque default dictionary configuration — Limitation

`app_data/dict/def_dictcfg.xmc` is a 4,356-byte high-entropy binary beginning `3f 4f 44 44`, SHA-256 `ae66ef91b667d79ae27b27660e1eed35b3d40ebd86907a2f83177b87647a5e8c`. Printable-string scanning did not expose a trustworthy group/order model. Therefore, exact initial dictionary enablement and group ordering are not claimed here.

## 4. Text translation and web search

### Text translation — Confirmed

- Dedicated **Text Translation** screen with source/target language selection, Translate command, progress state, connection/request failure, and result-parse failure messages (`lang-en.ini:503-510`).
- Translation settings include local language mapping, instant translation, engine selection, target language, “current user language,” and a fallback target when source equals target (`lang-en.ini:341-350`).
- Defaults select instant-translation `Engine=1`, an unset first target, and German fallback (`TargetLangCode2=de`) (`app_data/data/def_config.ini:46-50`).
- The full text-translation defaults select `Engine=101`, source English, destination German (`app_data/data/def_user.ini:18-21`).
- Translation can be launched from the result view, mini window, or captured text (`lang-en.ini:126`, `165-172`).
- Marketing copy in the start-page localization states support for translation among more than 80 languages (`lang-en.ini:497-501`). This is an intended product claim, not runtime-verified service coverage.

### Web search engines — Confirmed

`app_data/webe/webengine.xml:1-145` defines URL-template substitution using `[$key]` and `[$langcode]`:

- **96 engine occurrences, 44 unique engine IDs** across four locale profiles: default (22), Simplified Chinese (26), Traditional Chinese (22), and German (26).
- General search providers include Google, Bing, Yahoo, and Baidu.
- Dictionary/language providers include Dictionary.com, The Free Dictionary, Urban Dictionary, OneLook, Merriam-Webster, Oxford Dictionaries, Cambridge, Longman, Babylon, dict.cc, WordReference, LEO, dict.cn, Dr.eye, and locale-specific services.
- Reference providers include Answers.com, Britannica, Wikipedia variants, and Baike.
- Default user state chooses `google.com.hk` (`app_data/data/def_user.ini:15-16`).
- Configuration lets the user select the search engine; the input toolbar also links to obtaining more engines (`lang-en.ini:49-52`, `296-297`).

All packaged templates use historical plain-HTTP URLs except where a provider string happens otherwise; service availability and modern TLS redirects were not tested.

## 5. Capture modes, selection filtering, plug-ins, and hotkeys

### Three capture modes — Confirmed

The application distinguishes:

1. **Capture Word on Screen** (cursor/mouse capture).
2. **Translate Selected Text**.
3. **Translate Clipboard Text**.

These appear independently in the tray tooltip/info, main menu, status controls, default enable flags, capture settings, and hotkeys (`lang-en.ini:19-31`, `111-116`, `351-379`, `440-452`; `def_user.ini:1-5`). Defaults enable cursor capture, disable selection and clipboard capture (`def_user.ini:3-5`).

### Mouse/cursor capture behavior — Confirmed

- Activation choices: mouse-over, left, middle, right, XButton1, or XButton2, with configurable delay (`lang-en.ini:351-362`).
- Controls: match phrase; ignore numbers, Latin, CJK, Cyrillic, or Arabic/Hebrew scripts; optionally show only the first result (`lang-en.ini:363-373`).
- Defaults: encoded `GrabActivation=0xFF000004`, 200 ms delay, phrase matching enabled, numbers ignored, other script filters disabled, and all results shown (`app_data/data/def_config.ini:65-77`). The encoded activation value’s exact UI choice was not inferred without execution.
- `OpenText32.dll` exports `gtActiveMouseHook`, `gtSetGrabMode`, `gtSetSelectionMode`, `gtSetClipboardMode`, and `gtSetSelectionFilterData`. Imports/strings include Windows hook and clipboard APIs, supporting a native capture helper implementation.

### Selected-text filtering — Confirmed

- Selection translation may react to selection and/or double-click (`lang-en.ini:369-372`).
- Per-rule fields: executable filename, window class, ignore selection, ignore double-click, and comment; operations include add, modify, delete, restore defaults, browse for an executable, and capture a window class from screen (`lang-en.ini:380-403`).
- `lwcnc32.exe` statically identifies itself as **Lingoes Window Class Name Capture**.
- `app_data/data/def_zwfilter.txt:1-57` supplies default exclusions for standard control classes, menus/dialogs/desktop, toolbars/common controls, console/media windows, Lingoes’ own buttons, and named applications such as Photoshop, Excel, WinRAR, AutoCAD, Flash, MathType, Remote Desktop, and Visual Studio 6 docking trees.

### Browser/PDF capture plug-ins — Confirmed as packaged/integrated; runtime compatibility untested

- UI offers installers for generic Text Capture, Adobe Acrobat PDF, Firefox, Chrome, and 360 Browser plug-ins (`lang-en.ini:372-378`).
- Acrobat plug-ins: `lingoes_acrobat32.api` and `lingoes_acrobat32_v7.api`, both exporting `GetWord` and `PlugInMain`; the newer artifact contains `ADBE:LingoesTextCapture`.
- Chrome bridge: `lingoes_chrome_extension32.exe` plus `lingoes_chrome_extension.json`. The manifest declares native-messaging host `com.lingoes.translator`, stdio transport, executable `lingoes_chrome_extension32.exe`, and allowed origin `chrome-extension://pkijkcfcbkdfjfhddklifpmofknndggc/`.
- The Chrome bridge contains JSON protocol strings for `text_capture` and `text_select`, with response/event names `on_text_capture` and `on_text_select`.
- Main/helper strings identify `WM_LINGOES_PLUGIN_GET_TEXT`, Chrome/Firefox shared mutex/global-data names, `Chrome_RenderWidgetHostHWND`, and Acrobat plug-in paths.

### Global hotkeys — Confirmed

`Lingoes.exe` imports `RegisterHotKey`/`UnregisterHotKey`. Defaults in `app_data/data/def_config.ini:33-44` encode Win32 modifiers in the high word and virtual key in the low word:

| Action | Stored value | Decoded default |
|---|---:|---|
| Translate clipboard toggle | `262210` / `0x00040042` | Shift+B |
| Copy interpretation | `262217` / `0x00040049` | Shift+I |
| Cursor capture toggle | `262215` / `0x00040047` | Shift+G |
| Open/close main window | `262220` / `0x0004004C` | Shift+L |
| Open/close mini window | `393292` / `0x0006004C` | Ctrl+Shift+L |
| Pronounce | `262224` / `0x00040050` | Shift+P |
| Roll up/restore mini window | `393291` / `0x0006004B` | Ctrl+Shift+K |
| Selected-text toggle | `262234` / `0x0004005A` | Shift+Z |
| Speak selected text | `131195` / `0x0002007B` | Ctrl+F12 |
| Stop speaking | `393339` / `0x0006007B` | Ctrl+Shift+F12 |

The configuration UI can disable hotkeys globally (`lang-en.ini:440-452`; `def_config.ini:33-44`).

## 6. Speech and pronunciation

### Speech engines and controls — Confirmed

- Two engine concepts are exposed: **Natural Voice Engine** and system **Text-to-Speech (TTS) Engine** (`lang-en.ini:405-424`).
- Controls include enable, voice selection, volume, pitch, speed/rate, output format, voice test text, and Natural Voice index rebuild (`lang-en.ini:405-424`).
- Automatic real-time pronunciation can occur while looking up a word, capturing on-screen text, or translating selected text (`lang-en.ini:418-421`).
- Defaults enable speech globally at volume 75, pitch/rate 0, select `Microsoft Lili`, disable all three automatic pronunciation triggers, and configure (but disable) Natural Voice `sample` (`def_config.ini:51-63`).
- `Lingoes.exe` contains the registry path `HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Speech\Voices`, indicating Windows SAPI voice discovery.
- Packaged sample Natural Voice data includes `hello.mp3`, `good.mp3`, `dictionary.mp3`, `test.mp3`, and `voice.ini`; `voice.ini:1-3` permits a redirected voice-file folder.
- Failure UI directs users to install/download a system TTS engine (`lang-en.ini:578-579`).

## 7. Tray and startup behavior

### Notification-area integration — Confirmed

- Tray tooltip/balloon shows current enabled/disabled status for cursor capture, selected-text translation, and clipboard translation (`lang-en.ini:19-25`).
- The tray/main menu exposes Open, all three toggles, Configuration, Dictionaries, Appendices, activation/update/help/about/support, and Exit (`lang-en.ini:27-41`).
- `Lingoes.exe` imports `Shell_NotifyIconW`, corroborating Windows notification-area integration.

### Startup and shell integration — Confirmed/indicated

- Settings expose run at Windows logon, start minimized, suppress tip window, set associations for `.ld2`, `.ldp`, and `.laf`, and create a desktop shortcut (`lang-en.ini:307-317`).
- Defaults set `StartMinmize=n` and `HideTipWnd=n` (`def_config.ini:26-31`).
- Main strings include `-minimize`, indicating command-line minimized startup.
- Tip-of-the-day window is localized (`lang-en.ini:625-627`).

## 8. Settings inventory

The Configuration dialog has pages for **General, Appearance, Connection, Speech, Hotkeys, Translation, Licenses, Text Capture, and Other** (`lang-en.ini:276-289`).

### General

- UI language, selected web-search engine, history length/clear, online cache clear, Windows logon startup, minimized startup, tip suppression, import from Lingoes 2.8 or older, file associations, and desktop shortcut (`lang-en.ini:291-317`).

### Appearance

- Window style; UI font and size scheme; result font and size; phonetic font; capture-window text/background/border colors and translucency (`lang-en.ini:318-340`).
- Defaults use Tahoma, result 10.5 pt, UI 9 pt, `Lingoes Unicode` phonetic font, style 1, and fully opaque capture window (`def_config.ini:6-17`).

### Connection and updates

- Direct connection, browser/IE proxy settings, or explicit proxy/firewall host/port/credentials; authentication dialog can remember a password (`lang-en.ini:156-163`, `426-439`).
- Defaults enable updates and leave explicit proxy server/port empty (`def_config.ini:19-24`). The stored `ProxyAuth` blob is not interpreted.
- Online update UI has check/download/apply stages for both update and upgrade, success/failure/latest states, and exit-to-complete behavior (`lang-en.ini:533-548`).
- A dedicated packaged `lingoes_update32.exe` has a static description **Lingoes Update**. No updater was run.

### Experimental local API server

- Settings expose listen IP, port, enable toggle, and developer guide, explicitly labeling the feature experimental (`lang-en.ini:454-461`).
- Defaults: disabled, `127.0.0.1:11111` (`def_config.ini:1-4`).
- `Lingoes.exe` strings include `LingoesAPI`, `translate`, `search`, `main_search`, `mini_search`, an HTTP server response template, and Winsock event diagnostics. Imports include `WS2_32.dll`.
- Exact request/response schema, authentication, and safety properties were not established statically.

### Licensing

- License manager can enter/import `.llc` licenses, remove them, and view details; activation collects license key, name, and email and supports proxy/network settings (`lang-en.ini:587-623`).
- Dictionary and result UI can gate content or features behind activation/purchase (`lang-en.ini:207-209`, `512-518`).

## 9. Appendices and embedded tools

### Appendix platform and management — Confirmed

- Appendix manager can install `.laf`, delete, modify, create groups, reorder, download more, and overwrite an existing appendix (`lang-en.ini:73-90`).
- Appendix Builder creates a named project folder from a template and builds a `.laf`; users edit `index.html` (`lang-en.ini:100-109`).
- `app_data/appe/def_project.zip` statically contains `index.html`, three logo sizes, and `setup.xml`, confirming a distributable project template.
- `app_data/appe/def_appendix.xml:2-50` defines localized grouping and seven display styles: text, stressed text, tiled text, icon+text, and small/middle/large icon. Items navigate through appendix actions.
- Appendix files are HTML/JavaScript designed for the embedded web view and reference locale/font/path variables. Their use of old IE-era markup is visible in `guide.htm`/`start.htm` (`X-UA-Compatible=IE=6`).

### Declared appendix catalog — Confirmed

`app_data/appe/def_appendix.xml:29-50` declares 13 items:

- **Common:** Currency Converter, Weights & Measures Converter, International Dialing Codes, Time Zone Converter.
- **Recommended:** IPA 2005.
- **Tools:** Periodic Table, Calculator, Chinese Simplified/Traditional conversion, currency number-to-text, Base64 encode/decode, MD5 Algorithm.
- **Information:** dictionary abbreviations and irregular verbs.

Eight appendix directories are present directly under `app_data/appe` (`calc`, `convzh`, `dialcode`, `ipa`, `measures`, `ptoe`, `rate`, `timezone`), while five declared items are under `appe/pre_install` (`base64`, `dictabbr`, `md5`, `ntoc`, `verb`). Each has `setup.xml`; thus “pre_install” indicates packaged install candidates rather than proof they are active by default.

### Tool-specific static behavior — Confirmed

- **Calculator:** scientific functions, brackets, log/ln/trigonometry, powers/factorial/reciprocal, memory operations, base/carry and angle modes; controls are in `app_data/appe/calc/index.html` and localized variants.
- **Chinese conversion:** convert in both Simplified→Traditional and Traditional→Simplified directions, plus copy/cut/paste/clear (`appe/convzh/index.html`).
- **Measures:** length, weight, area, volume, power, heat, pressure, and temperature conversions (`appe/measures/index.html`).
- **Currency:** amount/from/to selection, swap, timestamp display, and a network request to `http://www.lingoes.net/api/rate/currency.php` (`appe/rate/index.html:33-76`).
- **Time zones:** conversion among input zones, “Now,” reset, and all-zone calculations (`appe/timezone/index.html`).
- **Periodic table:** interactive element fields including name, atomic number/weight, shells, orbital, melting point, and boiling point (`appe/ptoe/index.html`).
- **Base64:** UTF-8-aware encode/decode JavaScript (`appe/pre_install/base64/index.html`).
- **Hashes:** MD4, MD5, SHA-1, and HMAC helper functions, despite the catalog’s shorter “MD5 Algorithm” label (`appe/pre_install/md5/index.html`, `md4.js`, `md5.js`, `sha1.js`).
- **Number-to-text:** numeric currency conversion, with Chinese and English conversion functions (`appe/pre_install/ntoc/index.html`).
- **Static references:** dialing codes, IPA chart, abbreviations, and irregular verbs are packaged as HTML content.

The tool scripts were read, not executed; calculation correctness and remote service availability remain unverified.

## 10. Import/export and file formats

Confirmed user-facing file operations:

- Install dictionaries from `.ld2` / `.ldp` (`lang-en.ini:211-214`).
- Export/import dictionary backups as `.ldb` (`lang-en.ini:216-219`).
- Install appendices as `.laf` (`lang-en.ini:73-90`).
- Build appendix projects into appendix files (`lang-en.ini:100-109`).
- Import data/dictionaries from a Lingoes 2.8-or-older installation folder (`lang-en.ini:311-313`).
- Save displayed search results as `.htm` / `.html` and print them (`lang-en.ini:118-131`).
- Import licenses as `.llc` (`lang-en.ini:587-591`).
- Set shell associations for `.ld2`, `.ldp`, and `.laf` (`lang-en.ini:315-316`).

No general-purpose export of history, notebook words, settings, translation results to plain text, or favorites was established in the inspected static corpus.

## 11. Localization and bidirectional UI

### Packaged UI languages — Confirmed

- **44 `lang-*.ini` files** are present, covering Arabic, Belarusian, Bulgarian, Catalan, Czech, German, Greek, English, Spanish, Persian file, Finnish, French variants, Hebrew variants, Hungarian, Indonesian, Italian, Japanese, Kazakh, Khmer, Korean, Latvian, Malay, Nepali/Devanagari, Dutch, Polish, Portuguese variants, Russian, Serbian, Thai, Turkish, several Uyghur script/locale variants, Ukrainian, Uzbek Latin/Cyrillic, Vietnamese, and Simplified/Traditional Chinese.
- There are also **10 `ilc-*.ini` files**, apparently locale-code supplemental resources for English, Japanese, Portuguese variants, Chinese variants, and Uyghur variants. Their runtime role was not established.
- Each normal language file can declare language code/name, English name, direction (`ltr`/`rtl`), Windows LangID, version, and translator; see `lang-en.ini:1-9`.
- RTL is explicitly declared in Arabic, Hebrew, and Arabic-script Uyghur locale files. RTL-specific image assets include search, drop-list, index/guide/appendix tabs, and sidebar switch variants.
- Appendix and web-engine metadata use localized title attributes and locale-specific HTML variants (`def_appendix.xml:7-50`; appendix directory listings).

One file, `lang-fa.ini`, could not be decoded cleanly by the automated UTF-16 metadata pass due to malformed surrogate data; its presence is counted, but its header metadata is not asserted.

## 12. Web/HTML host and plug-in architecture

### Embedded web content — Confirmed

- Results, guide/start pages, update page, dictionaries, and appendices use packaged HTML, CSS, images, Flash-era audio assets, and `app://...` navigation actions.
- Examples of internal actions include `app://homepage/`, `help/`, `downdict/`, `dictionary_group/...`, `texttranslation/`, `websearch/`, `activate/`, `buy/...`, `appendix/...`, and `startpage/` (`lang-en.ini:54-70`, `488-519`; `app_data/update/update.xml`).
- `Lingoes.exe` imports OLE libraries (`ole32.dll`, `OLEAUT32.dll`) and contains web-view class strings (`CWebGuide`, `CWebTip`, `CWebViewQ`), consistent with an embedded Windows web/OLE surface. Static evidence does not uniquely identify the exact browser control version.
- Packaged `audio.swf` / `ap.swf` files and corresponding PNG/cursor assets indicate Flash-era audio affordances, but runtime fallback behavior was not observed.

### Native module layout — Confirmed

- `Lingoes.exe`: 32-bit Windows GUI PE; imports `LGui32.dll`, MFC42u, MSVCRT, Kernel/User/GDI, common dialogs, registry/shell/OLE, multimedia, and Winsock libraries.
- `LGui32.dll`: custom GUI controls including image/link/tool buttons, static controls, and splitter bars (named exports).
- `OpenText32.dll`: capture/selection/clipboard helper exports listed above.
- `lingoes_chrome_extension32.exe`: Chrome native-messaging bridge.
- `lingoes_acrobat32.api` and `_v7.api`: Acrobat text-capture plug-ins.
- `lwcnc32.exe`: window-class capture utility.
- `lingoes_update32.exe`: updater.

### PE resource inventory — Confirmed

Static parsing of `Lingoes.exe` resources (REA Evidence `ev_1c8dc997e20cac2221d0d54fe439a24a5d6b2c9201dadbcf5efc7dea1f179c6c`) completed over **257 directory entries / 124 payloads**:

- 61 icon images (RT_ICON/type 3)
- 31 cursors (type 5)
- 15 group cursors (type 14)
- 9 string-table blocks (type 6)
- 4 type-240 custom payloads
- 1 bitmap, 1 accelerator table, 1 version resource, and 1 manifest

The manifest requests `asInvoker` and targets 32-bit Windows common controls. Resource payloads other than icon relationships were opaque to the REA resource parser; no runtime loading was observed.

## 13. Defaults snapshot

The package’s declared defaults provide a useful reconstruction baseline:

- Portable mode on (`install.ini:1-3`).
- Cursor capture on; selection and clipboard translation off (`def_user.ini:1-5`).
- Main sidebar visible at width 155 (`def_user.ini:6`, `12`).
- History capacity 20; tips shown; no minimized startup (`def_config.ini:26-31`).
- Updates enabled; direct/default connection mode (`def_config.ini:19-24`).
- UI Tahoma 9 pt; result Tahoma 10.5 pt; style 1 (`def_config.ini:6-17`).
- Speech enabled, volume 75; automatic pronunciation triggers off (`def_config.ini:51-63`).
- Capture delay 200 ms; phrase matching on; ignore numbers on; other script ignores off (`def_config.ini:65-77`).
- API server disabled on loopback port 11111 (`def_config.ini:1-4`).
- Default web search `google.com.hk`; text translation English→German using engine 101 (`def_user.ini:15-21`).

These are template defaults, not observed first-run state.

## 14. Static-analysis limits and reconstruction cautions

- No behavior in this report was dynamically validated. Window layouts, command availability by license/OS, and error paths may differ at runtime.
- `app_data/system.inc`, `app_data/dict/def_dictcfg.xmc`, and `app_data/update/service.xmc` are opaque `?ODD`-header binary blobs. `system.inc` is 4,428 bytes, SHA-256 `79d1f33d59622075c9e1bcf85dba3fef97bac77d5bf794ed2a0d4d2ed4c0fbbc`; no reliable text fields were recoverable. Claims concealed solely in these blobs are intentionally omitted.
- The `.ldx`, `.db`, `.cfc`, `.swf`, media, font, and image contents were inventoried but not converted into implementation source or copied into the reconstruction.
- Online provider lists and endpoints reflect the 2014 package and are not evidence of current availability.
- “Favorites” should not be implemented as an asserted original named feature based on this audit; the defensible original behavior is **history plus “Add the word to notebook.”**
- The audit distinguishes package presence from default activation: files under `pre_install` and installer UI prove availability, not that every dictionary/appendix/plugin starts enabled.
