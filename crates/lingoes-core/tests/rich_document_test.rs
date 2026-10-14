use lingoes_core::document::{
    DefinitionSection, EntryBlock, LinkReference, PartOfSpeech, Phonetic, RichDocument,
    find_in_result, sanitize_html, save_as_html,
};
use lingoes_core::metadata::DictionaryMetadata;

#[test]
fn rich_document_constructors_and_blocks() {
    let mut doc = RichDocument::new("apple");
    assert_eq!(doc.headword, "apple");
    assert!(doc.phonetics.is_empty());
    assert!(doc.blocks.is_empty());

    let phonetic = Phonetic::new("/ˈæp.əl/");
    assert_eq!(phonetic.text, "/ˈæp.əl/");
    doc.phonetics.push(phonetic.clone());

    let pos = PartOfSpeech::new("noun");
    assert_eq!(pos.tag, "noun");

    let section = DefinitionSection {
        pos: Some(pos.clone()),
        definitions: vec!["the round fruit of a tree of the rose family".to_string()],
        examples: vec!["She ate a crisp red apple.".to_string()],
        synonyms: vec!["pome".to_string()],
    };

    let link = LinkReference::new("pear", "See also pear", false);
    assert_eq!(link.target, "pear");
    assert_eq!(link.label, "See also pear");
    assert!(!link.is_external);

    doc.blocks.push(EntryBlock::Phonetic(phonetic));
    doc.blocks.push(EntryBlock::PartOfSpeech(pos));
    doc.blocks.push(EntryBlock::Section(section));
    doc.blocks.push(EntryBlock::Link(link));

    assert_eq!(doc.blocks.len(), 4);
}

#[test]
fn parser_converts_lingoes_xml_to_rich_document() {
    let raw = r#"<C><F><H><M>[hə'ləʊ]</M></H><I><N><P><U>int.</U></P><Q>used as a greeting or to begin a phone conversation.</Q><Z>hello there, Katie!</Z></N></I></C>"#;
    let doc = RichDocument::from_html("hello", raw, Some("Oxford English"));

    assert_eq!(doc.headword, "hello");
    assert_eq!(doc.attribution.as_deref(), Some("Oxford English"));
    assert!(!doc.phonetics.is_empty());
    assert_eq!(doc.phonetics[0].text, "[hə'ləʊ]");

    assert!(!doc.pos_tags.is_empty());
    assert_eq!(doc.pos_tags[0].tag, "int.");

    // Check definitions and examples
    let has_def = doc.blocks.iter().any(|b| match b {
        EntryBlock::Section(sec) => sec
            .definitions
            .iter()
            .any(|d| d.contains("used as a greeting")),
        _ => false,
    });
    assert!(has_def, "expected definition section extracted from XML");
}

#[test]
fn parser_converts_html_formatting_and_tables() {
    let raw = r##"<div><b>bold word</b> <i>italic note</i> <font color="#ff0000">red note</font>
    <table>
        <tr><th>Col 1</th><th>Col 2</th></tr>
        <tr><td>Val 1</td><td>Val 2</td></tr>
    </table>
    <a href="https://example.com">external link</a>
    <a href="dict://jump/target">cross ref</a>
    </div>"##;

    let doc = RichDocument::from_html("test", raw, None);
    assert_eq!(doc.headword, "test");

    // Table extracted
    let has_table = doc
        .blocks
        .iter()
        .any(|b| matches!(b, EntryBlock::Table { .. }));
    assert!(has_table, "expected table block extracted");

    // Links extracted
    let has_ext_link = doc.blocks.iter().any(|b| match b {
        EntryBlock::Link(l) => l.is_external && l.target == "https://example.com",
        _ => false,
    });
    assert!(has_ext_link, "expected external link");

    let has_cross_ref = doc.blocks.iter().any(|b| match b {
        EntryBlock::Link(l) => !l.is_external && l.target.contains("target"),
        _ => false,
    });
    assert!(has_cross_ref, "expected cross-reference link");
}

#[test]
fn sanitizer_strips_scripts_and_malicious_tags() {
    let dangerous = r#"
        <h1>Greeting</h1>
        <script>alert('xss1');</script>
        <SCRIPT SRC="http://evil.com/payload.js"></SCRIPT>
        <p>Text</p>
        <object data="evil.swf"></object>
        <iframe src="http://evil.com"></iframe>
        <embed src="evil.swf">
        <applet code="Malicious.class"></applet>
    "#;

    let sanitized = sanitize_html(dangerous);
    assert!(!sanitized.to_lowercase().contains("<script"));
    assert!(!sanitized.to_lowercase().contains("alert('xss1')"));
    assert!(!sanitized.to_lowercase().contains("evil.com"));
    assert!(!sanitized.to_lowercase().contains("<object"));
    assert!(!sanitized.to_lowercase().contains("<iframe"));
    assert!(!sanitized.to_lowercase().contains("<embed"));
    assert!(!sanitized.to_lowercase().contains("<applet"));
    assert!(sanitized.contains("<h1>Greeting</h1>"));
    assert!(sanitized.contains("<p>Text</p>"));
}

#[test]
fn sanitizer_strips_inline_event_handlers() {
    let dangerous = r#"
        <img src="valid.png" onload="alert('onload')" onerror="alert('onerror')" alt="test">
        <div onclick="evil()" onmouseover="evil()" onfocus="evil()" style="color: blue;">Content</div>
        <a href="normal.html" ONCLICK="stealCookies()">Link</a>
    "#;

    let sanitized = sanitize_html(dangerous);
    assert!(!sanitized.to_lowercase().contains("onload"));
    assert!(!sanitized.to_lowercase().contains("onerror"));
    assert!(!sanitized.to_lowercase().contains("onclick"));
    assert!(!sanitized.to_lowercase().contains("onmouseover"));
    assert!(!sanitized.to_lowercase().contains("onfocus"));
    assert!(!sanitized.to_lowercase().contains("stealcookies"));
    assert!(sanitized.contains("alt=\"test\""));
    assert!(sanitized.contains("Content"));
    assert!(sanitized.contains("Link"));
}

#[test]
fn sanitizer_strips_javascript_and_dangerous_uris() {
    let dangerous = r#"
        <a href="javascript:alert(1)">bad link 1</a>
        <a href="   JaVaScRiPt:alert(2)">bad link 2</a>
        <a href="javascript&#58;alert(3)">bad link 3</a>
        <a href="vbscript:msgbox('hi')">bad link 4</a>
        <a href="data:text/html;base64,PHNjcmlwdD5hbGVydCgxKTwvc2NyaXB0Pg==">bad data</a>
        <a href="https://lingoes.net/safe">safe link</a>
    "#;

    let sanitized = sanitize_html(dangerous);
    assert!(!sanitized.to_lowercase().contains("javascript:"));
    assert!(!sanitized.to_lowercase().contains("alert(1)"));
    assert!(!sanitized.to_lowercase().contains("alert(2)"));
    assert!(!sanitized.to_lowercase().contains("vbscript:"));
    assert!(!sanitized.to_lowercase().contains("data:text/html"));
    assert!(sanitized.contains("https://lingoes.net/safe"));
}

#[test]
fn save_as_html_generates_valid_standalone_document() {
    let raw = r#"<C><F><H><M>[test]</M></H><I><N><P><U>noun</U></P><Q>a procedure intended to establish the quality, performance, or reliability of something.</Q></N></I></C>"#;
    let doc = RichDocument::from_html("test", raw, Some("Concise Oxford"));

    let html = save_as_html(&doc);

    // Metadata & structure
    assert!(html.starts_with("<!DOCTYPE html>"), "must have doctype");
    assert!(html.contains("<html"), "must have html tag");
    assert!(
        html.contains("<meta charset=\"utf-8\""),
        "must have charset"
    );
    assert!(
        html.contains("<meta name=\"viewport\""),
        "must have viewport"
    );
    assert!(
        html.contains("<meta name=\"generator\""),
        "must have generator meta tag"
    );
    assert!(html.contains("<title>test - Concise Oxford</title>"));

    // Styling
    assert!(html.contains("<style>"), "must contain CSS style block");
    assert!(
        html.contains(".dict-attribution"),
        "must style attribution header"
    );
    assert!(html.contains(".phonetic-badge"), "must style phonetic");
    assert!(html.contains(".pos-badge"), "must style part of speech");

    // Attribution header
    assert!(html.contains("Concise Oxford"));
    assert!(html.contains("test"));
    assert!(html.contains("[test]"));
    assert!(html.contains("noun"));
    assert!(html.contains("a procedure intended to establish"));

    // Document method matches function
    assert_eq!(html, doc.save_as_html());
}

#[test]
fn find_in_result_highlights_and_returns_selection_bounds() {
    let content =
        "The quick brown fox jumps over the lazy dog. A quick response is better than a slow Fox.";

    // Case-insensitive find
    let res = find_in_result("fox", content);
    assert_eq!(res.query, "fox");
    assert_eq!(res.total_matches, 2);
    assert_eq!(res.matches.len(), 2);

    // First match
    assert_eq!(res.matches[0].index, 0);
    assert_eq!(res.matches[0].matched_text, "fox");
    let range1 = &res.matches[0].range;
    assert_eq!(&content[range1.start..range1.end], "fox");

    // Second match preserves original casing
    assert_eq!(res.matches[1].index, 1);
    assert_eq!(res.matches[1].matched_text, "Fox");
    let range2 = &res.matches[1].range;
    assert_eq!(&content[range2.start..range2.end], "Fox");

    // Highlighted HTML contains marks
    assert!(
        res.highlighted_html
            .contains("<mark class=\"find-highlight\"")
    );
    assert!(res.highlighted_html.contains("fox</mark>"));
    assert!(res.highlighted_html.contains("Fox</mark>"));

    // Empty query returns no matches and unchanged content
    let empty_res = find_in_result("", content);
    assert_eq!(empty_res.total_matches, 0);
    assert!(empty_res.matches.is_empty());
    assert_eq!(empty_res.highlighted_html, content);

    // Special regex chars do not break find
    let code_content = "Value of C++ is higher than C# (or [abc] tags).";
    let cpp_res = find_in_result("C++", code_content);
    assert_eq!(cpp_res.total_matches, 1);
    assert_eq!(cpp_res.matches[0].matched_text, "C++");
}

#[test]
fn dictionary_metadata_roundtrip_and_defaults() {
    let meta = DictionaryMetadata {
        id: Some(1),
        title: "Oxford Advanced Learner's".to_string(),
        author: Some("A. S. Hornby".to_string()),
        publisher: Some("Oxford University Press".to_string()),
        website: Some("https://oxfordlearnersdictionaries.com".to_string()),
        copyright: Some("2026 OUP".to_string()),
        description: Some("The world's bestselling learner dictionary".to_string()),
        source_lang: Some("en".to_string()),
        target_lang: Some("en".to_string()),
        format: Some("stardict".to_string()),
        entry_count: Some(180000),
    };

    assert_eq!(meta.title, "Oxford Advanced Learner's");
    assert_eq!(meta.author.as_deref(), Some("A. S. Hornby"));
    assert_eq!(meta.source_lang.as_deref(), Some("en"));
}

#[test]
fn test_parse_formatted_lines_classifies_semantic_elements() {
    use lingoes_core::parse_formatted_lines;

    let raw = r#"<div><b>hello</b> — /həˈləʊ/<br/><div><b>■ thán từ</b></div><div><b>1.&nbsp;</b>xin chào ↔ hello</div><div>‣&nbsp;Say hello to your aunt</div><div><b>■ đồng nghĩa/liên quan</b></div><div>•&nbsp;hallo — hullo</div></div>"#;

    let lines = parse_formatted_lines(raw);
    assert!(
        lines
            .iter()
            .any(|l| l.kind == "headword" && l.body == "hello")
    );
    assert!(
        lines
            .iter()
            .any(|l| l.kind == "phonetic" && l.body.contains("həˈləʊ"))
    );
    assert!(
        lines
            .iter()
            .any(|l| l.kind == "pos" && l.body.contains("thán từ"))
    );
    assert!(
        lines
            .iter()
            .any(|l| l.kind == "sense" && l.prefix == "1." && l.body.contains("xin chào"))
    );
    assert!(
        lines
            .iter()
            .any(|l| l.kind == "example" && l.prefix == "▸" && l.body.contains("Say hello"))
    );
    assert!(
        lines
            .iter()
            .any(|l| l.kind == "section" && l.body.contains("đồng nghĩa"))
    );
    assert!(
        lines
            .iter()
            .any(|l| l.kind == "bullet" && l.prefix == "•" && l.body.contains("hallo"))
    );
}
