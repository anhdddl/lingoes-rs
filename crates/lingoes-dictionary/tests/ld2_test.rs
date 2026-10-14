use std::io::Write;
use std::path::Path;

use flate2::Compression;
use flate2::write::ZlibEncoder;
use lingoes_core::LookupQuery;
use lingoes_dictionary::ld2::{Ld2Dictionary, clean_lingoes_xml};
use lingoes_dictionary::{DictionaryFormat, SqliteDictionary, detect_format};
use opendict::Dictionary;
use tempfile::NamedTempFile;

/// Clean-room synthetic LD2/LDX generator for tests.
/// Synthesizes valid minimal binary buffers adhering to the reverse-engineered specification.
pub fn build_synthetic_ld2(
    entries: &[(&str, &str)],
    as_ldx: bool,
    use_utf16: bool,
    refs_map: Option<&[Vec<u32>]>,
) -> Vec<u8> {
    let mut index_bytes = Vec::new();
    let mut words_bytes = Vec::new();
    let mut xml_bytes = Vec::new();

    for (i, (word, xml)) in entries.iter().enumerate() {
        let w_pos = words_bytes.len() as u32;
        let x_pos = xml_bytes.len() as u32;
        let refs = refs_map.and_then(|m| m.get(i)).cloned().unwrap_or_default();
        let num_refs = refs.len() as u8;

        // Index entry: 10 bytes [word_pos: u32, xml_pos: u32, flag: u8, refs: u8]
        index_bytes.extend_from_slice(&w_pos.to_le_bytes());
        index_bytes.extend_from_slice(&x_pos.to_le_bytes());
        index_bytes.push(0); // flag
        index_bytes.push(num_refs); // refs

        // Word slice: refs * 4 bytes + word text
        for r in &refs {
            words_bytes.extend_from_slice(&r.to_le_bytes());
        }
        if use_utf16 {
            for c in word.encode_utf16() {
                words_bytes.extend_from_slice(&c.to_le_bytes());
            }
        } else {
            words_bytes.extend_from_slice(word.as_bytes());
        }

        // XML slice
        if use_utf16 {
            for c in xml.encode_utf16() {
                xml_bytes.extend_from_slice(&c.to_le_bytes());
            }
        } else {
            xml_bytes.extend_from_slice(xml.as_bytes());
        }
    }

    let idx_len = index_bytes.len() as u32;
    let w_len = words_bytes.len() as u32;
    let x_len = xml_bytes.len() as u32;

    let mut inflated = Vec::new();
    inflated.extend_from_slice(&index_bytes);
    inflated.extend_from_slice(&words_bytes);
    inflated.extend_from_slice(&xml_bytes);

    let total_inflated = inflated.len() as u32;
    let chunk_size: usize = 4096;
    let num_chunks = if total_inflated == 0 {
        0
    } else {
        (total_inflated as usize).div_ceil(chunk_size)
    };

    let mut compressed_chunks = Vec::new();
    let mut offsets = vec![0u32];

    for i in 0..num_chunks {
        let start = i * chunk_size;
        let end = (start + chunk_size).min(inflated.len());
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&inflated[start..end]).unwrap();
        let compressed = encoder.finish().unwrap();
        let last_offset = *offsets.last().unwrap();
        offsets.push(last_offset + compressed.len() as u32);
        compressed_chunks.push(compressed);
    }

    let mut offsets_bytes = Vec::new();
    for o in &offsets {
        offsets_bytes.extend_from_slice(&o.to_le_bytes());
    }

    let mut compressed_data = Vec::new();
    for c in &compressed_chunks {
        compressed_data.extend_from_slice(c);
    }

    let mut comp_header = Vec::new();
    comp_header.extend_from_slice(&total_inflated.to_le_bytes());
    comp_header.extend_from_slice(&(chunk_size as u32).to_le_bytes());
    comp_header.extend_from_slice(&offsets_bytes);
    comp_header.extend_from_slice(&compressed_data);

    let offset_with_info = 0x60usize;
    let val8 = 0u32;
    let val4 = (0x1C + comp_header.len() - 8) as u32;

    let mut info_block = Vec::new();
    info_block.extend_from_slice(&3u32.to_le_bytes()); // dtype = 3
    info_block.extend_from_slice(&val4.to_le_bytes());
    info_block.extend_from_slice(&val8.to_le_bytes());
    info_block.extend_from_slice(&idx_len.to_le_bytes());
    info_block.extend_from_slice(&w_len.to_le_bytes());
    info_block.extend_from_slice(&x_len.to_le_bytes());
    info_block.extend_from_slice(&[0u8; 4]); // padding to 0x1C (28 bytes)
    info_block.extend_from_slice(&comp_header);

    let mut header = vec![0u8; offset_with_info];
    let magic = if as_ldx { b"?LDX" } else { b"?LD2" };
    header[0..4].copy_from_slice(magic);
    header[0x18..0x1C].copy_from_slice(&2u32.to_le_bytes());
    header[0x1C..0x20].copy_from_slice(&0x1234u32.to_le_bytes());
    header[0x5C..0x60].copy_from_slice(&0u32.to_le_bytes()); // offsetData - 0x60 = 0

    header.extend_from_slice(&info_block);
    header
}

#[test]
fn test_detect_format_ld2_and_ldx() {
    assert_eq!(
        detect_format(Path::new("sample.ld2")).unwrap(),
        DictionaryFormat::Ld2
    );
    assert_eq!(
        detect_format(Path::new("SAMPLE.LD2")).unwrap(),
        DictionaryFormat::Ld2
    );
    assert_eq!(
        detect_format(Path::new("sample.ldx")).unwrap(),
        DictionaryFormat::Ld2
    );
    assert_eq!(
        detect_format(Path::new("sample.ldp")).unwrap(),
        DictionaryFormat::Ldp
    );
    assert_eq!(
        detect_format(Path::new("sample.ldb")).unwrap(),
        DictionaryFormat::Ldb
    );
}

#[test]
fn test_parse_synthetic_ld2_basic() {
    let entries = [
        (
            "apple",
            "<C><F><H /><I><N><P><U>n.</U></P><Q>a crisp round fruit</Q></N></I></F></C>",
        ),
        (
            "banana",
            "<C><F><H /><I><N><P><U>n.</U></P><Q>a curved yellow fruit</Q></N></I></F></C>",
        ),
    ];
    let buf = build_synthetic_ld2(&entries, false, false, None);
    let dict = Ld2Dictionary::from_bytes(&buf, Some("Synthetic Dict")).expect("parse ld2");

    assert_eq!(dict.word_count(), 2);
    let words = dict.word_list();
    assert_eq!(words, vec!["apple", "banana"]);

    let apple_entries = dict.lookup("apple").unwrap().expect("apple lookup");
    assert_eq!(apple_entries.len(), 1);
    let def = String::from_utf8(apple_entries[0].data.clone()).unwrap();
    assert!(def.contains("a crisp round fruit"));
    assert!(def.contains("n."));

    let banana_entries = dict.lookup("banana").unwrap().expect("banana lookup");
    let def_b = String::from_utf8(banana_entries[0].data.clone()).unwrap();
    assert!(def_b.contains("a curved yellow fruit"));

    assert!(dict.lookup("cherry").unwrap().is_none());
}

#[test]
fn test_parse_synthetic_ldx_basic() {
    let entries = [(
        "rust",
        "<C><F><H><M>rʌst</M></H><I><N><P><U>n.</U></P><Q>a systems programming language</Q></N></I></F></C>",
    )];
    let buf = build_synthetic_ld2(&entries, true, false, None);
    let dict = Ld2Dictionary::from_bytes(&buf, Some("LDX Dict")).expect("parse ldx");

    assert_eq!(dict.word_count(), 1);
    let entries = dict.lookup("rust").unwrap().expect("rust lookup");
    let def = String::from_utf8(entries[0].data.clone()).unwrap();
    assert!(def.contains("[rʌst]"));
    assert!(def.contains("a systems programming language"));
}

#[test]
fn test_parse_synthetic_ld2_utf16() {
    let entries = [(
        "thành phố",
        "<C><F><H /><I><N><P><U>n.</U></P><Q>khu dân cư đô thị</Q></N></I></F></C>",
    )];
    let buf = build_synthetic_ld2(&entries, false, true, None);
    let dict = Ld2Dictionary::from_bytes(&buf, Some("UTF-16 Dict")).expect("parse utf16");

    assert_eq!(dict.word_count(), 1);
    assert_eq!(dict.word_list(), vec!["thành phố"]);
    let entries = dict.lookup("thành phố").unwrap().expect("lookup");
    let def = String::from_utf8(entries[0].data.clone()).unwrap();
    assert!(def.contains("khu dân cư đô thị"));
}

#[test]
fn test_parse_synthetic_ld2_with_references() {
    let entries = [
        (
            "fruit",
            "<C><F><H /><I><N><P><U>n.</U></P><Q>edible reproductive body of a seed plant</Q></N></I></F></C>",
        ),
        (
            "apple",
            "<C><F><H /><I><N><P><U>n.</U></P><Q>crisp pomaceous fruit</Q></N></I></F></C>",
        ),
    ];
    // Entry 1 (apple) references entry 0 (fruit)
    let refs = vec![vec![], vec![0u32]];
    let buf = build_synthetic_ld2(&entries, false, false, Some(&refs));
    let dict = Ld2Dictionary::from_bytes(&buf, Some("Ref Dict")).expect("parse ref dict");

    assert_eq!(dict.word_count(), 2);
    let apple_res = dict.lookup("apple").unwrap().expect("apple lookup");
    let def = String::from_utf8(apple_res[0].data.clone()).unwrap();
    // Combined definitions
    assert!(def.contains("crisp pomaceous fruit"));
    assert!(def.contains("edible reproductive body"));
}

#[test]
fn test_xml_cleanup_lingoes_tags() {
    let raw = "<C><F><H><M>fɹuːt</M></H><I><N><P><U>n.</U></P><Q>sweet edible produce</Q></N></I></F></C>";
    let cleaned = clean_lingoes_xml(raw);
    assert!(cleaned.contains("[fɹuːt]"));
    assert!(cleaned.contains("n."));
    assert!(cleaned.contains("sweet edible produce"));
}

#[test]
fn test_xml_cleanup_cdata_and_html() {
    let raw = "<b>Title</b><br><![CDATA[Some &lt;raw&gt; text]]><p>Paragraph 2 &amp; more</p>";
    let cleaned = clean_lingoes_xml(raw);
    assert!(cleaned.contains("Title"));
    assert!(cleaned.contains("Some <raw> text"));
    assert!(cleaned.contains("Paragraph 2 & more"));
}

#[test]
fn test_malformed_and_truncated_safe_rejection() {
    // Empty
    assert!(Ld2Dictionary::from_bytes(&[], None).is_err());
    // Truncated header
    assert!(Ld2Dictionary::from_bytes(&[0x3F, 0x4C, 0x44, 0x32, 0, 0, 0], None).is_err());
    // Wrong magic
    let mut bad_magic = vec![0u8; 100];
    bad_magic[0..4].copy_from_slice(b"BAD!");
    assert!(Ld2Dictionary::from_bytes(&bad_magic, None).is_err());
}

#[test]
fn test_import_path_ld2_into_database() {
    let entries = [
        (
            "orange",
            "<C><F><H /><I><N><P><U>n.</U></P><Q>a round citrus fruit</Q></N></I></F></C>",
        ),
        (
            "lemon",
            "<C><F><H /><I><N><P><U>n.</U></P><Q>a sour yellow citrus fruit</Q></N></I></F></C>",
        ),
    ];
    let buf = build_synthetic_ld2(&entries, false, false, None);
    let mut temp = NamedTempFile::with_suffix(".ld2").unwrap();
    temp.write_all(&buf).unwrap();

    let mut dict = SqliteDictionary::in_memory().unwrap();
    let report = dict
        .import_path_with_name(temp.path(), Some("Citrus Dict"))
        .expect("import ld2");
    assert_eq!(report.inserted, 2);

    let res = dict
        .lookup_exact(&LookupQuery::new("orange").unwrap())
        .unwrap();
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].headword(), "orange");
    assert!(res[0].definition().contains("round citrus fruit"));

    let prefix_res = dict
        .lookup_prefix(&LookupQuery::new("lem").unwrap(), 10)
        .unwrap();
    assert_eq!(prefix_res.len(), 1);
    assert_eq!(prefix_res[0].headword(), "lemon");

    let fts_res = dict
        .lookup_full_text(&LookupQuery::new("citrus").unwrap(), 10)
        .unwrap();
    assert_eq!(fts_res.len(), 2);
}

#[test]
fn test_import_path_ldx_into_database() {
    let entries = [(
        "blueberry",
        "<C><F><H /><I><N><P><U>n.</U></P><Q>a small dark-blue berry</Q></N></I></F></C>",
    )];
    let buf = build_synthetic_ld2(&entries, true, false, None);
    let mut temp = NamedTempFile::with_suffix(".ldx").unwrap();
    temp.write_all(&buf).unwrap();

    let mut dict = SqliteDictionary::in_memory().unwrap();
    let report = dict.import_path(temp.path()).expect("import ldx");
    assert_eq!(report.inserted, 1);

    let res = dict
        .lookup_exact(&LookupQuery::new("blueberry").unwrap())
        .unwrap();
    assert_eq!(res.len(), 1);
    assert!(res[0].definition().contains("dark-blue berry"));
}

#[test]
fn test_import_path_ldp_into_database() {
    use zip::write::SimpleFileOptions;

    let entries = [(
        "strawberry",
        "<C><F><H /><I><N><P><U>n.</U></P><Q>a sweet red fruit with seeds on surface</Q></N></I></F></C>",
    )];
    let ld2_buf = build_synthetic_ld2(&entries, false, false, None);

    let temp_zip = NamedTempFile::with_suffix(".ldp").unwrap();
    let mut zip_writer = zip::ZipWriter::new(std::fs::File::create(temp_zip.path()).unwrap());
    zip_writer
        .start_file("dict.ld2", SimpleFileOptions::default())
        .unwrap();
    zip_writer.write_all(&ld2_buf).unwrap();
    zip_writer.finish().unwrap();

    let mut dict = SqliteDictionary::in_memory().unwrap();
    let report = dict
        .import_path_with_name(temp_zip.path(), Some("Berry LDP"))
        .expect("import ldp");
    assert_eq!(report.inserted, 1);

    let res = dict
        .lookup_exact(&LookupQuery::new("strawberry").unwrap())
        .unwrap();
    assert_eq!(res.len(), 1);
    assert!(res[0].definition().contains("sweet red fruit"));
}

#[test]
fn test_import_path_ldb_into_database() {
    use zip::write::SimpleFileOptions;

    let entries = [(
        "grape",
        "<C><F><H /><I><N><P><U>n.</U></P><Q>berry growing in clusters on a grapevine</Q></N></I></F></C>",
    )];
    let ld2_buf = build_synthetic_ld2(&entries, false, false, None);

    let temp_zip = NamedTempFile::with_suffix(".ldb").unwrap();
    let mut zip_writer = zip::ZipWriter::new(std::fs::File::create(temp_zip.path()).unwrap());
    zip_writer
        .start_file("backup_dict.ld2", SimpleFileOptions::default())
        .unwrap();
    zip_writer.write_all(&ld2_buf).unwrap();
    zip_writer.finish().unwrap();

    let mut dict = SqliteDictionary::in_memory().unwrap();
    let report = dict
        .import_path_with_name(temp_zip.path(), Some("Grape LDB"))
        .expect("import ldb");
    assert_eq!(report.inserted, 1);

    let res = dict
        .lookup_exact(&LookupQuery::new("grape").unwrap())
        .unwrap();
    assert_eq!(res.len(), 1);
    assert!(res[0].definition().contains("berry growing in clusters"));
}

#[test]
fn test_ldp_rejects_path_traversal() {
    use zip::write::SimpleFileOptions;

    let temp_zip = NamedTempFile::with_suffix(".ldp").unwrap();
    let mut zip_writer = zip::ZipWriter::new(std::fs::File::create(temp_zip.path()).unwrap());
    // Try to write entry with directory traversal
    let _ = zip_writer.start_file("../evil.ld2", SimpleFileOptions::default());
    let _ = zip_writer.write_all(b"not an ld2");
    let _ = zip_writer.finish();

    let mut dict = SqliteDictionary::in_memory().unwrap();
    assert!(dict.import_path(temp_zip.path()).is_err());
}
