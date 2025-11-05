/// Mathematical expressions, units, and numerical patterns protection
/// Implements Section 2.1 of the Codex specification:
/// - Arithmetic expressions with operators
/// - Ranges and intervals
/// - Percentages
/// - Units (ms, GB, km/h, etc.)
/// - Scientific notation

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

// === Section 2.1: Mathematical Expressions and Numbers ===

// Arithmetic expressions: numbers with operators + - × * ÷ / ^ = ≠ ≈ ≤ ≥ < >
// Examples: 3.14 × r^2, (a+b)/2, x ≥ 10
static MATH_EXPR_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?x)
        \d+(?:\.\d+)?  # number
        \s*[+\-×*÷/^=≠≈≤≥<>]\s*  # operator with optional whitespace
        \d+(?:\.\d+)?  # another number
        (?:\s*[+\-×*÷/^=≠≈≤≥<>]\s*\d+(?:\.\d+)?)*  # additional terms
        |
        \([^)]+[+\-×*÷/^=≠≈≤≥<>][^)]+\)  # expressions in parentheses
        "
    )
    .expect("valid math expression regex")
});

// Range/interval patterns: a~b, a-b, a–b (with en dash)
// Examples: 10-20, 5~10, 100–200, 16-32 ms
static RANGE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\d+(?:\.\d+)?\s*[~\-–]\s*\d+(?:\.\d+)?(?:\s*[a-zA-Z°%]+)?")
        .expect("valid range regex")
});

// Percentages: n%, {n}%, n‒m%
// Examples: 50%, {0}%, 10-20%
static PERCENT_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\d+(?:\.\d+)?%|\{\d+\}%|\d+\s*[‒\-–]\s*\d+%")
        .expect("valid percent regex")
});

// Scientific notation: 1e-6, 2×10^9, 10^n
static SCIENTIFIC_NOTATION_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\d+(?:\.\d+)?[eE][+\-]?\d+|\d+(?:\.\d+)?\s*[×x]\s*10\^[\d\-]+|10\^\d+|10\^[a-z]")
        .expect("valid scientific notation regex")
});

// Units with numbers: 16 ms, 60 FPS, 4 GB, 100 km/h, 90°
static UNIT_WITH_NUMBER_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\d+(?:\.\d+)?\s*(?:ms|fps|FPS|GB|MB|KB|TB|km/h|m/s|°C|°F|°|px|pt|em|rem|Hz|kHz|MHz)")
        .expect("valid unit with number regex")
});

/// Common units that should be preserved
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitDictionary {
    /// Time units
    pub time: HashSet<String>,
    /// Distance/length units
    pub distance: HashSet<String>,
    /// Data/storage units
    pub data: HashSet<String>,
    /// Temperature units
    pub temperature: HashSet<String>,
    /// Speed units
    pub speed: HashSet<String>,
    /// Display units
    pub display: HashSet<String>,
    /// Frequency units
    pub frequency: HashSet<String>,
    /// Other units
    pub other: HashSet<String>,
}

impl Default for UnitDictionary {
    fn default() -> Self {
        let mut time = HashSet::new();
        time.insert("ms".to_string());
        time.insert("s".to_string());
        time.insert("sec".to_string());
        time.insert("m".to_string());
        time.insert("min".to_string());
        time.insert("h".to_string());
        time.insert("hr".to_string());
        time.insert("d".to_string());
        time.insert("day".to_string());
        
        let mut distance = HashSet::new();
        distance.insert("mm".to_string());
        distance.insert("cm".to_string());
        distance.insert("m".to_string());
        distance.insert("km".to_string());
        distance.insert("in".to_string());
        distance.insert("ft".to_string());
        distance.insert("yd".to_string());
        distance.insert("mi".to_string());
        
        let mut data = HashSet::new();
        data.insert("B".to_string());
        data.insert("KB".to_string());
        data.insert("MB".to_string());
        data.insert("GB".to_string());
        data.insert("TB".to_string());
        data.insert("PB".to_string());
        
        let mut temperature = HashSet::new();
        temperature.insert("°C".to_string());
        temperature.insert("°F".to_string());
        temperature.insert("°K".to_string());
        temperature.insert("K".to_string());
        
        let mut speed = HashSet::new();
        speed.insert("m/s".to_string());
        speed.insert("km/h".to_string());
        speed.insert("mph".to_string());
        speed.insert("fps".to_string());
        speed.insert("FPS".to_string());
        
        let mut display = HashSet::new();
        display.insert("px".to_string());
        display.insert("pt".to_string());
        display.insert("em".to_string());
        display.insert("rem".to_string());
        display.insert("vh".to_string());
        display.insert("vw".to_string());
        
        let mut frequency = HashSet::new();
        frequency.insert("Hz".to_string());
        frequency.insert("kHz".to_string());
        frequency.insert("MHz".to_string());
        frequency.insert("GHz".to_string());
        
        let mut other = HashSet::new();
        other.insert("°".to_string()); // degrees
        other.insert("%".to_string());
        
        Self {
            time,
            distance,
            data,
            temperature,
            speed,
            display,
            frequency,
            other,
        }
    }
}

impl UnitDictionary {
    /// Check if a string is a known unit
    pub fn contains(&self, unit: &str) -> bool {
        self.time.contains(unit)
            || self.distance.contains(unit)
            || self.data.contains(unit)
            || self.temperature.contains(unit)
            || self.speed.contains(unit)
            || self.display.contains(unit)
            || self.frequency.contains(unit)
            || self.other.contains(unit)
    }
    
    /// Add a custom unit to the dictionary
    pub fn add_unit(&mut self, unit: String, category: UnitCategory) {
        match category {
            UnitCategory::Time => { self.time.insert(unit); }
            UnitCategory::Distance => { self.distance.insert(unit); }
            UnitCategory::Data => { self.data.insert(unit); }
            UnitCategory::Temperature => { self.temperature.insert(unit); }
            UnitCategory::Speed => { self.speed.insert(unit); }
            UnitCategory::Display => { self.display.insert(unit); }
            UnitCategory::Frequency => { self.frequency.insert(unit); }
            UnitCategory::Other => { self.other.insert(unit); }
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UnitCategory {
    Time,
    Distance,
    Data,
    Temperature,
    Speed,
    Display,
    Frequency,
    Other,
}

/// Detects mathematical expressions and numerical patterns
pub struct MathUnitDetector {
    unit_dict: UnitDictionary,
}

impl Default for MathUnitDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl MathUnitDetector {
    pub fn new() -> Self {
        Self {
            unit_dict: UnitDictionary::default(),
        }
    }
    
    pub fn with_custom_units(mut self, units: UnitDictionary) -> Self {
        self.unit_dict = units;
        self
    }
    
    /// Detect if text contains mathematical expressions
    pub fn has_math_expr(&self, text: &str) -> bool {
        MATH_EXPR_REGEX.is_match(text)
    }
    
    /// Detect if text contains ranges/intervals
    pub fn has_range(&self, text: &str) -> bool {
        RANGE_REGEX.is_match(text)
    }
    
    /// Detect if text contains percentages
    pub fn has_percent(&self, text: &str) -> bool {
        PERCENT_REGEX.is_match(text)
    }
    
    /// Detect if text contains scientific notation
    pub fn has_scientific(&self, text: &str) -> bool {
        SCIENTIFIC_NOTATION_REGEX.is_match(text)
    }
    
    /// Detect if text contains units
    pub fn has_units(&self, text: &str) -> bool {
        UNIT_WITH_NUMBER_REGEX.is_match(text)
    }
    
    /// Find all math expressions in text
    pub fn find_math_exprs<'a>(&self, text: &'a str) -> Vec<(usize, usize, &'a str)> {
        MATH_EXPR_REGEX
            .find_iter(text)
            .map(|m| (m.start(), m.end(), m.as_str()))
            .collect()
    }
    
    /// Find all ranges in text
    pub fn find_ranges<'a>(&self, text: &'a str) -> Vec<(usize, usize, &'a str)> {
        RANGE_REGEX
            .find_iter(text)
            .map(|m| (m.start(), m.end(), m.as_str()))
            .collect()
    }
    
    /// Find all percentages in text
    pub fn find_percents<'a>(&self, text: &'a str) -> Vec<(usize, usize, &'a str)> {
        PERCENT_REGEX
            .find_iter(text)
            .map(|m| (m.start(), m.end(), m.as_str()))
            .collect()
    }
    
    /// Find all scientific notations in text
    pub fn find_scientific<'a>(&self, text: &'a str) -> Vec<(usize, usize, &'a str)> {
        SCIENTIFIC_NOTATION_REGEX
            .find_iter(text)
            .map(|m| (m.start(), m.end(), m.as_str()))
            .collect()
    }
    
    /// Find all units with numbers in text
    pub fn find_units<'a>(&self, text: &'a str) -> Vec<(usize, usize, &'a str)> {
        UNIT_WITH_NUMBER_REGEX
            .find_iter(text)
            .map(|m| (m.start(), m.end(), m.as_str()))
            .collect()
    }
    
    /// Get the unit dictionary
    pub fn unit_dict(&self) -> &UnitDictionary {
        &self.unit_dict
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_math_expressions() {
        let detector = MathUnitDetector::new();
        
        assert!(detector.has_math_expr("3.14 × r^2"));
        assert!(detector.has_math_expr("(a+b)/2"));
        assert!(detector.has_math_expr("x ≥ 10"));
        assert!(detector.has_math_expr("2 + 2 = 4"));
        assert!(!detector.has_math_expr("just text"));
    }
    
    #[test]
    fn test_ranges() {
        let detector = MathUnitDetector::new();
        
        assert!(detector.has_range("10-20"));
        assert!(detector.has_range("5~10"));
        assert!(detector.has_range("100–200"));
        assert!(detector.has_range("16-32 ms"));
        assert!(!detector.has_range("no range here"));
    }
    
    #[test]
    fn test_percentages() {
        let detector = MathUnitDetector::new();
        
        assert!(detector.has_percent("50%"));
        assert!(detector.has_percent("{0}%"));
        assert!(detector.has_percent("10-20%"));
        assert!(!detector.has_percent("no percent"));
    }
    
    #[test]
    fn test_scientific_notation() {
        let detector = MathUnitDetector::new();
        
        assert!(detector.has_scientific("1e-6"));
        assert!(detector.has_scientific("2×10^9"));
        assert!(detector.has_scientific("10^3"));
        assert!(!detector.has_scientific("regular text"));
    }
    
    #[test]
    fn test_units() {
        let detector = MathUnitDetector::new();
        
        assert!(detector.has_units("16 ms"));
        assert!(detector.has_units("60 FPS"));
        assert!(detector.has_units("4 GB"));
        assert!(detector.has_units("100 km/h"));
        assert!(detector.has_units("90°"));
        assert!(!detector.has_units("no units"));
    }
    
    #[test]
    fn test_find_math_exprs() {
        let detector = MathUnitDetector::new();
        let text = "The formula is 3.14 × r^2 and x + y = 10";
        let exprs = detector.find_math_exprs(text);
        
        assert_eq!(exprs.len(), 2);
        assert_eq!(exprs[0].2, "3.14 × r^2");
        assert_eq!(exprs[1].2, "x + y = 10");
    }
    
    #[test]
    fn test_find_ranges() {
        let detector = MathUnitDetector::new();
        let text = "Range is 10-20 or 5~10";
        let ranges = detector.find_ranges(text);
        
        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges[0].2, "10-20");
        assert_eq!(ranges[1].2, "5~10");
    }
    
    #[test]
    fn test_unit_dictionary() {
        let dict = UnitDictionary::default();
        
        assert!(dict.contains("ms"));
        assert!(dict.contains("GB"));
        assert!(dict.contains("°C"));
        assert!(dict.contains("km/h"));
        assert!(!dict.contains("unknown"));
    }
    
    #[test]
    fn test_custom_units() {
        let mut dict = UnitDictionary::default();
        dict.add_unit("custom".to_string(), UnitCategory::Other);
        
        assert!(dict.contains("custom"));
    }
    
    #[test]
    fn test_complex_expression() {
        let detector = MathUnitDetector::new();
        let text = "Speed {0}% at 16-32 ms with 2×10^9 operations";
        
        assert!(detector.has_percent(text));
        assert!(detector.has_range(text));
        assert!(detector.has_scientific(text));
    }
}
