use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use lingoes_core::DictionaryMetadata;
use opendict::{DictEntry, DictInfo, Dictionary};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Ld2Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid magic header, expected '?LD2' or '?LDX'")]
    InvalidMagic,
    #[error("file is truncated or unexpected end of file")]
    Truncated,
    #[error("invalid offsets or lengths in dictionary header")]
    InvalidOffsets,
    #[error("decompression failed: {0}")]
    Decompression(String),
    #[error("malformed entry: {0}")]
    MalformedEntry(String),
}

/// Reverse-engineered clean-room LD2/LDX parser implementing opendict::Dictionary.
pub struct Ld2Dictionary {
    info: DictInfo,
    words: Vec<String>,
    entries: HashMap<String, Vec<String>>,
    metadata: DictionaryMetadata,
}

impl Ld2Dictionary {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, Ld2Error> {
        let path = path.as_ref();
        let bytes = std::fs::read(path)?;
        let name = path.file_stem().and_then(|s| s.to_str());
        Self::from_bytes(&bytes, name)
    }

    pub fn from_bytes(bytes: &[u8], name: Option<&str>) -> Result<Self, Ld2Error> {
        if bytes.len() < 0x60 {
            return Err(Ld2Error::Truncated);
        }

        let magic = &bytes[0..4];
        if magic != b"?LD2" && magic != b"?LDX" {
            return Err(Ld2Error::InvalidMagic);
        }

        let version = u32::from_le_bytes(bytes[0x18..0x1C].try_into().unwrap());
        let dict_id = u32::from_le_bytes(bytes[0x1C..0x20].try_into().unwrap());

        let offset_data_rel = u32::from_le_bytes(bytes[0x5C..0x60].try_into().unwrap()) as usize;
        let offset_data = offset_data_rel
            .checked_add(0x60)
            .ok_or(Ld2Error::InvalidOffsets)?;

        if offset_data > bytes.len() {
            return Err(Ld2Error::Truncated);
        }
        if offset_data == bytes.len() {
            return Ok(Self::empty(name, version, dict_id));
        }
        if offset_data + 4 > bytes.len() {
            return Err(Ld2Error::Truncated);
        }

        let dtype = u32::from_le_bytes(bytes[offset_data..offset_data + 4].try_into().unwrap());
        let offset_with_info = if dtype == 3 {
            offset_data
        } else {
            if offset_data + 8 > bytes.len() {
                return Err(Ld2Error::Truncated);
            }
            let rel =
                u32::from_le_bytes(bytes[offset_data + 4..offset_data + 8].try_into().unwrap())
                    as usize;
            offset_data
                .checked_add(rel)
                .and_then(|v| v.checked_add(12))
                .ok_or(Ld2Error::InvalidOffsets)?
        };

        if offset_with_info + 24 > bytes.len() {
            return Err(Ld2Error::Truncated);
        }

        let offset_index = offset_with_info
            .checked_add(0x1C)
            .ok_or(Ld2Error::InvalidOffsets)?;
        let val4 = u32::from_le_bytes(
            bytes[offset_with_info + 4..offset_with_info + 8]
                .try_into()
                .unwrap(),
        ) as usize;
        let val8 = u32::from_le_bytes(
            bytes[offset_with_info + 8..offset_with_info + 12]
                .try_into()
                .unwrap(),
        ) as usize;
        let inflated_words_index_length = u32::from_le_bytes(
            bytes[offset_with_info + 12..offset_with_info + 16]
                .try_into()
                .unwrap(),
        ) as usize;
        let inflated_words_length = u32::from_le_bytes(
            bytes[offset_with_info + 16..offset_with_info + 20]
                .try_into()
                .unwrap(),
        ) as usize;
        let inflated_xml_length = u32::from_le_bytes(
            bytes[offset_with_info + 20..offset_with_info + 24]
                .try_into()
                .unwrap(),
        ) as usize;

        let offset_compressed_data_header = offset_index
            .checked_add(val8)
            .ok_or(Ld2Error::InvalidOffsets)?;
        let limit = offset_with_info
            .checked_add(val4)
            .and_then(|v| v.checked_add(8))
            .ok_or(Ld2Error::InvalidOffsets)?;

        if limit > bytes.len()
            || offset_compressed_data_header
                .checked_add(8)
                .ok_or(Ld2Error::InvalidOffsets)?
                > limit
        {
            return Err(Ld2Error::Truncated);
        }

        let total_inflated_length = u32::from_le_bytes(
            bytes[offset_compressed_data_header..offset_compressed_data_header + 4]
                .try_into()
                .unwrap(),
        ) as usize;
        let chunk_size = u32::from_le_bytes(
            bytes[offset_compressed_data_header + 4..offset_compressed_data_header + 8]
                .try_into()
                .unwrap(),
        ) as usize;

        let expected_total = inflated_words_index_length
            .checked_add(inflated_words_length)
            .and_then(|v| v.checked_add(inflated_xml_length))
            .ok_or(Ld2Error::InvalidOffsets)?;

        if total_inflated_length != expected_total {
            return Err(Ld2Error::InvalidOffsets);
        }

        if total_inflated_length == 0 {
            return Ok(Self::empty(name, version, dict_id));
        }

        if chunk_size == 0 {
            return Err(Ld2Error::InvalidOffsets);
        }

        let num_chunks = total_inflated_length.div_ceil(chunk_size);
        let offsets_bytes_len = (num_chunks + 1)
            .checked_mul(4)
            .ok_or(Ld2Error::InvalidOffsets)?;
        let offsets_start = offset_compressed_data_header + 8;
        let compressed_data_start = offsets_start
            .checked_add(offsets_bytes_len)
            .ok_or(Ld2Error::InvalidOffsets)?;

        if compressed_data_start > limit {
            return Err(Ld2Error::Truncated);
        }

        let mut offsets = Vec::with_capacity(num_chunks + 1);
        for i in 0..=num_chunks {
            let pos = offsets_start + i * 4;
            let off = u32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap()) as usize;
            offsets.push(off);
        }

        // Decompress chunks with zlib
        let mut decompressed = Vec::with_capacity(total_inflated_length);
        for i in 0..num_chunks {
            let c_start = compressed_data_start
                .checked_add(offsets[i])
                .ok_or(Ld2Error::InvalidOffsets)?;
            let c_end = compressed_data_start
                .checked_add(offsets[i + 1])
                .ok_or(Ld2Error::InvalidOffsets)?;
            if c_start > c_end || c_end > limit {
                return Err(Ld2Error::Truncated);
            }
            let chunk_bytes = &bytes[c_start..c_end];
            let mut decoder = flate2::read::ZlibDecoder::new(chunk_bytes);
            decoder
                .read_to_end(&mut decompressed)
                .map_err(|e| Ld2Error::Decompression(e.to_string()))?;
        }

        if decompressed.len() != total_inflated_length {
            return Err(Ld2Error::Decompression(format!(
                "expected {total_inflated_length} bytes, got {}",
                decompressed.len()
            )));
        }

        if inflated_words_index_length % 10 != 0 {
            return Err(Ld2Error::InvalidOffsets);
        }
        let num_entries = inflated_words_index_length / 10;
        let index_slice = &decompressed[0..inflated_words_index_length];
        let words_start = inflated_words_index_length;
        let words_end = words_start + inflated_words_length;
        let xml_start = words_end;
        let xml_end = xml_start + inflated_xml_length;

        let words_slice = &decompressed[words_start..words_end];
        let xml_slice = &decompressed[xml_start..xml_end];

        struct RawEntry {
            word_pos: usize,
            xml_pos: usize,
            refs: u8,
        }

        let mut raw_entries = Vec::with_capacity(num_entries);
        for i in 0..num_entries {
            let pos = i * 10;
            let w_pos = u32::from_le_bytes(index_slice[pos..pos + 4].try_into().unwrap()) as usize;
            let x_pos =
                u32::from_le_bytes(index_slice[pos + 4..pos + 8].try_into().unwrap()) as usize;
            let refs = index_slice[pos + 9];
            raw_entries.push(RawEntry {
                word_pos: w_pos,
                xml_pos: x_pos,
                refs,
            });
        }

        struct DecodedItem {
            word: String,
            ref_indices: Vec<usize>,
            raw_def: String,
        }

        let mut items = Vec::with_capacity(num_entries);
        for i in 0..num_entries {
            let entry = &raw_entries[i];
            let w_next = if i + 1 < num_entries {
                raw_entries[i + 1].word_pos
            } else {
                inflated_words_length
            };
            let x_next = if i + 1 < num_entries {
                raw_entries[i + 1].xml_pos
            } else {
                inflated_xml_length
            };

            if entry.word_pos > w_next || w_next > words_slice.len() {
                return Err(Ld2Error::MalformedEntry(format!(
                    "entry {i} word slice out of bounds"
                )));
            }
            if entry.xml_pos > x_next || x_next > xml_slice.len() {
                return Err(Ld2Error::MalformedEntry(format!(
                    "entry {i} xml slice out of bounds"
                )));
            }

            let word_slice = &words_slice[entry.word_pos..w_next];
            let xml_slice_entry = &xml_slice[entry.xml_pos..x_next];

            let num_refs = entry.refs as usize;
            let (ref_indices, word_text_bytes) = if num_refs > 0 {
                let refs_byte_len = num_refs * 4;
                if refs_byte_len > word_slice.len() {
                    return Err(Ld2Error::MalformedEntry(format!(
                        "entry {i} invalid refs length"
                    )));
                }
                let mut refs = Vec::with_capacity(num_refs);
                for r in 0..num_refs {
                    let r_idx =
                        u32::from_le_bytes(word_slice[r * 4..(r + 1) * 4].try_into().unwrap())
                            as usize;
                    refs.push(r_idx);
                }
                (refs, &word_slice[refs_byte_len..])
            } else {
                (Vec::new(), word_slice)
            };

            let word_str = decode_text(word_text_bytes);
            let raw_def_str = if !xml_slice_entry.is_empty() {
                clean_lingoes_xml(&decode_text(xml_slice_entry))
            } else {
                String::new()
            };

            items.push(DecodedItem {
                word: word_str,
                ref_indices,
                raw_def: raw_def_str,
            });
        }

        let mut words_list = Vec::with_capacity(num_entries);
        let mut definitions_map: HashMap<String, Vec<String>> = HashMap::new();

        for i in 0..num_entries {
            let item = &items[i];
            if item.word.is_empty() {
                continue;
            }

            let mut def_parts = Vec::new();
            if !item.raw_def.is_empty() {
                def_parts.push(item.raw_def.clone());
            }

            for &ref_idx in &item.ref_indices {
                if ref_idx < items.len() && ref_idx != i {
                    let ref_def = &items[ref_idx].raw_def;
                    if !ref_def.is_empty() && !def_parts.contains(ref_def) {
                        def_parts.push(ref_def.clone());
                    }
                }
            }

            let combined_def = def_parts.join("\n\n");
            if combined_def.is_empty() {
                continue;
            }

            if !definitions_map.contains_key(&item.word) {
                words_list.push(item.word.clone());
            }
            definitions_map
                .entry(item.word.clone())
                .or_default()
                .push(combined_def);
        }

        let mut metadata = DictionaryMetadata {
            format: Some("ld2".to_string()),
            entry_count: Some(words_list.len()),
            ..DictionaryMetadata::default()
        };

        if dtype != 3 && offset_data + 8 < offset_with_info && offset_with_info <= bytes.len() {
            let meta_slice = &bytes[offset_data + 8..offset_with_info];
            parse_ld2_metadata_slice(meta_slice, &mut metadata);
        }
        if offset_data > 0x20 && offset_data <= bytes.len() {
            let header_slice = &bytes[0x20..offset_data];
            parse_ld2_metadata_slice(header_slice, &mut metadata);
        }

        let dict_name = if !metadata.title.is_empty() {
            metadata.title.clone()
        } else {
            name.unwrap_or("Lingoes Dictionary").to_string()
        };
        metadata.title = dict_name.clone();
        let word_count = words_list.len();

        Ok(Self {
            info: create_dict_info(
                dict_name,
                metadata
                    .author
                    .clone()
                    .unwrap_or_else(|| "Lingoes".to_string()),
                metadata
                    .description
                    .clone()
                    .unwrap_or_else(|| format!("Version: {version:#x}, Dict ID: {dict_id:#x}")),
                word_count,
            ),
            words: words_list,
            entries: definitions_map,
            metadata,
        })
    }

    pub fn metadata(&self) -> &DictionaryMetadata {
        &self.metadata
    }

    fn empty(name: Option<&str>, version: u32, dict_id: u32) -> Self {
        let dict_name = name.unwrap_or("Lingoes Dictionary").to_string();
        Self {
            info: create_dict_info(
                dict_name.clone(),
                "Lingoes".to_string(),
                format!("Version: {version:#x}, Dict ID: {dict_id:#x}"),
                0,
            ),
            words: Vec::new(),
            entries: HashMap::new(),
            metadata: DictionaryMetadata {
                title: dict_name,
                format: Some("ld2".to_string()),
                entry_count: Some(0),
                ..Default::default()
            },
        }
    }
}

fn create_dict_info(
    name: String,
    author: String,
    description: String,
    word_count: usize,
) -> DictInfo {
    let mut uninit = std::mem::MaybeUninit::<DictInfo>::uninit();
    let ptr = uninit.as_mut_ptr();
    unsafe {
        std::ptr::addr_of_mut!((*ptr).name).write(name);
        std::ptr::addr_of_mut!((*ptr).author).write(author);
        std::ptr::addr_of_mut!((*ptr).description).write(description);
        std::ptr::addr_of_mut!((*ptr).word_count).write(word_count);
        uninit.assume_init()
    }
}

impl Dictionary for Ld2Dictionary {
    fn lookup(&self, word: &str) -> opendict::Result<Option<Vec<DictEntry>>> {
        let trimmed = word.trim();
        if let Some(defs) = self.entries.get(trimmed) {
            let entries = defs
                .iter()
                .map(|d| DictEntry::new('h', d.as_bytes().to_vec()))
                .collect();
            Ok(Some(entries))
        } else {
            Ok(None)
        }
    }

    fn lookup_synonym(&self, _word: &str) -> opendict::Result<Option<Vec<DictEntry>>> {
        Ok(None)
    }

    fn word_list(&self) -> Vec<&str> {
        self.words.iter().map(|s| s.as_str()).collect()
    }

    fn word_count(&self) -> usize {
        self.words.len()
    }

    fn info(&self) -> &DictInfo {
        &self.info
    }

    fn search_prefix(&self, prefix: &str, limit: usize) -> Vec<String> {
        let prefix_lower = prefix.to_lowercase();
        self.words
            .iter()
            .filter(|w| w.to_lowercase().starts_with(&prefix_lower))
            .take(limit)
            .cloned()
            .collect()
    }
}

/// Decodes binary bytes into text using UTF-8 or UTF-16LE.
pub fn decode_text(bytes: &[u8]) -> String {
    // Check if it's UTF-16LE: non-empty, even length, and either starts with typical ASCII pair or fails utf-8
    let is_likely_utf16 =
        bytes.len() >= 2 && bytes.len() % 2 == 0 && (bytes[1] == 0 && bytes[0] != 0);

    if is_likely_utf16 {
        let u16s: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        if let Ok(s) = String::from_utf16(&u16s) {
            return s.trim_matches('\0').trim().to_string();
        }
    }

    // Try standard UTF-8
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.trim_matches('\0').trim().to_string();
    }

    // Secondary UTF-16LE check
    if bytes.len() >= 2 && bytes.len() % 2 == 0 {
        let u16s: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        if let Ok(s) = String::from_utf16(&u16s) {
            return s.trim_matches('\0').trim().to_string();
        }
    }

    String::from_utf8_lossy(bytes)
        .trim_matches('\0')
        .trim()
        .to_string()
}

/// Cleans and formats Lingoes XML markup (<C>, <F>, <H><M>phonetic</M></H>, <I><N><P><U>pos</U></P><Q>def</Q></N></I>)
/// as well as CDATA sections and basic HTML tags into clean readable definition text.
pub fn clean_lingoes_xml(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '<' {
            let mut tag = String::new();
            let mut is_cdata = false;
            for next_c in chars.by_ref() {
                tag.push(next_c);
                if !is_cdata && tag.starts_with("![CDATA[") {
                    is_cdata = true;
                }
                if is_cdata {
                    if tag.ends_with("]]>") {
                        break;
                    }
                } else if next_c == '>' {
                    break;
                }
            }
            if is_cdata && tag.ends_with("]]>") {
                let inner = &tag[8..tag.len() - 3];
                out.push_str(&decode_html_entities(inner));
                continue;
            }
            let tag_trimmed = tag.trim_end_matches('>').trim();
            let tag_lower = tag_trimmed.to_ascii_lowercase();

            if tag_lower == "m" {
                // Phonetic tag: gather text until </m> or </M>
                let mut m_content = String::new();
                while let Some(mc) = chars.next() {
                    if mc == '<' {
                        let mut close_tag = String::new();
                        for nc in chars.by_ref() {
                            if nc == '>' {
                                break;
                            }
                            close_tag.push(nc);
                        }
                        if close_tag.trim().eq_ignore_ascii_case("/m") {
                            break;
                        }
                    } else {
                        m_content.push(mc);
                    }
                }
                let m_clean = m_content.trim();
                if !m_clean.is_empty() {
                    if !out.is_empty() && !out.ends_with('\n') {
                        out.push('\n');
                    }
                    out.push('[');
                    out.push_str(m_clean);
                    out.push(']');
                    out.push('\n');
                }
            } else if tag_lower == "u" {
                if !out.is_empty() && !out.ends_with('\n') && !out.ends_with(' ') {
                    out.push(' ');
                }
            } else if tag_lower == "/u" {
                out.push(' ');
            } else if tag_lower == "q" {
                if !out.is_empty() && !out.ends_with('\n') && !out.ends_with(' ') {
                    out.push(' ');
                }
            } else if (tag_lower == "/q"
                || tag_lower == "/n"
                || tag_lower == "/i"
                || tag_lower == "/p"
                || tag_lower == "br"
                || tag_lower == "br/"
                || tag_lower == "br /"
                || tag_lower == "tr"
                || tag_lower == "/div")
                && !out.is_empty()
                && !out.ends_with('\n')
            {
                out.push('\n');
            }
        } else if c == '&' {
            let mut raw_ent = String::from("&");
            while let Some(&next_c) = chars.peek() {
                if next_c == ';' {
                    raw_ent.push(';');
                    chars.next();
                    break;
                }
                if next_c.is_alphanumeric() || next_c == '#' {
                    chars.next();
                    raw_ent.push(next_c);
                } else {
                    break;
                }
            }
            out.push_str(&decode_html_entities(&raw_ent));
        } else {
            out.push(c);
        }
    }

    // Normalize whitespace and collapse excess blank lines
    let mut normalized = String::new();
    let mut prev_newline_count = 0;
    for line in out.lines() {
        let trimmed_line = line.trim();
        if trimmed_line.is_empty() {
            if prev_newline_count < 1 && !normalized.is_empty() {
                normalized.push('\n');
                prev_newline_count += 1;
            }
        } else {
            if !normalized.is_empty() && prev_newline_count > 0 {
                normalized.push('\n');
            }
            normalized.push_str(trimmed_line);
            normalized.push('\n');
            prev_newline_count = 0;
        }
    }
    normalized.trim().to_string()
}

pub fn decode_html_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&' {
            let mut entity = String::new();
            while let Some(&nc) = chars.peek() {
                if nc == ';' {
                    chars.next();
                    break;
                }
                if nc.is_alphanumeric() || nc == '#' {
                    chars.next();
                    entity.push(nc);
                } else {
                    break;
                }
            }
            match entity.as_str() {
                "amp" => out.push('&'),
                "lt" => out.push('<'),
                "gt" => out.push('>'),
                "quot" => out.push('"'),
                "apos" => out.push('\''),
                ent if ent.starts_with('#') => {
                    let code_str = &ent[1..];
                    let code = if let Some(hex) = code_str
                        .strip_prefix('x')
                        .or_else(|| code_str.strip_prefix('X'))
                    {
                        u32::from_str_radix(hex, 16).ok()
                    } else {
                        code_str.parse::<u32>().ok()
                    };
                    if let Some(ch) = code.and_then(char::from_u32) {
                        out.push(ch);
                    } else {
                        out.push('&');
                        out.push_str(&entity);
                        out.push(';');
                    }
                }
                _ => {
                    out.push('&');
                    out.push_str(&entity);
                    if !entity.is_empty() {
                        out.push(';');
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn parse_ld2_metadata_slice(slice: &[u8], meta: &mut DictionaryMetadata) {
    if slice.is_empty() {
        return;
    }

    let mut text_candidates = Vec::new();
    if let Ok(utf8_str) = std::str::from_utf8(slice) {
        text_candidates.push(utf8_str.to_string());
    } else {
        text_candidates.push(String::from_utf8_lossy(slice).to_string());
    }

    if slice.len() >= 2 {
        let u16s: Vec<u16> = slice
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        if let Ok(utf16_str) = String::from_utf16(&u16s) {
            text_candidates.push(utf16_str);
        }
    }

    for text in text_candidates {
        extract_metadata_from_text(&text, meta);
    }
}

fn extract_metadata_from_text(text: &str, meta: &mut DictionaryMetadata) {
    let lower = text.to_lowercase();
    let extract_attr_or_tag = |keys: &[&str]| -> Option<String> {
        for &k in keys {
            let k_attr = format!("{k}=\"");
            if let Some(pos) = lower.find(&k_attr) {
                let val_start = pos + k_attr.len();
                if let Some(end) = text[val_start..].find('"') {
                    let val = text[val_start..val_start + end].trim();
                    if !val.is_empty() {
                        return Some(val.to_string());
                    }
                }
            }
            let k_tag = format!("<{k}>");
            let k_close = format!("</{k}>");
            if let Some(pos) = lower.find(&k_tag) {
                let val_start = pos + k_tag.len();
                if let Some(end) = lower[val_start..].find(&k_close) {
                    let val = text[val_start..val_start + end].trim();
                    if !val.is_empty() {
                        return Some(val.to_string());
                    }
                }
            }
        }
        None
    };

    if meta.title.is_empty() || meta.title == "Lingoes Dictionary" {
        if let Some(t) = extract_attr_or_tag(&["title", "bookname", "name"]) {
            meta.title = t;
        }
    }
    if meta.author.is_none() {
        meta.author = extract_attr_or_tag(&["author", "creator"]);
    }
    if meta.publisher.is_none() {
        meta.publisher = extract_attr_or_tag(&["publisher"]);
    }
    if meta.website.is_none() {
        meta.website = extract_attr_or_tag(&["website", "url", "link"]);
    }
    if meta.copyright.is_none() {
        meta.copyright = extract_attr_or_tag(&["copyright"]);
    }
    if meta.description.is_none() {
        meta.description = extract_attr_or_tag(&["description", "summary", "intro"]);
    }
    if meta.source_lang.is_none() {
        meta.source_lang = extract_attr_or_tag(&["langfrom", "from", "sourcelang"]);
    }
    if meta.target_lang.is_none() {
        meta.target_lang = extract_attr_or_tag(&["langto", "to", "targetlang"]);
    }
}
