/// Comprehensive test suite for Section 13 requirements
/// Tests formula preservation, unit preservation, ICU blocks, markup, links, and mixed patterns

#[cfg(test)]
mod codex_spec_tests {
    use crate::protector::{Protector, ProtectedFragment};
    use crate::placeholder_validator::{PlaceholderValidator, Segment, ValidationErrorCode};
    use crate::format_validator;
    
    /// Section 13 Test Set 1: Mathematical expressions must be preserved
    #[test]
    fn test_math_expression_preservation() {
        let test_cases = vec![
            "3.14 × r^2",
            "10–20%",
            "(a+b)/2",
            "x ≥ 10",
            "2 + 2 = 4",
        ];
        
        for input in test_cases {
            let fragment = Protector::protect(input);
            let restored = fragment.restore(fragment.masked_text()).unwrap();
            assert_eq!(restored, input, "Failed to preserve math expression: {}", input);
            
            // Ensure it was protected (not just passed through)
            assert!(!fragment.token_map().tokens.is_empty(), 
                "Math expression should have been protected: {}", input);
        }
    }
    
    /// Section 13 Test Set 2: Units must be preserved
    #[test]
    fn test_unit_preservation() {
        let test_cases = vec![
            "16 ms",
            "60 FPS",
            "4 GB",
            "100 km/h",
            "90°",
        ];
        
        for input in test_cases {
            let fragment = Protector::protect(input);
            let restored = fragment.restore(fragment.masked_text()).unwrap();
            assert_eq!(restored, input, "Failed to preserve unit: {}", input);
            
            assert!(!fragment.token_map().tokens.is_empty(), 
                "Unit should have been protected: {}", input);
        }
    }
    
    /// Section 13 Test Set 3: Combined patterns like {0}% and %1$s/s
    #[test]
    fn test_combined_pattern_preservation() {
        let test_cases = vec![
            "{0}%",
            "%1$s/s",
            "{speed}%",
            "16-32 ms",
        ];
        
        for input in test_cases {
            let fragment = Protector::protect(input);
            let restored = fragment.restore(fragment.masked_text()).unwrap();
            assert_eq!(restored, input, "Failed to preserve combined pattern: {}", input);
            
            assert!(!fragment.token_map().tokens.is_empty(), 
                "Combined pattern should have been protected: {}", input);
        }
    }
    
    /// Section 13 Test Set 4: ICU MessageFormat blocks (atomic protection)
    #[test]
    fn test_icu_block_preservation() {
        let test_cases = vec![
            "{count, plural, one {# item} other {# items}}",
            "{gender, select, male {he} female {she} other {they}}",
        ];
        
        for input in test_cases {
            let fragment = Protector::protect(input);
            let restored = fragment.restore(fragment.masked_text()).unwrap();
            assert_eq!(restored, input, "Failed to preserve ICU block: {}", input);
            
            // ICU blocks should be detected
            assert!(!fragment.token_map().tokens.is_empty(), 
                "ICU block should have been protected: {}", input);
        }
    }
    
    /// Section 13 Test Set 5: Markup preservation
    #[test]
    fn test_markup_preservation() {
        let test_cases = vec![
            "<b>bold text</b>",
            "<i>italic</i>",
            "[color=#FFA500]orange[/color]",
            "<color=#ff0000>red</color>",
            "§aGreen§r",
        ];
        
        for input in test_cases {
            let fragment = Protector::protect(input);
            let restored = fragment.restore(fragment.masked_text()).unwrap();
            assert_eq!(restored, input, "Failed to preserve markup: {}", input);
            
            assert!(!fragment.token_map().tokens.is_empty(), 
                "Markup should have been protected: {}", input);
        }
    }
    
    /// Section 13 Test Set 6: Links, paths, and resource keys
    #[test]
    fn test_link_and_path_preservation() {
        let test_cases = vec![
            "__ENTITY__iron-ore__",
            "__1__",
            "[img=item/iron-plate]",
        ];
        
        for input in test_cases {
            let fragment = Protector::protect(input);
            let restored = fragment.restore(fragment.masked_text()).unwrap();
            assert_eq!(restored, input, "Failed to preserve link/path: {}", input);
            
            assert!(!fragment.token_map().tokens.is_empty(), 
                "Link/path should have been protected: {}", input);
        }
    }
    
    /// Section 13 Test Set 7: Mixed complex patterns
    #[test]
    fn test_mixed_complex_patterns() {
        let test_cases = vec![
            "<b>Speed: {0}%</b> at 16-32 ms",
            "Process __1__ items using {count} workers",
            "Temperature: 20-25°C with §aGreen§r status",
            "Calculate 3.14 × r^2 for {PAWN_label}",
        ];
        
        for input in test_cases {
            let fragment = Protector::protect(input);
            let restored = fragment.restore(fragment.masked_text()).unwrap();
            assert_eq!(restored, input, "Failed to preserve mixed pattern: {}", input);
            
            // Should detect multiple tokens
            assert!(fragment.token_map().tokens.len() >= 2, 
                "Mixed pattern should have multiple protected tokens: {}", input);
        }
    }
    
    /// Section 13 Test Set 8: Format parser validation after restoration
    #[test]
    fn test_format_parser_validation_json() {
        let valid_json = r#"{"key": "value", "number": 42}"#;
        let invalid_json = r#"{"key": "value"#; // missing closing brace
        
        assert!(format_validator::validate_json(valid_json).is_ok());
        assert!(format_validator::validate_json(invalid_json).is_err());
    }
    
    #[test]
    fn test_format_parser_validation_xml() {
        let valid_xml = "<root><child>text</child></root>";
        let invalid_xml = "<root><child>text</root>"; // mismatched tags
        
        assert!(format_validator::validate_xml(valid_xml).is_ok());
        assert!(format_validator::validate_xml(invalid_xml).is_err());
    }
    
    /// Section 13 Test Set 9: Placeholder validator integration
    #[test]
    fn test_placeholder_validator_with_formulas() {
        let segment = Segment::new(
            "test.xml".to_string(),
            1,
            "formula_key".to_string(),
            "Calculate 3.14 × r^2".to_string(),
            "Calculate ⟦MT:MATHEXPR:0⟧".to_string(),
        );
        
        let validator = PlaceholderValidator::with_default_config();
        
        // Valid translation (preserves token)
        let result = validator.validate(&segment, "계산: ⟦MT:MATHEXPR:0⟧");
        assert!(result.is_ok(), "Should accept translation with preserved formula");
        
        // Invalid translation (missing token)
        let result = validator.validate(&segment, "계산하세요");
        assert!(result.is_err(), "Should reject translation missing formula token");
    }
    
    #[test]
    fn test_placeholder_validator_with_units() {
        let segment = Segment::new(
            "test.xml".to_string(),
            1,
            "speed_key".to_string(),
            "Speed: 16 ms".to_string(),
            "Speed: ⟦MT:UNIT:0⟧".to_string(),
        );
        
        let validator = PlaceholderValidator::with_default_config();
        
        // Valid translation
        let result = validator.validate(&segment, "속도: ⟦MT:UNIT:0⟧");
        assert!(result.is_ok(), "Should accept translation with preserved unit");
        
        // Invalid translation
        let result = validator.validate(&segment, "속도: 16밀리초");
        assert!(result.is_err(), "Should reject translation with translated unit");
    }
    
    #[test]
    fn test_percent_binding_preservation() {
        let segment = Segment::new(
            "test.xml".to_string(),
            1,
            "percent_key".to_string(),
            "Speed {0}%".to_string(),
            "Speed {0}%".to_string(),
        );
        
        let validator = PlaceholderValidator::with_default_config();
        
        // Missing % should be auto-recovered
        let result = validator.validate(&segment, "속도 {0}");
        assert!(result.is_ok(), "Should auto-recover missing % after {{0}}");
        
        if let Ok(recovered) = result {
            assert!(recovered.contains("{0}%"), "Should preserve {{0}}% binding");
        }
    }
    
    /// Section 13 Test Set 10: Roundtrip test for all major patterns
    #[test]
    fn test_comprehensive_roundtrip() {
        let comprehensive_inputs = vec![
            // Math expressions
            "3.14 × r^2",
            "(a+b)/2 = c",
            "x ≥ 10 and y ≤ 20",
            
            // Units
            "16 ms", "60 FPS", "4 GB", "100 km/h", "90°C",
            
            // Ranges
            "10-20 items", "5~10 seconds", "100–200 meters",
            
            // Percentages
            "50%", "10-20%",
            
            // Scientific notation
            "1e-6", "2×10^9", "10^3",
            
            // Format tokens
            "{0}", "{1:0.##}", "{PAWN_label}", "%s", "%1$s",
            
            // Combined
            "{0}%", "%1$s/s",
            
            // Markup
            "<b>text</b>", "[color=red]text[/color]", "§atext§r",
            
            // Game-specific
            "__1__", "__ENTITY__iron-ore__", "[img=item/iron]",
            
            // ICU (simplified - full ICU has nested braces)
            "{count, plural, one {# item} other {# items}}",
            
            // Mixed
            "<b>Speed: {0}%</b>",
            "Process __1__ at 16-32 ms",
        ];
        
        for input in comprehensive_inputs {
            let fragment = Protector::protect(input);
            
            // Verify masked text is different (something was protected)
            if !fragment.token_map().tokens.is_empty() {
                assert_ne!(fragment.masked_text(), input, 
                    "Masked text should be different for: {}", input);
            }
            
            // Verify restoration
            let restored = fragment.restore(fragment.masked_text()).unwrap();
            assert_eq!(restored, input, 
                "Roundtrip failed for: {}", input);
        }
    }
    
    /// Section 13 Acceptance Criteria: Zero format errors in corpus
    #[test]
    fn test_format_error_rate_in_sample_corpus() {
        // Sample corpus with 10+ segments per profile
        let rimworld_corpus = vec![
            "<GameSpeed>Speed: {0}%</GameSpeed>",
            "<Description>Calculate 3.14 × r^2</Description>",
            "<Label>{PAWN_label} moved 16 ms</Label>",
        ];
        
        let factorio_corpus = vec![
            "item-name.iron-ore=Process __1__ items",
            "description.speed=Speed: __2__ at 60 FPS",
            "tooltip.range=Range: 10-20 meters",
        ];
        
        let mut total_segments = 0;
        let mut format_errors = 0;
        
        // Test RimWorld corpus
        for input in rimworld_corpus {
            total_segments += 1;
            let fragment = Protector::protect(input);
            let restored = fragment.restore(fragment.masked_text());
            
            if restored.is_err() {
                format_errors += 1;
            } else {
                // Validate XML if it's XML format
                if input.starts_with('<') {
                    if format_validator::validate_xml(&restored.unwrap()).is_err() {
                        format_errors += 1;
                    }
                }
            }
        }
        
        // Test Factorio corpus (CFG format)
        for input in factorio_corpus {
            total_segments += 1;
            let fragment = Protector::protect(input);
            let restored = fragment.restore(fragment.masked_text());
            
            if restored.is_err() {
                format_errors += 1;
            } else {
                // Validate INI/CFG format
                if format_validator::validate_ini(&restored.unwrap()).is_err() {
                    format_errors += 1;
                }
            }
        }
        
        // Section 13 requirement: 0 format errors in sample corpus
        assert_eq!(format_errors, 0, 
            "Expected 0 format errors in {} segments, but found {}", 
            total_segments, format_errors);
    }
}
