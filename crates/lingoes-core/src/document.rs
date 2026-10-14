use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Phonetic {
    pub text: String,
    pub lang: Option<String>,
    pub audio_url: Option<String>,
}

impl Phonetic {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            lang: None,
            audio_url: None,
        }
    }

    pub fn with_audio(text: impl Into<String>, audio_url: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            lang: None,
            audio_url: Some(audio_url.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PartOfSpeech {
    pub tag: String,
    pub description: Option<String>,
}

impl PartOfSpeech {
    pub fn new(tag: impl Into<String>) -> Self {
        Self {
            tag: tag.into(),
            description: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LinkReference {
    pub target: String,
    pub label: String,
    pub is_external: bool,
}

impl LinkReference {
    pub fn new(target: impl Into<String>, label: impl Into<String>, is_external: bool) -> Self {
        Self {
            target: target.into(),
            label: label.into(),
            is_external,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DefinitionSection {
    pub pos: Option<PartOfSpeech>,
    pub definitions: Vec<String>,
    pub examples: Vec<String>,
    pub synonyms: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryBlock {
    Header(String),
    Phonetic(Phonetic),
    PartOfSpeech(PartOfSpeech),
    Section(DefinitionSection),
    Paragraph(String),
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    Link(LinkReference),
    RawHtml(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RichDocument {
    pub headword: String,
    pub phonetics: Vec<Phonetic>,
    pub pos_tags: Vec<PartOfSpeech>,
    pub blocks: Vec<EntryBlock>,
    pub attribution: Option<String>,
}

impl RichDocument {
    pub fn new(headword: impl Into<String>) -> Self {
        Self {
            headword: headword.into(),
            phonetics: Vec::new(),
            pos_tags: Vec::new(),
            blocks: Vec::new(),
            attribution: None,
        }
    }

    pub fn from_html(headword: &str, raw: &str, attribution: Option<&str>) -> Self {
        parse_rich_document(headword, raw, attribution)
    }

    pub fn to_sanitized_html(&self) -> String {
        let mut html = String::new();
        for block in &self.blocks {
            match block {
                EntryBlock::Header(h) => {
                    html.push_str("<h3>");
                    html.push_str(&escape_html(h));
                    html.push_str("</h3>\n");
                }
                EntryBlock::Phonetic(p) => {
                    html.push_str("<span class=\"phonetic-badge\">");
                    html.push_str(&escape_html(&p.text));
                    html.push_str("</span>\n");
                }
                EntryBlock::PartOfSpeech(pos) => {
                    html.push_str("<span class=\"pos-badge\">");
                    html.push_str(&escape_html(&pos.tag));
                    html.push_str("</span>\n");
                }
                EntryBlock::Section(sec) => {
                    html.push_str("<div class=\"definition-section\">\n");
                    if let Some(pos) = &sec.pos {
                        html.push_str("<span class=\"pos-badge\">");
                        html.push_str(&escape_html(&pos.tag));
                        html.push_str("</span>\n");
                    }
                    for def in &sec.definitions {
                        html.push_str("<div class=\"definition-sense\">");
                        html.push_str(&sanitize_html(def));
                        html.push_str("</div>\n");
                    }
                    for ex in &sec.examples {
                        html.push_str("<div class=\"definition-example\">");
                        html.push_str(&sanitize_html(ex));
                        html.push_str("</div>\n");
                    }
                    html.push_str("</div>\n");
                }
                EntryBlock::Paragraph(p) => {
                    html.push_str("<p>");
                    html.push_str(&sanitize_html(p));
                    html.push_str("</p>\n");
                }
                EntryBlock::Table { headers, rows } => {
                    html.push_str("<table class=\"dict-table\">\n");
                    if !headers.is_empty() {
                        html.push_str("  <thead><tr>\n");
                        for h in headers {
                            html.push_str("    <th>");
                            html.push_str(&escape_html(h));
                            html.push_str("</th>\n");
                        }
                        html.push_str("  </tr></thead>\n");
                    }
                    html.push_str("  <tbody>\n");
                    for row in rows {
                        html.push_str("  <tr>\n");
                        for cell in row {
                            html.push_str("    <td>");
                            html.push_str(&escape_html(cell));
                            html.push_str("</td>\n");
                        }
                        html.push_str("  </tr>\n");
                    }
                    html.push_str("  </tbody>\n</table>\n");
                }
                EntryBlock::Link(l) => {
                    html.push_str("<a href=\"");
                    html.push_str(&escape_html(&l.target));
                    html.push_str("\" class=\"");
                    html.push_str(if l.is_external {
                        "ext-link"
                    } else {
                        "cross-ref"
                    });
                    html.push_str("\">");
                    html.push_str(&escape_html(&l.label));
                    html.push_str("</a>\n");
                }
                EntryBlock::RawHtml(raw) => {
                    html.push_str(&sanitize_html(raw));
                    html.push('\n');
                }
            }
        }
        if html.trim().is_empty() && (!self.phonetics.is_empty() || !self.pos_tags.is_empty()) {
            for p in &self.phonetics {
                html.push_str("<span class=\"phonetic-badge\">");
                html.push_str(&escape_html(&p.text));
                html.push_str("</span> ");
            }
            for pos in &self.pos_tags {
                html.push_str("<span class=\"pos-badge\">");
                html.push_str(&escape_html(&pos.tag));
                html.push_str("</span> ");
            }
        }
        html
    }

    pub fn save_as_html(&self) -> String {
        save_as_html(self)
    }

    pub fn to_plain_text(&self) -> String {
        let mut lines = Vec::new();
        lines.push(self.headword.clone());
        if !self.phonetics.is_empty() {
            let p = self
                .phonetics
                .iter()
                .map(|p| p.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            lines.push(p);
        }
        for block in &self.blocks {
            match block {
                EntryBlock::Header(h) => lines.push(h.clone()),
                EntryBlock::Phonetic(p) => lines.push(p.text.clone()),
                EntryBlock::PartOfSpeech(pos) => lines.push(pos.tag.clone()),
                EntryBlock::Section(sec) => {
                    for def in &sec.definitions {
                        lines.push(strip_tags(def));
                    }
                    for ex in &sec.examples {
                        lines.push(format!("  ex: {}", strip_tags(ex)));
                    }
                }
                EntryBlock::Paragraph(p) => lines.push(strip_tags(p)),
                EntryBlock::Table { rows, .. } => {
                    for r in rows {
                        lines.push(r.join(" | "));
                    }
                }
                EntryBlock::Link(l) => lines.push(l.label.clone()),
                EntryBlock::RawHtml(raw) => lines.push(strip_tags(raw)),
            }
        }
        lines.join("\n")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TextRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FindMatch {
    pub index: usize,
    pub range: TextRange,
    pub matched_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FindResult {
    pub query: String,
    pub total_matches: usize,
    pub matches: Vec<FindMatch>,
    pub highlighted_html: String,
}

impl FindResult {
    pub fn is_empty(&self) -> bool {
        self.matches.is_empty()
    }

    pub fn get_match(&self, index: usize) -> Option<&FindMatch> {
        self.matches.get(index)
    }
}

/// Find query occurrences in result content with case-insensitive search, selection bounds, and highlight markup (IO-04).
pub fn find_in_result(query: &str, content: &str) -> FindResult {
    let query_trimmed = query.trim();
    if query_trimmed.is_empty() {
        return FindResult {
            query: query.to_string(),
            total_matches: 0,
            matches: Vec::new(),
            highlighted_html: content.to_string(),
        };
    }

    let query_lower = query_trimmed.to_lowercase();
    let content_lower = content.to_lowercase();

    let mut matches = Vec::new();
    let mut highlighted = String::with_capacity(content.len() + 64);
    let mut last_idx = 0;
    let mut search_idx = 0;
    let mut match_count = 0;

    while let Some(found_idx) = content_lower[search_idx..].find(&query_lower) {
        let abs_start = search_idx + found_idx;
        let abs_end = abs_start + query_lower.len();

        // Ensure we are at valid UTF-8 char boundaries
        if content.is_char_boundary(abs_start) && content.is_char_boundary(abs_end) {
            let matched_text = content[abs_start..abs_end].to_string();
            matches.push(FindMatch {
                index: match_count,
                range: TextRange {
                    start: abs_start,
                    end: abs_end,
                },
                matched_text: matched_text.clone(),
            });

            highlighted.push_str(&content[last_idx..abs_start]);
            highlighted.push_str(&format!(
                "<mark class=\"find-highlight\" data-match-index=\"{match_count}\">{matched_text}</mark>"
            ));
            last_idx = abs_end;
            match_count += 1;
        }

        search_idx = abs_end.max(search_idx + 1);
    }

    highlighted.push_str(&content[last_idx..]);

    FindResult {
        query: query.to_string(),
        total_matches: matches.len(),
        matches,
        highlighted_html: highlighted,
    }
}

/// Parses dictionary XML/HTML definitions into a structured RichDocument.
pub fn parse_rich_document(headword: &str, raw: &str, attribution: Option<&str>) -> RichDocument {
    let mut doc = RichDocument::new(headword);
    if let Some(attr) = attribution {
        doc.attribution = Some(attr.to_string());
    }

    let mut current_pos: Option<PartOfSpeech> = None;
    let mut current_defs: Vec<String> = Vec::new();
    let mut current_examples: Vec<String> = Vec::new();

    // 1. Extract phonetic <M>...</M>
    let mut scan_pos = 0;
    let lower_raw = raw.to_lowercase();
    while let Some(m_start) = lower_raw[scan_pos..].find("<m>") {
        let abs_m_start = scan_pos + m_start + 3;
        if let Some(m_end) = lower_raw[abs_m_start..].find("</m>") {
            let abs_m_end = abs_m_start + m_end;
            let phonetic_text = raw[abs_m_start..abs_m_end].trim();
            if !phonetic_text.is_empty() {
                doc.phonetics.push(Phonetic::new(phonetic_text));
            }
            scan_pos = abs_m_end + 4;
        } else {
            break;
        }
    }

    // 2. Extract POS <P><U>...</U></P> or <U>...</U>
    scan_pos = 0;
    while let Some(u_start) = lower_raw[scan_pos..].find("<u>") {
        let abs_u_start = scan_pos + u_start + 3;
        if let Some(u_end) = lower_raw[abs_u_start..].find("</u>") {
            let abs_u_end = abs_u_start + u_end;
            let pos_text = raw[abs_u_start..abs_u_end].trim();
            if !pos_text.is_empty() {
                let pos = PartOfSpeech::new(pos_text);
                if !doc.pos_tags.contains(&pos) {
                    doc.pos_tags.push(pos.clone());
                }
                current_pos = Some(pos);
            }
            scan_pos = abs_u_end + 4;
        } else {
            break;
        }
    }

    // 3. Extract definitions <Q>...</Q>
    scan_pos = 0;
    while let Some(q_start) = lower_raw[scan_pos..].find("<q>") {
        let abs_q_start = scan_pos + q_start + 3;
        if let Some(q_end) = lower_raw[abs_q_start..].find("</q>") {
            let abs_q_end = abs_q_start + q_end;
            let def_text = raw[abs_q_start..abs_q_end].trim();
            if !def_text.is_empty() {
                current_defs.push(def_text.to_string());
            }
            scan_pos = abs_q_end + 4;
        } else {
            break;
        }
    }

    // 4. Extract examples <Z>...</Z>
    scan_pos = 0;
    while let Some(z_start) = lower_raw[scan_pos..].find("<z>") {
        let abs_z_start = scan_pos + z_start + 3;
        if let Some(z_end) = lower_raw[abs_z_start..].find("</z>") {
            let abs_z_end = abs_z_start + z_end;
            let ex_text = raw[abs_z_start..abs_z_end].trim();
            if !ex_text.is_empty() {
                current_examples.push(ex_text.to_string());
            }
            scan_pos = abs_z_end + 4;
        } else {
            break;
        }
    }

    // 5. Extract tables <table>...</table>
    scan_pos = 0;
    while let Some(tbl_start) = lower_raw[scan_pos..].find("<table") {
        let abs_tbl_start = scan_pos + tbl_start;
        if let Some(tbl_end) = lower_raw[abs_tbl_start..].find("</table>") {
            let abs_tbl_end = abs_tbl_start + tbl_end + 8;
            let table_html = &raw[abs_tbl_start..abs_tbl_end];
            let (headers, rows) = parse_simple_table(table_html);
            if !headers.is_empty() || !rows.is_empty() {
                doc.blocks.push(EntryBlock::Table { headers, rows });
            }
            scan_pos = abs_tbl_end;
        } else {
            break;
        }
    }

    // 6. Extract links <a href="...">...</a>
    scan_pos = 0;
    while let Some(a_start) = lower_raw[scan_pos..].find("<a ") {
        let abs_a_start = scan_pos + a_start;
        if let Some(a_end) = lower_raw[abs_a_start..].find("</a>") {
            let abs_a_end = abs_a_start + a_end + 4;
            let a_tag_html = &raw[abs_a_start..abs_a_end];
            if let Some(link) = parse_link(a_tag_html) {
                doc.blocks.push(EntryBlock::Link(link));
            }
            scan_pos = abs_a_end;
        } else {
            break;
        }
    }

    // If definition section was found from <Q> / <Z> tags:
    if !current_defs.is_empty() || !current_examples.is_empty() {
        doc.blocks.push(EntryBlock::Section(DefinitionSection {
            pos: current_pos,
            definitions: current_defs,
            examples: current_examples,
            synonyms: Vec::new(),
        }));
    } else {
        // Fallback: clean the HTML / XML and add as paragraph or sanitized HTML
        let sanitized = sanitize_html(raw);
        if !sanitized.trim().is_empty() {
            doc.blocks.push(EntryBlock::Paragraph(sanitized));
        }
    }

    doc
}

fn parse_simple_table(table_html: &str) -> (Vec<String>, Vec<Vec<String>>) {
    let mut headers = Vec::new();
    let mut rows = Vec::new();

    let lower = table_html.to_lowercase();
    let mut tr_pos = 0;
    while let Some(tr_start) = lower[tr_pos..].find("<tr") {
        let abs_tr_start = tr_pos + tr_start;
        if let Some(tr_end) = lower[abs_tr_start..].find("</tr>") {
            let abs_tr_end = abs_tr_start + tr_end + 5;
            let tr_content = &table_html[abs_tr_start..abs_tr_end];
            let tr_lower = tr_content.to_lowercase();

            // Check if th row
            let mut th_pos = 0;
            let mut th_row = Vec::new();
            while let Some(th_start) = tr_lower[th_pos..].find("<th") {
                let abs_th_start = th_pos + th_start;
                if let Some(gt) = tr_content[abs_th_start..].find('>') {
                    let cell_start = abs_th_start + gt + 1;
                    if let Some(th_end) = tr_lower[cell_start..].find("</th>") {
                        let cell_end = cell_start + th_end;
                        th_row.push(
                            strip_tags(&tr_content[cell_start..cell_end])
                                .trim()
                                .to_string(),
                        );
                        th_pos = cell_end + 5;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            if !th_row.is_empty() && headers.is_empty() {
                headers = th_row;
                tr_pos = abs_tr_end;
                continue;
            }

            // Check td cells
            let mut td_pos = 0;
            let mut row = Vec::new();
            while let Some(td_start) = tr_lower[td_pos..].find("<td") {
                let abs_td_start = td_pos + td_start;
                if let Some(gt) = tr_content[abs_td_start..].find('>') {
                    let cell_start = abs_td_start + gt + 1;
                    if let Some(td_end) = tr_lower[cell_start..].find("</td>") {
                        let cell_end = cell_start + td_end;
                        row.push(
                            strip_tags(&tr_content[cell_start..cell_end])
                                .trim()
                                .to_string(),
                        );
                        td_pos = cell_end + 5;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            if !row.is_empty() {
                rows.push(row);
            }

            tr_pos = abs_tr_end;
        } else {
            break;
        }
    }

    (headers, rows)
}

fn parse_link(a_tag: &str) -> Option<LinkReference> {
    let lower = a_tag.to_lowercase();
    let href_idx = lower.find("href=")?;
    let quote_start = href_idx + 5;
    let quote_char = a_tag.as_bytes().get(quote_start)?;
    let (target, after_quote) = if *quote_char == b'"' || *quote_char == b'\'' {
        let end_quote = a_tag[quote_start + 1..].find(*quote_char as char)?;
        (
            &a_tag[quote_start + 1..quote_start + 1 + end_quote],
            quote_start + 1 + end_quote + 1,
        )
    } else {
        let space_end = a_tag[quote_start..]
            .find(|c: char| c.is_whitespace() || c == '>')
            .unwrap_or(a_tag.len() - quote_start);
        (
            &a_tag[quote_start..quote_start + space_end],
            quote_start + space_end,
        )
    };

    let gt_idx = a_tag[after_quote..].find('>')?;
    let content_start = after_quote + gt_idx + 1;
    let close_idx = a_tag.to_lowercase().rfind("</a>").unwrap_or(a_tag.len());
    let label = if content_start < close_idx {
        strip_tags(&a_tag[content_start..close_idx])
            .trim()
            .to_string()
    } else {
        target.to_string()
    };

    let is_external = target.starts_with("http://")
        || target.starts_with("https://")
        || target.starts_with("ftp://");

    Some(LinkReference::new(target, label, is_external))
}

/// Strips all dangerous tags (`<script>`, `<object>`, `<iframe>`, `<embed>`, `<applet>`),
/// inline event handlers (`onload`, `onerror`, etc.), and `javascript:`/`vbscript:`/`data:text` URIs.
pub fn sanitize_html(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '<' {
            let mut tag_buf = String::new();
            let mut is_cdata = false;

            for next_c in chars.by_ref() {
                tag_buf.push(next_c);
                if !is_cdata && tag_buf.starts_with("![CDATA[") {
                    is_cdata = true;
                }
                if is_cdata {
                    if tag_buf.ends_with("]]>") {
                        break;
                    }
                } else if next_c == '>' {
                    break;
                }
            }

            if is_cdata && tag_buf.ends_with("]]>") {
                let inner = &tag_buf[8..tag_buf.len() - 3];
                out.push_str(&sanitize_html(inner));
                continue;
            }

            let trimmed = tag_buf.trim_end_matches('>').trim();
            let lower = trimmed.to_ascii_lowercase();

            // Check if tag is prohibited: script, object, iframe, embed, applet, meta, style, link
            let tag_name = lower
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_start_matches('/');

            if tag_name == "script"
                || tag_name == "object"
                || tag_name == "iframe"
                || tag_name == "embed"
                || tag_name == "applet"
            {
                // Discard entire block until matching closing tag
                let close_tag = format!("</{tag_name}>");
                let mut close_buf = String::new();
                for sc in chars.by_ref() {
                    close_buf.push(sc);
                    if close_buf.to_ascii_lowercase().ends_with(&close_tag) {
                        break;
                    }
                }
                continue;
            }

            // If it's a closing tag
            if let Some(rest) = trimmed.strip_prefix('/') {
                let name = rest.trim();
                out.push_str(&format!("</{name}>"));
                continue;
            }

            // Sanitize tag attributes
            let sanitized_tag = sanitize_element_tag(trimmed);
            if !sanitized_tag.is_empty() {
                out.push('<');
                out.push_str(&sanitized_tag);
                out.push('>');
            }
        } else {
            out.push(c);
        }
    }

    out
}

fn sanitize_element_tag(tag_str: &str) -> String {
    let parts: Vec<&str> = tag_str.splitn(2, char::is_whitespace).collect();
    let tag_name = parts[0];
    let tag_name_lower = tag_name.to_ascii_lowercase();

    if tag_name_lower == "script"
        || tag_name_lower == "object"
        || tag_name_lower == "iframe"
        || tag_name_lower == "embed"
        || tag_name_lower == "applet"
        || tag_name_lower == "form"
        || tag_name_lower == "input"
    {
        return String::new();
    }

    if parts.len() == 1 {
        return tag_name.to_string();
    }

    let attr_str = parts[1];
    let safe_attrs = sanitize_attributes(attr_str);
    if safe_attrs.is_empty() {
        tag_name.to_string()
    } else {
        format!("{tag_name} {safe_attrs}")
    }
}

fn sanitize_attributes(attr_str: &str) -> String {
    let mut safe = Vec::new();
    let mut chars = attr_str.chars().peekable();

    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }

        // Parse attr name
        let mut name = String::new();
        while let Some(&nc) = chars.peek() {
            if nc == '=' || nc.is_whitespace() || nc == '/' {
                break;
            }
            name.push(nc);
            chars.next();
        }

        while let Some(&nc) = chars.peek() {
            if nc.is_whitespace() {
                chars.next();
            } else {
                break;
            }
        }

        let mut val = String::new();
        if let Some(&'=') = chars.peek() {
            chars.next(); // consume '='
            while let Some(&nc) = chars.peek() {
                if nc.is_whitespace() {
                    chars.next();
                } else {
                    break;
                }
            }

            if let Some(&quote) = chars.peek() {
                if quote == '"' || quote == '\'' {
                    chars.next(); // consume opening quote
                    for qc in chars.by_ref() {
                        if qc == quote {
                            break;
                        }
                        val.push(qc);
                    }
                } else {
                    while let Some(&nc) = chars.peek() {
                        if nc.is_whitespace() || nc == '/' {
                            break;
                        }
                        val.push(nc);
                        chars.next();
                    }
                }
            }
        }

        let name_lower = name.to_ascii_lowercase();

        // 1. Strip any event handler starting with 'on'
        if name_lower.starts_with("on") {
            continue;
        }

        // 2. Strip dangerous URLs
        if name_lower == "href"
            || name_lower == "src"
            || name_lower == "action"
            || name_lower == "data"
        {
            let val_cleaned = val
                .chars()
                .filter(|c| !c.is_whitespace() && *c != '\n' && *c != '\r' && *c != '\t')
                .collect::<String>()
                .to_ascii_lowercase();

            if val_cleaned.starts_with("javascript:")
                || val_cleaned.starts_with("vbscript:")
                || (val_cleaned.starts_with("data:") && !val_cleaned.starts_with("data:image/"))
            {
                continue;
            }
        }

        // 3. Strip dangerous style expressions
        if name_lower == "style" {
            let val_lower = val.to_ascii_lowercase();
            if val_lower.contains("expression")
                || val_lower.contains("javascript:")
                || val_lower.contains("behavior:")
            {
                continue;
            }
        }

        if !name.is_empty() {
            if val.is_empty() {
                safe.push(name);
            } else {
                safe.push(format!("{name}=\"{}\"", escape_attr(&val)));
            }
        }
    }

    safe.join(" ")
}

pub fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

pub fn escape_attr(input: &str) -> String {
    escape_html(input)
}

pub fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut inside = false;
    for c in html.chars() {
        if c == '<' {
            inside = true;
        } else if c == '>' {
            inside = false;
        } else if !inside {
            out.push(c);
        }
    }
    out
}

/// Generates a complete standalone HTML document with clean CSS styling, meta tags, and dictionary attribution header (IO-02).
pub fn save_as_html(doc: &RichDocument) -> String {
    let title = format!(
        "{} - {}",
        doc.headword,
        doc.attribution.as_deref().unwrap_or("Dictionary")
    );
    let attribution_html = doc
        .attribution
        .as_ref()
        .map(|a| {
            format!(
                "<header class=\"dict-attribution\"><span>📖 {}</span></header>\n",
                escape_html(a)
            )
        })
        .unwrap_or_default();

    let phonetics_html = if !doc.phonetics.is_empty() {
        let badges: Vec<String> = doc
            .phonetics
            .iter()
            .map(|p| {
                format!(
                    "<span class=\"phonetic-badge\">{}</span>",
                    escape_html(&p.text)
                )
            })
            .collect();
        format!(
            "<div class=\"phonetics-container\">{}</div>\n",
            badges.join(" ")
        )
    } else {
        String::new()
    };

    let pos_html = if !doc.pos_tags.is_empty() {
        let badges: Vec<String> = doc
            .pos_tags
            .iter()
            .map(|pos| format!("<span class=\"pos-badge\">{}</span>", escape_html(&pos.tag)))
            .collect();
        format!("<div class=\"pos-container\">{}</div>\n", badges.join(" "))
    } else {
        String::new()
    };

    let body_content = doc.to_sanitized_html();

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <meta name="generator" content="LexiForge / Lingoes-RS">
  <title>{title}</title>
  <style>
    :root {{
      --bg: #f8fafc;
      --card-bg: #ffffff;
      --text: #0f172a;
      --text-muted: #64748b;
      --primary: #0f3d64;
      --primary-accent: #0284c7;
      --border: #e2e8f0;
    }}
    @media (prefers-color-scheme: dark) {{
      :root {{
        --bg: #0f172a;
        --card-bg: #1e293b;
        --text: #f8fafc;
        --text-muted: #94a3b8;
        --primary: #38bdf8;
        --primary-accent: #0284c7;
        --border: #334155;
      }}
    }}
    body {{
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
      background-color: var(--bg);
      color: var(--text);
      line-height: 1.6;
      margin: 0;
      padding: 24px 16px;
    }}
    .dict-container {{
      max-width: 820px;
      margin: 0 auto;
      background: var(--card-bg);
      border: 1px solid var(--border);
      border-radius: 12px;
      box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.05);
      padding: 32px;
    }}
    .dict-attribution {{
      background: #e0f2fe;
      color: #0369a1;
      border: 1px solid #7dd3fc;
      border-radius: 6px;
      padding: 6px 14px;
      font-size: 13px;
      font-weight: 600;
      display: inline-block;
      margin-bottom: 20px;
    }}
    .headword {{
      font-size: 32px;
      font-weight: 800;
      color: var(--primary);
      margin: 0 0 12px 0;
    }}
    .phonetic-badge {{
      display: inline-block;
      font-family: "Lucida Sans", "Lucida Grande", "Segoe UI", monospace;
      font-size: 15px;
      color: var(--text-muted);
      background: #f1f5f9;
      border: 1px solid #cbd5e1;
      padding: 3px 10px;
      border-radius: 6px;
      margin-right: 8px;
      margin-bottom: 8px;
    }}
    .pos-badge {{
      display: inline-block;
      font-size: 12px;
      font-weight: 700;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      color: #4338ca;
      background: #e0e7ff;
      border: 1px solid #c7d2fe;
      padding: 3px 10px;
      border-radius: 6px;
      margin-right: 8px;
      margin-bottom: 8px;
    }}
    .definition-section {{
      margin-top: 20px;
      padding-left: 16px;
      border-left: 3px solid var(--primary-accent);
    }}
    .definition-sense {{
      font-size: 16px;
      margin-bottom: 10px;
      color: var(--text);
    }}
    .definition-example {{
      font-size: 14px;
      font-style: italic;
      color: var(--text-muted);
      margin-left: 16px;
      margin-bottom: 8px;
    }}
    .dict-table {{
      border-collapse: collapse;
      width: 100%;
      margin: 20px 0;
      border-radius: 6px;
      overflow: hidden;
    }}
    .dict-table th, .dict-table td {{
      border: 1px solid var(--border);
      padding: 10px 14px;
      text-align: left;
    }}
    .dict-table th {{
      background-color: #f1f5f9;
      font-weight: 600;
    }}
    .cross-ref, .ext-link {{
      color: var(--primary-accent);
      text-decoration: none;
      font-weight: 500;
    }}
    .cross-ref:hover, .ext-link:hover {{
      text-decoration: underline;
    }}
    .find-highlight {{
      background-color: #fef08a;
      color: #713f12;
      padding: 1px 3px;
      border-radius: 3px;
    }}
    footer.dict-footer {{
      margin-top: 36px;
      padding-top: 16px;
      border-top: 1px solid var(--border);
      font-size: 12px;
      color: var(--text-muted);
      text-align: center;
    }}
    @media print {{
      body {{
        background: #ffffff !important;
        color: #000000 !important;
        padding: 0;
      }}
      .dict-container {{
        box-shadow: none;
        border: none;
        max-width: 100%;
        padding: 0;
      }}
    }}
  </style>
</head>
<body>
  <div class="dict-container">
    {attribution_html}
    <h1 class="headword">{}</h1>
    {phonetics_html}
    {pos_html}
    <div class="dict-content">
      {body_content}
    </div>
    <footer class="dict-footer">
      Generated by LexiForge (Lingoes-RS) - Standalone Dictionary Export
    </footer>
  </div>
</body>
</html>"#,
        escape_html(&doc.headword)
    )
}
