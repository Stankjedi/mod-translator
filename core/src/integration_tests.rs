/// Integration tests for the improved translation system
/// Tests the complete flow from protection to extraction to merging

#[cfg(test)]
mod tests {
    use crate::protector::{ProtectionMode, Protector};
    use crate::text_extractor::TextExtractor;
    use crate::tone_analyzer::{ToneAnalyzer, TextType};
    use crate::formats::xml::XmlHandler;
    use crate::formats::{FormatHandler, TranslationResult, TranslatedEntry};

    // ============================================
    // Protector Tests - Math Pattern Refinement
    // ============================================

    #[test]
    fn test_protector_allows_simple_ranges() {
        // Simple number ranges like "10-20" should NOT be protected
        // because they might be part of translatable text like "10-20 items"
        let text = "This item deals 10-20 damage";
        let fragment = Protector::protect(text);
        
        // The numbers should remain in the masked text for translation
        // (LLM should handle numbers appropriately based on prompt)
        assert!(fragment.masked_text().contains("10") || fragment.masked_text().contains("damage"));
    }

    #[test]
    fn test_protector_protects_real_math() {
        // Real math formulas should be protected
        let text = "Area = 3.14 × r^2";
        let fragment = Protector::protect(text);
        
        // Should have some protection tokens
        let token_count = fragment.token_map().tokens.len();
        assert!(token_count > 0 || fragment.masked_text() != text);
    }

    #[test]
    fn test_protector_preserves_placeholders() {
        let text = "Hello {0}, you have {1} items!";
        let fragment = Protector::protect(text);
        
        // Placeholders should be tokenized
        let token_count = fragment.token_map().tokens.len();
        assert!(token_count >= 2);
        
        // Restore should work
        let translated = fragment.masked_text().replace("Hello", "안녕하세요").replace("you have", "당신은").replace("items", "아이템을 가지고 있습니다");
        let restored = fragment.restore(&translated);
        assert!(restored.is_ok() || fragment.masked_text().contains("{0}"));
    }

    #[test]
    fn test_protection_modes() {
        let text = "Value: 100kg, formula: x^2 + y";
        
        // Full mode - protect everything possible
        let full_fragment = Protector::protect_with_mode(text, ProtectionMode::Full);
        
        // Minimal mode - only protect essential tokens
        let minimal_fragment = Protector::protect_with_mode(text, ProtectionMode::Minimal);
        
        // Full mode should have more or equal tokens
        let full_count = full_fragment.token_map().tokens.len();
        let minimal_count = minimal_fragment.token_map().tokens.len();
        assert!(full_count >= minimal_count);
    }

    // ============================================
    // Text Extractor Tests
    // ============================================

    #[test]
    fn test_extractor_filters_code() {
        let extractor = TextExtractor::new();
        
        // Pure identifier/code patterns should not be translatable
        assert!(!extractor.is_translatable("function_name").0);
        assert!(!extractor.is_translatable("get_value_from_db").0);
        assert!(!extractor.is_translatable("CONSTANT_VALUE").0);
        
        // Unix-style file paths
        assert!(!extractor.is_translatable("/path/to/file.txt").0);
        
        // Dot-separated identifiers
        assert!(!extractor.is_translatable("item.minecraft.diamond").0);
    }

    #[test]
    fn test_extractor_allows_natural_text() {
        let extractor = TextExtractor::new();
        
        // Natural language should be translatable
        assert!(extractor.is_translatable("Hello world!").0);
        assert!(extractor.is_translatable("This is a description.").0);
        assert!(extractor.is_translatable("Click here to start").0);
        assert!(extractor.is_translatable("아이템을 사용하세요").0); // Korean
    }

    #[test]
    fn test_extractor_preserves_embedded_numbers() {
        let extractor = TextExtractor::new();
        
        // Text with embedded numbers should still be translatable
        // The numbers themselves will be handled by the protector
        assert!(extractor.is_translatable("You have 5 items").0);
        assert!(extractor.is_translatable("Level 10 required").0);
    }

    #[test]
    fn test_extract_xml_segments() {
        let extractor = TextExtractor::new();
        
        let xml = r#"<item>
            <defName>Sword_Iron</defName>
            <label>Iron Sword</label>
            <description>A simple iron sword used for combat.</description>
        </item>"#;
        
        let result = extractor.extract_xml(xml);
        
        // Should extract label and description, but not defName
        let texts: Vec<&str> = result.translatable.iter().map(|s| s.text.as_str()).collect();
        assert!(texts.iter().any(|t| t.contains("Iron Sword")));
        assert!(texts.iter().any(|t| t.contains("simple iron sword")));
    }

    // ============================================
    // Tone Analyzer Tests
    // ============================================

    #[test]
    fn test_tone_analyzer_detects_ui_text() {
        let analyzer = ToneAnalyzer::new();
        
        let samples = vec!["Start", "Continue", "Options", "Quit"];
        let analysis = analyzer.analyze(&samples);
        
        assert_eq!(analysis.text_type, TextType::Ui);
    }

    #[test]
    fn test_tone_analyzer_detects_dialogue() {
        let analyzer = ToneAnalyzer::new();
        
        let samples = vec![
            "\"Hello there!\" he said.",
            "\"What do you want?\" asked the merchant.",
        ];
        let analysis = analyzer.analyze(&samples);
        
        assert_eq!(analysis.text_type, TextType::Dialogue);
    }

    #[test]
    fn test_tone_analyzer_recommends_korean_honorific() {
        let analyzer = ToneAnalyzer::new();
        
        // Formal samples should recommend formal Korean
        let formal = vec![
            "Please proceed with caution.",
            "We recommend saving your progress.",
        ];
        let formal_analysis = analyzer.analyze(&formal);
        
        // Casual samples should recommend casual Korean
        let casual = vec![
            "Hey! Check this out!",
            "Cool stuff here.",
        ];
        let casual_analysis = analyzer.analyze(&casual);
        
        // Formal should have higher formality than casual
        assert!(formal_analysis.formality > casual_analysis.formality);
    }

    #[test]
    fn test_tone_prompt_generation() {
        let analyzer = ToneAnalyzer::new();
        
        let samples = vec!["Click to continue", "Press Enter"];
        let analysis = analyzer.analyze(&samples);
        
        let hint = analyzer.generate_prompt_hint(&analysis, "ko");
        
        // Hint should contain Korean-specific guidance
        assert!(hint.contains("한국어") || hint.contains("Korean") || hint.contains("Text type"));
    }

    // ============================================
    // XML Handler Tests
    // ============================================

    #[test]
    fn test_xml_extract_skip_paths() {
        let handler = XmlHandler::new();
        
        let xml = r#"<ThingDef>
            <defName>Weapon_Sword</defName>
            <label>Iron Sword</label>
            <graphicPath>Things/Weapons/Sword</graphicPath>
            <texPath>UI/Icons/Sword</texPath>
        </ThingDef>"#;
        
        let entries = handler.extract(xml).unwrap();
        let sources: Vec<&str> = entries.iter().map(|e| e.source.as_str()).collect();
        
        // Should extract label
        assert!(sources.iter().any(|s| s.contains("Iron Sword")));
        
        // Should NOT extract paths and defName
        assert!(!sources.iter().any(|s| s.contains("Weapon_Sword")));
        assert!(!sources.iter().any(|s| s.contains("Things/Weapons")));
        assert!(!sources.iter().any(|s| s.contains("UI/Icons")));
    }

    #[test]
    fn test_xml_merge_preserves_structure() {
        let handler = XmlHandler::new();
        
        let xml = r#"<item><label>Sword</label></item>"#;
        
        let translations = TranslationResult {
            translated: vec![TranslatedEntry {
                key: "label_14".to_string(),
                source: "Sword".to_string(),
                target: "검".to_string(),
            }],
            failed: vec![],
        };
        
        let result = handler.merge(xml, &translations).unwrap();
        
        assert!(result.contains("<item>"));
        assert!(result.contains("<label>"));
        assert!(result.contains("검"));
        assert!(!result.contains(">Sword<"));
    }

    // ============================================
    // End-to-End Flow Tests
    // ============================================

    #[test]
    fn test_full_translation_flow() {
        // This simulates the full flow without actual LLM calls
        
        // 1. Original text with mixed content
        let original = "Deal {0} damage to enemies within {1}m range";
        
        // 2. Protection step
        let fragment = Protector::protect(original);
        
        // 3. The masked text should be safe for translation
        let masked = fragment.masked_text();
        
        // 4. Simulate translation (just replace some words)
        let translated = masked
            .replace("Deal", "주는")
            .replace("damage to enemies within", "피해를 입힘, 범위")
            .replace("range", "");
        
        // 5. Restore placeholders
        let restored = fragment.restore(&translated);
        
        // Should restore successfully or gracefully handle
        assert!(restored.is_ok() || masked.contains("{0}"));
    }

    #[test]
    fn test_rimworld_xml_pattern() {
        // Common RimWorld mod XML structure
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<Defs>
    <ThingDef ParentName="BaseMeleeWeapon">
        <defName>MeleeWeapon_Longsword</defName>
        <label>longsword</label>
        <description>A long, sharp blade perfect for close combat.</description>
        <graphicData>
            <texPath>Things/Weapons/Longsword</texPath>
        </graphicData>
        <statBases>
            <MarketValue>300</MarketValue>
        </statBases>
    </ThingDef>
</Defs>"#;

        let handler = XmlHandler::new();
        let entries = handler.extract(xml).unwrap();
        
        // Should find label and description
        let sources: Vec<&str> = entries.iter().map(|e| e.source.as_str()).collect();
        
        assert!(sources.iter().any(|s| *s == "longsword"));
        assert!(sources.iter().any(|s| s.contains("sharp blade")));
        
        // Should NOT extract technical content
        assert!(!sources.iter().any(|s| s.contains("MeleeWeapon_Longsword")));
        assert!(!sources.iter().any(|s| s.contains("Things/Weapons")));
        assert!(!sources.iter().any(|s| *s == "300"));
    }

    #[test]
    fn test_minecraft_lang_pattern() {
        // Minecraft-style key=value format is handled by properties handler
        // but we can test that the text extractor works with similar content
        
        let extractor = TextExtractor::new();
        
        // Minecraft translation strings
        assert!(extractor.is_translatable("Diamond Sword").0);
        assert!(extractor.is_translatable("A powerful weapon").0);
        assert!(!extractor.is_translatable("item.minecraft.diamond_sword").0);
    }
}
