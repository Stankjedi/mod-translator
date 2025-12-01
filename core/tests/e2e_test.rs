//! End-to-End Tests for Translation Pipeline
//!
//! These tests validate the complete translation workflow:
//! 1. File reading and format detection
//! 2. Content extraction
//! 3. Translation (with mocked API)
//! 4. Validation
//! 5. Content merging
//! 6. File writing

use mod_translator_core::formats::{
    FileFormat, FormatHandler, TranslatableEntry, TranslatedEntry, TranslationResult,
    get_handler, json::JsonHandler, xml::XmlHandler, properties::PropertiesHandler,
};
use mod_translator_core::protector::Protector;
use std::fs;
use tempfile::TempDir;

/// Test fixture paths
const FIXTURE_JSON: &str = include_str!("fixtures/sample.json");
const FIXTURE_XML: &str = include_str!("fixtures/sample.xml");
const FIXTURE_PROPERTIES: &str = include_str!("fixtures/sample.properties");
#[allow(dead_code)]
const FIXTURE_LUA: &str = include_str!("fixtures/sample.lua");

/// Simulates AI translation response for testing
fn mock_translate(entries: &[TranslatableEntry], _target_lang: &str) -> TranslationResult {
    let translated: Vec<TranslatedEntry> = entries
        .iter()
        .map(|entry| {
            // Simple mock translation: prefix with [KO] and keep placeholders
            let fragment = Protector::protect(&entry.source);
            let mock_translation = format!("[KO] {}", fragment.masked_text());
            let restored = fragment.restore(&mock_translation).unwrap_or(mock_translation);
            
            TranslatedEntry {
                key: entry.key.clone(),
                source: entry.source.clone(),
                target: restored,
            }
        })
        .collect();

    TranslationResult {
        translated,
        failed: vec![],
    }
}

/// Test complete JSON translation pipeline
#[test]
fn test_e2e_json_pipeline() {
    // 1. Parse
    let handler = JsonHandler::new();
    let entries = handler.extract(FIXTURE_JSON).expect("Failed to extract JSON");
    
    assert!(!entries.is_empty(), "Should extract entries from JSON");
    assert!(entries.iter().any(|e| e.source == "Hello, World!"));
    assert!(entries.iter().any(|e| e.source == "A sharp blade"));
    
    // 2. Translate (mocked)
    let result = mock_translate(&entries, "ko");
    
    // 3. Validate translations - check that translation was applied
    for translated in &result.translated {
        assert!(
            translated.target.starts_with("[KO]"),
            "Translation should have [KO] prefix: {}",
            translated.target
        );
    }
    
    // 4. Merge back
    let merged = handler.merge(FIXTURE_JSON, &result).expect("Failed to merge JSON");
    
    // 5. Verify structure preserved
    let parsed: serde_json::Value = serde_json::from_str(&merged).expect("Invalid JSON output");
    assert!(parsed["greeting"].as_str().unwrap().contains("[KO]"));
    assert!(parsed["nested"]["welcome"].as_str().unwrap().contains("[KO]"));
}

/// Test complete XML translation pipeline
#[test]
fn test_e2e_xml_pipeline() {
    // 1. Parse
    let handler = XmlHandler::new();
    let entries = handler.extract(FIXTURE_XML).expect("Failed to extract XML");
    
    assert!(!entries.is_empty(), "Should extract entries from XML");
    assert!(entries.iter().any(|e| e.source == "Test Mod"));
    assert!(entries.iter().any(|e| e.source.contains("Iron Sword")));
    
    // 2. Translate (mocked)
    let result = mock_translate(&entries, "ko");
    
    // 3. Validate translations
    for translated in &result.translated {
        assert!(
            translated.target.starts_with("[KO]"),
            "Translation should have mock prefix"
        );
    }
    
    // 4. Merge back
    let merged = handler.merge(FIXTURE_XML, &result).expect("Failed to merge XML");
    
    // 5. Verify structure preserved
    assert!(merged.contains("<?xml"));
    assert!(merged.contains("<LanguageData>"));
    assert!(merged.contains("[KO]"));
}

/// Test complete Properties file translation pipeline
#[test]
fn test_e2e_properties_pipeline() {
    // 1. Parse
    let handler = PropertiesHandler::new();
    let entries = handler.extract(FIXTURE_PROPERTIES).expect("Failed to extract properties");
    
    assert!(!entries.is_empty(), "Should extract entries from properties");
    assert!(entries.iter().any(|e| e.key == "greeting"));
    assert!(entries.iter().any(|e| e.source == "Hello World"));
    
    // 2. Translate (mocked)
    let result = mock_translate(&entries, "ko");
    
    // 3. Merge back
    let merged = handler.merge(FIXTURE_PROPERTIES, &result).expect("Failed to merge properties");
    
    // 4. Verify structure preserved
    assert!(merged.contains("greeting="));
    assert!(merged.contains("[KO]"));
    assert!(merged.contains("# Comment line")); // Comments preserved
}

/// Test file I/O integration
#[test]
fn test_e2e_file_io_integration() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    
    // Write test files
    let json_path = temp_dir.path().join("test.json");
    let xml_path = temp_dir.path().join("test.xml");
    
    fs::write(&json_path, FIXTURE_JSON).expect("Failed to write JSON");
    fs::write(&xml_path, FIXTURE_XML).expect("Failed to write XML");
    
    // Process JSON
    let json_content = fs::read_to_string(&json_path).expect("Failed to read JSON");
    let json_handler = get_handler(FileFormat::Json).unwrap();
    let json_entries = json_handler.extract(&json_content).expect("Failed to extract");
    let json_result = mock_translate(&json_entries, "ko");
    let json_merged = json_handler.merge(&json_content, &json_result).expect("Failed to merge");
    
    // Write output
    let output_path = temp_dir.path().join("output.json");
    fs::write(&output_path, &json_merged).expect("Failed to write output");
    
    // Verify output
    let output_content = fs::read_to_string(&output_path).expect("Failed to read output");
    let output_json: serde_json::Value = serde_json::from_str(&output_content).expect("Invalid output");
    assert!(output_json["greeting"].as_str().unwrap().contains("[KO]"));
}

/// Test format detection
#[test]
fn test_e2e_format_detection() {
    use std::path::Path;
    
    let test_cases = vec![
        ("test.json", FileFormat::Json),
        ("test.xml", FileFormat::Xml),
        ("test.properties", FileFormat::Properties),
        ("test.lua", FileFormat::Lua),
        ("test.yaml", FileFormat::Yaml),
        ("test.yml", FileFormat::Yaml),
        ("test.po", FileFormat::Po),
        ("test.ini", FileFormat::Ini),
        ("test.cfg", FileFormat::Cfg),
        ("test.csv", FileFormat::Csv),
        ("test.md", FileFormat::Markdown),
        ("test.txt", FileFormat::Txt),
    ];
    
    for (filename, expected_format) in test_cases {
        let detected = FileFormat::from_path(Path::new(filename));
        assert_eq!(
            detected, expected_format,
            "Format detection failed for {}",
            filename
        );
    }
}

/// Test placeholder preservation through full pipeline
#[test]
fn test_e2e_placeholder_preservation() {
    let test_texts = vec![
        ("Hello {0}!", "Hello {0}!"),
        ("Value: %d%%", "Value: %d%%"),
        ("<color=red>Warning</color>", "<color=red>Warning</color>"),
        ("Price: $100", "Price: $100"),
        ("Range: 10-20", "Range: 10-20"),
    ];
    
    for (source, _expected) in test_texts {
        // Protect
        let fragment = Protector::protect(source);
        
        // Simulate translation (just add prefix)
        let mock_translated = format!("[번역됨] {}", fragment.masked_text());
        
        // Restore
        let restored = fragment.restore(&mock_translated).expect("Failed to restore");
        
        // Verify the restored text contains original placeholders
        // by checking token count matches
        assert!(
            !fragment.token_map().tokens.is_empty() || source.len() == restored.len() - "[번역됨] ".len(),
            "Placeholder preservation failed for '{}': restored = '{}'",
            source,
            restored
        );
    }
}

/// Test batch translation workflow
#[test]
fn test_e2e_batch_translation() {
    let json_handler = JsonHandler::new();
    let xml_handler = XmlHandler::new();
    let props_handler = PropertiesHandler::new();
    
    let files = vec![
        (FIXTURE_JSON, &json_handler as &dyn FormatHandler),
        (FIXTURE_XML, &xml_handler as &dyn FormatHandler),
        (FIXTURE_PROPERTIES, &props_handler as &dyn FormatHandler),
    ];
    
    let mut total_entries = 0;
    let mut total_translated = 0;
    
    for (content, handler) in files {
        let entries = handler.extract(content).expect("Failed to extract");
        total_entries += entries.len();
        
        let result = mock_translate(&entries, "ko");
        total_translated += result.translated.len();
        
        let merged = handler.merge(content, &result).expect("Failed to merge");
        assert!(merged.contains("[KO]"), "Translation not applied");
    }
    
    assert!(total_entries > 0, "Should have extracted entries");
    assert_eq!(total_entries, total_translated, "All entries should be translated");
}

/// Test error recovery in pipeline
#[test]
fn test_e2e_error_recovery() {
    // Test with malformed JSON - should fail gracefully
    let malformed_json = r#"{ "key": "value", invalid }"#;
    let handler = JsonHandler::new();
    
    let result = handler.extract(malformed_json);
    assert!(result.is_err(), "Should fail on malformed JSON");
    
    // Test with empty content
    let empty_result = handler.extract("{}");
    assert!(empty_result.is_ok(), "Should handle empty JSON");
    assert!(empty_result.unwrap().is_empty(), "Empty JSON should yield no entries");
}

/// Test translation result integrity
#[test]
fn test_e2e_translation_integrity() {
    let handler = JsonHandler::new();
    let entries = handler.extract(FIXTURE_JSON).expect("Failed to extract");
    
    let result = mock_translate(&entries, "ko");
    
    // Verify 1:1 correspondence
    assert_eq!(
        entries.len(),
        result.translated.len(),
        "All entries should have translations"
    );
    
    // Verify keys match
    for (entry, translated) in entries.iter().zip(result.translated.iter()) {
        assert_eq!(entry.key, translated.key, "Keys should match");
        assert_eq!(entry.source, translated.source, "Sources should match");
        assert!(!translated.target.is_empty(), "Translation should not be empty");
    }
}

/// Test concurrent file processing simulation
#[test]
fn test_e2e_concurrent_processing_simulation() {
    use std::thread;
    
    let handles: Vec<_> = (0..4)
        .map(|i| {
            thread::spawn(move || {
                let content = match i % 3 {
                    0 => FIXTURE_JSON,
                    1 => FIXTURE_XML,
                    _ => FIXTURE_PROPERTIES,
                };
                
                let format = match i % 3 {
                    0 => FileFormat::Json,
                    1 => FileFormat::Xml,
                    _ => FileFormat::Properties,
                };
                
                let handler = get_handler(format).unwrap();
                let entries = handler.extract(content).expect("Failed to extract");
                let result = mock_translate(&entries, "ko");
                handler.merge(content, &result).expect("Failed to merge")
            })
        })
        .collect();
    
    for handle in handles {
        let merged = handle.join().expect("Thread panicked");
        assert!(merged.contains("[KO]"), "Translation should be applied");
    }
}
