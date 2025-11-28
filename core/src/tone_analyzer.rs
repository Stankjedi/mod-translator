/// Tone Analyzer Module
/// 
/// Analyzes the tone, style, and formality of text to provide hints for translation.
/// This helps maintain consistent translation style across a mod.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents the analyzed tone of a text corpus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToneAnalysis {
    /// Formality level (0.0 = very casual, 1.0 = very formal)
    pub formality: f32,
    /// Politeness level (0.0 = direct/blunt, 1.0 = very polite)
    pub politeness: f32,
    /// Energy/enthusiasm level (0.0 = calm/neutral, 1.0 = very energetic)
    pub energy: f32,
    /// Detected text type
    pub text_type: TextType,
    /// Recommended target language style
    pub recommended_style: TranslationStyle,
    /// Key terminology that should be preserved/consistently translated
    pub key_terms: Vec<TermEntry>,
    /// Sample sentences for style reference
    pub style_samples: Vec<String>,
    /// Confidence score for this analysis (0.0 - 1.0)
    pub confidence: f32,
}

/// Type of text being analyzed
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TextType {
    /// User interface elements (buttons, menus, labels)
    Ui,
    /// Narrative/story text
    Narrative,
    /// Character dialogue
    Dialogue,
    /// Tutorial/instructional text
    Tutorial,
    /// Item/object descriptions
    Description,
    /// System messages/errors
    System,
    /// Mixed or unknown
    Mixed,
}

/// Recommended translation style
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranslationStyle {
    /// For Korean: 존댓말 level
    pub korean_honorific: KoreanHonorific,
    /// For Japanese: formality level
    pub japanese_formality: JapaneseFormality,
    /// General tone description
    pub tone_description: String,
    /// Whether to preserve original punctuation style
    pub preserve_punctuation: bool,
}

/// Korean honorific levels (존댓말)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum KoreanHonorific {
    /// 해체 (very casual, e.g., 해, 가)
    Haechae,
    /// 해라체 (plain form, e.g., 한다, 간다)
    Haerache,
    /// 해요체 (polite informal, e.g., 해요, 가요)
    #[default]
    Haeyoche,
    /// 합쇼체 (formal polite, e.g., 합니다, 갑니다)
    Hapsochu,
}

impl KoreanHonorific {
    pub fn description(&self) -> &'static str {
        match self {
            KoreanHonorific::Haechae => "해체 (아주 친근한 반말)",
            KoreanHonorific::Haerache => "해라체 (평서형 반말)",
            KoreanHonorific::Haeyoche => "해요체 (친근한 존댓말)",
            KoreanHonorific::Hapsochu => "합쇼체 (격식있는 존댓말)",
        }
    }
}

/// Japanese formality levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum JapaneseFormality {
    /// Casual (だ/である)
    Casual,
    /// Polite (です/ます)
    #[default]
    Polite,
    /// Very formal (ございます)
    Formal,
}

/// A key term that should be consistently translated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermEntry {
    /// Original term
    pub term: String,
    /// Frequency of occurrence
    pub frequency: usize,
    /// Suggested translation (if determinable from context)
    pub suggested_translation: Option<String>,
    /// Context examples
    pub contexts: Vec<String>,
}

// Patterns for detecting text characteristics
static QUESTION_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\?+").expect("valid regex")
});

static EXCLAMATION_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"!+").expect("valid regex")
});

#[allow(dead_code)]
static ELLIPSIS_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\.{2,}|…").expect("valid regex")
});

static DIALOGUE_MARKER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"^["'「『]|["'」』]$|:\s*$|said|says|asked|replied|shouted|whispered"#).expect("valid regex")
});

static UI_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(Start|Play|Continue|Options|Settings|Quit|Exit|Save|Load|New Game|Cancel|OK|Yes|No|Apply|Back|Next|Previous)$").expect("valid regex")
});

static INSTRUCTION_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^(click|press|select|choose|enter|type|use|go to|navigate|find|open|close|drag|drop)\b").expect("valid regex")
});

static FORMAL_INDICATOR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(please|kindly|would you|could you|shall|may I|if you would|we recommend|it is recommended)\b").expect("valid regex")
});

static CASUAL_INDICATOR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(hey|cool|awesome|gonna|wanna|gotta|yeah|yep|nope|stuff|things|kinda|sorta)\b").expect("valid regex")
});

/// Tone Analyzer - analyzes text to determine appropriate translation style
pub struct ToneAnalyzer {
    /// Minimum sample size for reliable analysis
    min_samples: usize,
    /// Whether to extract terminology
    extract_terms: bool,
}

impl Default for ToneAnalyzer {
    fn default() -> Self {
        Self {
            min_samples: 5,
            extract_terms: true,
        }
    }
}

impl ToneAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Analyze a collection of text samples
    pub fn analyze(&self, samples: &[&str]) -> ToneAnalysis {
        if samples.is_empty() {
            return self.default_analysis();
        }
        
        let mut formality_sum = 0.0f32;
        let mut politeness_sum = 0.0f32;
        let mut energy_sum = 0.0f32;
        let mut type_votes: HashMap<TextType, usize> = HashMap::new();
        let mut term_counts: HashMap<String, (usize, Vec<String>)> = HashMap::new();
        let mut style_samples: Vec<String> = Vec::new();
        
        for sample in samples {
            let sample_str = *sample;
            
            // Analyze formality
            let formal_count = FORMAL_INDICATOR.find_iter(sample_str).count();
            let casual_count = CASUAL_INDICATOR.find_iter(sample_str).count();
            let formality = if formal_count + casual_count > 0 {
                formal_count as f32 / (formal_count + casual_count) as f32
            } else {
                0.5 // neutral
            };
            formality_sum += formality;
            
            // Analyze politeness (based on sentence structure)
            let has_please = sample_str.to_lowercase().contains("please");
            let has_question = QUESTION_PATTERN.is_match(sample_str);
            politeness_sum += if has_please { 0.8 } else if has_question { 0.6 } else { 0.4 };
            
            // Analyze energy
            let exclamation_count = EXCLAMATION_PATTERN.find_iter(sample_str).count();
            let has_caps = sample_str.chars().filter(|c| c.is_alphabetic()).any(|c| c.is_uppercase());
            energy_sum += (exclamation_count as f32 * 0.2 + if has_caps { 0.1 } else { 0.0 }).min(1.0);
            
            // Detect text type
            let text_type = self.detect_text_type(sample_str);
            *type_votes.entry(text_type).or_insert(0) += 1;
            
            // Extract potential terminology
            if self.extract_terms {
                self.extract_terms_from_sample(sample_str, &mut term_counts);
            }
            
            // Collect representative samples
            if style_samples.len() < 5 && sample_str.len() > 10 && sample_str.len() < 200 {
                style_samples.push(sample_str.to_string());
            }
        }
        
        let sample_count = samples.len() as f32;
        let formality = formality_sum / sample_count;
        let politeness = politeness_sum / sample_count;
        let energy = energy_sum / sample_count;
        
        // Determine dominant text type
        let text_type = type_votes.into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(tt, _)| tt)
            .unwrap_or(TextType::Mixed);
        
        // Convert term counts to entries
        let key_terms: Vec<TermEntry> = term_counts.into_iter()
            .filter(|(_, (count, _))| *count >= 2) // At least 2 occurrences
            .map(|(term, (frequency, contexts))| TermEntry {
                term,
                frequency,
                suggested_translation: None,
                contexts: contexts.into_iter().take(3).collect(),
            })
            .collect();
        
        // Determine recommended style
        let recommended_style = self.recommend_style(formality, politeness, text_type);
        
        // Calculate confidence
        let confidence = if samples.len() >= self.min_samples {
            0.8 + (samples.len().min(50) as f32 / 250.0) // Up to 0.1 bonus
        } else {
            samples.len() as f32 / self.min_samples as f32 * 0.8
        };
        
        ToneAnalysis {
            formality,
            politeness,
            energy,
            text_type,
            recommended_style,
            key_terms,
            style_samples,
            confidence: confidence.min(1.0),
        }
    }
    
    /// Detect the type of text
    fn detect_text_type(&self, text: &str) -> TextType {
        let trimmed = text.trim();
        
        // UI elements (short, specific patterns)
        if trimmed.len() < 30 && UI_PATTERN.is_match(trimmed) {
            return TextType::Ui;
        }
        
        // Instructions/tutorials
        if INSTRUCTION_PATTERN.is_match(trimmed) {
            return TextType::Tutorial;
        }
        
        // Dialogue (quoted text or dialogue markers)
        if DIALOGUE_MARKER.is_match(trimmed) {
            return TextType::Dialogue;
        }
        
        // Descriptions (longer text with descriptive language)
        let word_count = trimmed.split_whitespace().count();
        if word_count > 15 && !trimmed.contains('!') {
            return TextType::Description;
        }
        
        // System messages (often contain specific keywords)
        if trimmed.to_lowercase().contains("error") || 
           trimmed.to_lowercase().contains("failed") ||
           trimmed.to_lowercase().contains("success") {
            return TextType::System;
        }
        
        // Short UI-like text
        if word_count <= 5 {
            return TextType::Ui;
        }
        
        TextType::Mixed
    }
    
    /// Extract potential terminology from a sample
    fn extract_terms_from_sample(
        &self,
        sample: &str,
        term_counts: &mut HashMap<String, (usize, Vec<String>)>,
    ) {
        // Find capitalized words that might be game-specific terms
        let words: Vec<&str> = sample.split_whitespace().collect();
        
        for word in &words {
            let cleaned: String = word.chars()
                .filter(|c| c.is_alphabetic())
                .collect();
            
            // Skip very short or very long terms
            if cleaned.len() < 3 || cleaned.len() > 30 {
                continue;
            }
            
            // Look for capitalized terms (likely proper nouns or game terms)
            let first_char = cleaned.chars().next();
            if let Some(c) = first_char {
                if c.is_uppercase() && cleaned.len() > 3 {
                    let entry = term_counts.entry(cleaned.clone()).or_insert((0, Vec::new()));
                    entry.0 += 1;
                    if entry.1.len() < 5 {
                        entry.1.push(sample.chars().take(100).collect());
                    }
                }
            }
        }
    }
    
    /// Recommend translation style based on analysis
    fn recommend_style(
        &self,
        formality: f32,
        politeness: f32,
        text_type: TextType,
    ) -> TranslationStyle {
        // Determine Korean honorific
        let korean_honorific = match (formality, politeness, text_type) {
            (f, _, TextType::Ui) if f < 0.3 => KoreanHonorific::Haechae,
            (f, _, _) if f > 0.7 => KoreanHonorific::Hapsochu,
            (_, p, _) if p > 0.6 => KoreanHonorific::Haeyoche,
            (f, _, _) if f < 0.4 => KoreanHonorific::Haerache,
            _ => KoreanHonorific::Haeyoche, // default
        };
        
        // Determine Japanese formality
        let japanese_formality = match formality {
            f if f > 0.7 => JapaneseFormality::Formal,
            f if f < 0.3 => JapaneseFormality::Casual,
            _ => JapaneseFormality::Polite,
        };
        
        // Generate tone description
        let tone_description = match text_type {
            TextType::Ui => "간결하고 명확한 UI 스타일".to_string(),
            TextType::Dialogue => "자연스러운 대화체".to_string(),
            TextType::Narrative => "서술적인 문체".to_string(),
            TextType::Tutorial => "친절하고 명확한 안내 문체".to_string(),
            TextType::Description => "상세한 설명 문체".to_string(),
            TextType::System => "기술적이고 간결한 시스템 메시지 스타일".to_string(),
            TextType::Mixed => "일반적인 번역 스타일".to_string(),
        };
        
        TranslationStyle {
            korean_honorific,
            japanese_formality,
            tone_description,
            preserve_punctuation: matches!(text_type, TextType::Dialogue | TextType::Narrative),
        }
    }
    
    /// Create default analysis when no samples available
    fn default_analysis(&self) -> ToneAnalysis {
        ToneAnalysis {
            formality: 0.5,
            politeness: 0.5,
            energy: 0.3,
            text_type: TextType::Mixed,
            recommended_style: TranslationStyle {
                korean_honorific: KoreanHonorific::Haeyoche,
                japanese_formality: JapaneseFormality::Polite,
                tone_description: "일반적인 번역 스타일".to_string(),
                preserve_punctuation: false,
            },
            key_terms: Vec::new(),
            style_samples: Vec::new(),
            confidence: 0.0,
        }
    }
    
    /// Generate a prompt hint based on tone analysis
    pub fn generate_prompt_hint(&self, analysis: &ToneAnalysis, target_lang: &str) -> String {
        let mut hints = Vec::new();
        
        // Language-specific hints
        match target_lang.to_lowercase().as_str() {
            "ko" | "korean" | "한국어" => {
                hints.push(format!(
                    "한국어 번역 시 {} 사용",
                    analysis.recommended_style.korean_honorific.description()
                ));
                
                if analysis.text_type == TextType::Ui {
                    hints.push("UI 텍스트는 간결하게 번역".to_string());
                }
            }
            "ja" | "japanese" | "日本語" => {
                let formality = match analysis.recommended_style.japanese_formality {
                    JapaneseFormality::Casual => "カジュアル (だ/である)",
                    JapaneseFormality::Polite => "丁寧 (です/ます)",
                    JapaneseFormality::Formal => "フォーマル (ございます)",
                };
                hints.push(format!("Japanese formality: {}", formality));
            }
            _ => {}
        }
        
        // General hints
        hints.push(format!("Text type: {:?}", analysis.text_type));
        hints.push(analysis.recommended_style.tone_description.clone());
        
        if !analysis.key_terms.is_empty() {
            let terms: Vec<_> = analysis.key_terms.iter()
                .take(5)
                .map(|t| t.term.as_str())
                .collect();
            hints.push(format!("Key terms to maintain consistency: {}", terms.join(", ")));
        }
        
        hints.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_detect_text_type_ui() {
        let analyzer = ToneAnalyzer::new();
        
        assert_eq!(analyzer.detect_text_type("Start"), TextType::Ui);
        assert_eq!(analyzer.detect_text_type("Continue"), TextType::Ui);
        assert_eq!(analyzer.detect_text_type("Options"), TextType::Ui);
    }
    
    #[test]
    fn test_detect_text_type_tutorial() {
        let analyzer = ToneAnalyzer::new();
        
        assert_eq!(
            analyzer.detect_text_type("Click the button to start"),
            TextType::Tutorial
        );
        assert_eq!(
            analyzer.detect_text_type("Press Enter to confirm"),
            TextType::Tutorial
        );
    }
    
    #[test]
    fn test_detect_text_type_dialogue() {
        let analyzer = ToneAnalyzer::new();
        
        assert_eq!(
            analyzer.detect_text_type("\"Hello there!\" he said."),
            TextType::Dialogue
        );
    }
    
    #[test]
    fn test_analyze_samples() {
        let analyzer = ToneAnalyzer::new();
        
        let formal_samples = vec![
            "Please proceed to the next section.",
            "Would you kindly confirm your selection?",
            "We recommend saving your progress before continuing.",
        ];
        
        let analysis = analyzer.analyze(&formal_samples);
        
        assert!(analysis.formality > 0.5);
        assert!(analysis.politeness > 0.5);
    }
    
    #[test]
    fn test_analyze_casual_samples() {
        let analyzer = ToneAnalyzer::new();
        
        let casual_samples = vec![
            "Hey! Check this out!",
            "Gonna need more stuff here.",
            "Cool! That's awesome!",
        ];
        
        let analysis = analyzer.analyze(&casual_samples);
        
        assert!(analysis.formality < 0.5);
        assert!(analysis.energy > 0.3);
    }
    
    #[test]
    fn test_korean_honorific_recommendation() {
        let analyzer = ToneAnalyzer::new();
        
        // Formal text should recommend formal Korean
        let formal_samples = vec![
            "We are pleased to inform you.",
            "Please proceed with caution.",
            "It is recommended to save frequently.",
        ];
        
        let analysis = analyzer.analyze(&formal_samples);
        assert!(matches!(
            analysis.recommended_style.korean_honorific,
            KoreanHonorific::Haeyoche | KoreanHonorific::Hapsochu
        ));
    }
    
    #[test]
    fn test_generate_prompt_hint() {
        let analyzer = ToneAnalyzer::new();
        
        let samples = vec!["Click to start", "Press Enter", "Select option"];
        let analysis = analyzer.analyze(&samples);
        
        let hint = analyzer.generate_prompt_hint(&analysis, "ko");
        
        assert!(hint.contains("한국어"));
    }
}
