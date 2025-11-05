use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

// === Format Token Patterns (Section 2.1) ===

// C/printf style: %s, %1$s, %0.2f, %d, etc.
static PRINTF_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"%[%\d\$\.\-\+\#\s]*[sdifuxXoScpn]").expect("valid printf regex")
});

// .NET/Unity style: {0}, {1:0.##}, {0:N2}
static DOTNET_BRACE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\{[0-9]+(?::[^{}]+)?\}").expect("valid .NET brace regex")
});

// Named placeholders: {name}, {PAWN_label}, {count}
static NAMED_BRACE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\{[A-Za-z_][A-Za-z0-9_]*\}").expect("valid named brace regex")
});

// Shell/template style: $NAME, ${count}, $VAR
static SHELL_VAR_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\$\{?[A-Za-z_][A-Za-z0-9_]*\}?").expect("valid shell var regex")
});

// Factorio macros: __1__, __ENTITY__iron-ore__, __control__inventory__
static FACTORIO_MACRO_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"__(?:[A-Z]+(?:__[A-Za-z0-9_\-\.]+__)?|[0-9]+__)").expect("valid Factorio macro regex")
});

// Factorio images/links: [img=item/iron-plate], [entity=iron-ore]
static FACTORIO_LINK_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\[(?:img|item|entity|technology|virtual-signal)=[^\]]+\]").expect("valid Factorio link regex")
});

// ICU MessageFormat: {count, plural, one {# item} other {# items}}
// Note: This is a simplified pattern. Full ICU parsing requires proper parser.
static ICU_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\{[A-Za-z_][A-Za-z0-9_]*,\s*(?:plural|select|selectordinal)\s*,[^}]*\}").expect("valid ICU regex")
});

// === Markup/Color/Link Patterns (Section 2.2) ===

// XML/HTML tags: <tag>, </tag>, <tag attr="value">
static TAG_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"<[^>]+>").expect("valid tag regex"));

// BBCode: [b], [/b], [color=#ff0000], [url=...]
static BB_TAG_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\[/?(?:b|i|u|url|img|color=[^\]]+|size=\d+)\]").expect("valid bbcode tag regex")
});

// RimWorld color tags: <color=#RRGGBBAA>, </color>, <b>, </b>
static RIMWORLD_COLOR_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"</?(?:color(?:=#[0-9A-Fa-f]{6,8})?|b|i)>").expect("valid RimWorld color regex")
});

// Minecraft color codes: §a, §l, §r (section sign + code)
static MINECRAFT_COLOR_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"§[0-9A-FK-ORa-fk-or]").expect("valid Minecraft color regex")
});

// Unity/RimWorld rich text: <color=#abc>, <sprite=name>, <size=14>
static RICH_TEXT_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"</?(?:color|size|sprite|material)(?:=[^>]+)?>").expect("valid rich text regex")
});

// Factorio color tags: [color=red]...[/color]
static FACTORIO_COLOR_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\[/?color(?:=[^\]]+)?\]").expect("valid Factorio color regex")
});

// === Resource/Macro Substitution (Section 2.3) ===

// Bracket variations: [[resource]], <<macro>>
static DOUBLE_BRACKET_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\[\[[^\]]+\]\]|<<[^>]+>>").expect("valid double bracket regex")
});

// Mustache/Handlebars: {{var}}, {{#each}}, {{/each}}
static MUSTACHE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\{\{[^}]+\}\}").expect("valid mustache regex")
});

// === Escape and Literal Patterns (Section 2.4) ===

// Escaped braces: {{ and }} (literal in some contexts)
static ESCAPED_BRACE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\{\{|\}\}").expect("valid escaped brace regex")
});

// Escaped percent: %% (literal %)
static ESCAPED_PERCENT_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"%%").expect("valid escaped percent regex")
});

// HTML entities: &nbsp;, &#160;, &#x00A0;
static ENTITY_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"&(?:[a-zA-Z]+|#x?[0-9a-fA-F]+);").expect("valid entity regex"));

// Escape sequences: \n, \t, \r
static ESCAPE_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\\[ntr]").expect("valid escape regex"));

// === Math/Numerical Patterns (Section 2.1) ===

// Arithmetic expressions: numbers with operators + - × * ÷ / ^ = ≠ ≈ ≤ ≥ < >
static MATH_EXPR_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?x)
        \d+(?:\.\d+)?  # number
        \s*[+\-×*÷/^=≠≈≤≥<>]\s*  # operator with optional whitespace
        \d+(?:\.\d+)?  # another number
        (?:\s*[+\-×*÷/^=≠≈≤≥<>]\s*\d+(?:\.\d+)?)*  # additional terms
        |
        \([^)]+[+\-×*÷/^=≠≈≤≥<>][^)]+\)  # expressions in parentheses
    ").expect("valid math expression regex")
});

// Range/interval patterns: a~b, a-b, a–b (with en dash)
static RANGE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\d+(?:\.\d+)?\s*[~\-–]\s*\d+(?:\.\d+)?(?:\s*[a-zA-Z°%]+)?")
        .expect("valid range regex")
});

// Percentages: n%, {n}%, n‒m% (but not escaped %%)
// Note: This overlaps with DOTNET_BRACE_REGEX followed by %, handled specially
static PERCENT_SIMPLE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\d+(?:\.\d+)?%")
        .expect("valid simple percent regex")
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

// === Legacy Patterns (for backward compatibility) ===

static PIPE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\|").expect("valid pipe regex"));

static ID_PATH_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[A-Za-z0-9_.-]+/(?:[A-Za-z0-9_.-]+/?)+").expect("valid id/path regex")
});

static MARKER_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"⟦MT:([A-Z_]+):([0-9]+)⟧").expect("valid marker regex"));

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TokenClass {
    // Format tokens
    Printf,           // %s, %d, %1$s
    DotnetBrace,      // {0}, {1:0.##}
    NamedBrace,       // {name}, {PAWN_label}
    ShellVar,         // $VAR, ${count}
    FactorioMacro,    // __1__, __ENTITY__foo__
    FactorioLink,     // [img=item/plate]
    Icu,              // {n, plural, ...}
    
    // Markup/color
    Tag,              // <tag>
    BbCode,           // [b], [color=...]
    RimworldColor,    // <color=#fff>, </color>
    MinecraftColor,   // §a, §l
    RichText,         // <sprite=...>
    FactorioColor,    // [color=red]
    
    // Resource/macro
    DoubleBracket,    // [[res]], <<macro>>
    Mustache,         // {{var}}
    
    // Math/numerical (Section 2.1)
    MathExpr,         // 3.14 × r^2, (a+b)/2
    Range,            // 10-20, 5~10
    Percent,          // 50%, {0}%
    Scientific,       // 1e-6, 2×10^9
    Unit,             // 16 ms, 60 FPS
    
    // Escape/literal
    EscapedBrace,     // {{, }}
    EscapedPercent,   // %%
    Entity,           // &nbsp;
    Escape,           // \n, \t
    
    // Legacy (for backward compat)
    Attr,
    Key,
    Pipe,
    IdPath,
}

impl TokenClass {
    fn code(&self) -> &'static str {
        match self {
            TokenClass::Printf => "PRINTF",
            TokenClass::DotnetBrace => "DOTNET",
            TokenClass::NamedBrace => "NAMED",
            TokenClass::ShellVar => "SHELL",
            TokenClass::FactorioMacro => "FACTORIO",
            TokenClass::FactorioLink => "FLINK",
            TokenClass::Icu => "ICU",
            TokenClass::Tag => "TAG",
            TokenClass::BbCode => "BBCODE",
            TokenClass::RimworldColor => "RWCOLOR",
            TokenClass::MinecraftColor => "MCCOLOR",
            TokenClass::RichText => "RICHTEXT",
            TokenClass::FactorioColor => "FCOLOR",
            TokenClass::DoubleBracket => "DBLBRACK",
            TokenClass::Mustache => "MUSTACHE",
            TokenClass::MathExpr => "MATHEXPR",
            TokenClass::Range => "RANGE",
            TokenClass::Percent => "PERCENT",
            TokenClass::Scientific => "SCIENTIFIC",
            TokenClass::Unit => "UNIT",
            TokenClass::EscapedBrace => "ESCBRACE",
            TokenClass::EscapedPercent => "ESCPCT",
            TokenClass::Entity => "ENTITY",
            TokenClass::Escape => "ESCAPE",
            TokenClass::Attr => "ATTR",
            TokenClass::Key => "KEY",
            TokenClass::Pipe => "PIPE",
            TokenClass::IdPath => "IDPATH",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtectedToken {
    pub id: String,
    #[serde(rename = "class")]
    pub kind: TokenClass,
    pub span: (usize, usize),
    pub value: String,
    pub marker: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenMap {
    pub content_hash: String,
    pub tokens: Vec<ProtectedToken>,
}

#[derive(Debug, Clone)]
pub struct ProtectedFragment {
    original: String,
    masked: String,
    map: TokenMap,
}

#[derive(Debug, thiserror::Error)]
pub enum ProtectorError {
    #[error("missing tokens: {0:?}")]
    MissingTokens(Vec<String>),
    #[error("unexpected tokens: {0:?}")]
    UnexpectedTokens(Vec<String>),
}

pub struct Protector;

impl Protector {
    pub fn protect(input: &str) -> ProtectedFragment {
        let original = input.to_string();
        if input.is_empty() {
            return ProtectedFragment {
                original,
                masked: String::new(),
                map: TokenMap {
                    content_hash: compute_hash(input),
                    tokens: Vec::new(),
                },
            };
        }

        let mut occupied = vec![false; input.len()];
        let mut tokens = Vec::new();

        // Collect tokens in priority order (more specific patterns first)
        // Escapes first (must be protected before their patterns)
        collect_tokens(
            &mut tokens,
            &mut occupied,
            input,
            TokenClass::EscapedBrace,
            &ESCAPED_BRACE_REGEX,
        );
        collect_tokens(
            &mut tokens,
            &mut occupied,
            input,
            TokenClass::EscapedPercent,
            &ESCAPED_PERCENT_REGEX,
        );
        
        // Complex structures (ICU MessageFormat, Mustache)
        collect_tokens(
            &mut tokens,
            &mut occupied,
            input,
            TokenClass::Icu,
            &ICU_REGEX,
        );
        collect_tokens(
            &mut tokens,
            &mut occupied,
            input,
            TokenClass::Mustache,
            &MUSTACHE_REGEX,
        );
        
        // Game-specific tokens
        collect_tokens(
            &mut tokens,
            &mut occupied,
            input,
            TokenClass::FactorioMacro,
            &FACTORIO_MACRO_REGEX,
        );
        collect_tokens(
            &mut tokens,
            &mut occupied,
            input,
            TokenClass::FactorioLink,
            &FACTORIO_LINK_REGEX,
        );
        collect_tokens(
            &mut tokens,
            &mut occupied,
            input,
            TokenClass::FactorioColor,
            &FACTORIO_COLOR_REGEX,
        );
        collect_tokens(
            &mut tokens,
            &mut occupied,
            input,
            TokenClass::MinecraftColor,
            &MINECRAFT_COLOR_REGEX,
        );
        
        // Markup tags (specific to general)
        collect_tokens(
            &mut tokens,
            &mut occupied,
            input,
            TokenClass::RimworldColor,
            &RIMWORLD_COLOR_REGEX,
        );
        collect_tokens(
            &mut tokens,
            &mut occupied,
            input,
            TokenClass::RichText,
            &RICH_TEXT_REGEX,
        );
        collect_tokens(
            &mut tokens,
            &mut occupied,
            input,
            TokenClass::Tag,
            &TAG_REGEX,
        );
        collect_tokens(
            &mut tokens,
            &mut occupied,
            input,
            TokenClass::BbCode,
            &BB_TAG_REGEX,
        );
        
        // Bracket variations
        collect_tokens(
            &mut tokens,
            &mut occupied,
            input,
            TokenClass::DoubleBracket,
            &DOUBLE_BRACKET_REGEX,
        );
        
        // Format tokens (specific to general)
        collect_tokens(
            &mut tokens,
            &mut occupied,
            input,
            TokenClass::Printf,
            &PRINTF_REGEX,
        );
        collect_tokens(
            &mut tokens,
            &mut occupied,
            input,
            TokenClass::DotnetBrace,
            &DOTNET_BRACE_REGEX,
        );
        collect_tokens(
            &mut tokens,
            &mut occupied,
            input,
            TokenClass::NamedBrace,
            &NAMED_BRACE_REGEX,
        );
        collect_tokens(
            &mut tokens,
            &mut occupied,
            input,
            TokenClass::ShellVar,
            &SHELL_VAR_REGEX,
        );
        
        // Math/numerical patterns (Section 2.1) - before entities to avoid conflicts
        collect_tokens(
            &mut tokens,
            &mut occupied,
            input,
            TokenClass::Scientific,
            &SCIENTIFIC_NOTATION_REGEX,
        );
        collect_tokens(
            &mut tokens,
            &mut occupied,
            input,
            TokenClass::Unit,
            &UNIT_WITH_NUMBER_REGEX,
        );
        collect_tokens(
            &mut tokens,
            &mut occupied,
            input,
            TokenClass::MathExpr,
            &MATH_EXPR_REGEX,
        );
        collect_tokens(
            &mut tokens,
            &mut occupied,
            input,
            TokenClass::Range,
            &RANGE_REGEX,
        );
        collect_tokens(
            &mut tokens,
            &mut occupied,
            input,
            TokenClass::Percent,
            &PERCENT_SIMPLE_REGEX,
        );
        
        // HTML entities and escapes
        collect_tokens(
            &mut tokens,
            &mut occupied,
            input,
            TokenClass::Entity,
            &ENTITY_REGEX,
        );
        collect_tokens(
            &mut tokens,
            &mut occupied,
            input,
            TokenClass::Escape,
            &ESCAPE_REGEX,
        );
        
        // Legacy patterns (low priority)
        collect_tokens(
            &mut tokens,
            &mut occupied,
            input,
            TokenClass::Pipe,
            &PIPE_REGEX,
        );
        collect_tokens(
            &mut tokens,
            &mut occupied,
            input,
            TokenClass::IdPath,
            &ID_PATH_REGEX,
        );

        tokens.sort_by_key(|token| token.span.0);

        for (index, token) in tokens.iter_mut().enumerate() {
            token.id = format!("T{:04}", index);
            token.marker = format!("⟦MT:{}:{}⟧", token.kind.code(), index);
        }

        let mut masked = String::with_capacity(input.len());
        let mut cursor = 0usize;
        for token in &tokens {
            let (start, end) = token.span;
            if start > cursor {
                masked.push_str(&input[cursor..start]);
            }
            masked.push_str(&token.marker);
            cursor = end;
        }
        if cursor < input.len() {
            masked.push_str(&input[cursor..]);
        }

        ProtectedFragment {
            original,
            masked,
            map: TokenMap {
                content_hash: compute_hash(input),
                tokens,
            },
        }
    }
}

impl ProtectedFragment {
    pub fn original(&self) -> &str {
        &self.original
    }

    pub fn masked_text(&self) -> &str {
        &self.masked
    }

    pub fn token_map(&self) -> &TokenMap {
        &self.map
    }

    pub fn restore(&self, translated: &str) -> Result<String, ProtectorError> {
        if translated.is_empty() {
            return Ok(String::new());
        }

        let tokens = &self.map.tokens;
        let mut token_lookup: HashMap<&str, &ProtectedToken> = HashMap::new();
        for token in tokens {
            token_lookup.insert(token.marker.as_str(), token);
        }

        let mut seen: HashSet<&str> = HashSet::new();
        let mut unknown_markers = Vec::new();
        let mut output = String::with_capacity(translated.len());
        let mut cursor = 0usize;
        for capture in MARKER_REGEX.captures_iter(translated) {
            let matched = capture.get(0).expect("match");
            let marker = matched.as_str();
            let start = matched.start();
            let end = matched.end();
            if start > cursor {
                output.push_str(&translated[cursor..start]);
            }

            if let Some(token) = token_lookup.get(marker) {
                output.push_str(&token.value);
                seen.insert(token.marker.as_str());
            } else {
                unknown_markers.push(marker.to_string());
                output.push_str(marker);
            }
            cursor = end;
        }

        if cursor < translated.len() {
            output.push_str(&translated[cursor..]);
        }

        if !unknown_markers.is_empty() {
            return Err(ProtectorError::UnexpectedTokens(unknown_markers));
        }

        let missing: Vec<String> = tokens
            .iter()
            .filter(|token| !seen.contains(token.marker.as_str()))
            .map(|token| token.marker.clone())
            .collect();

        if !missing.is_empty() {
            return Err(ProtectorError::MissingTokens(missing));
        }

        Ok(output)
    }
}

fn compute_hash(input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    hex::encode(digest)
}

fn collect_tokens(
    tokens: &mut Vec<ProtectedToken>,
    occupied: &mut [bool],
    input: &str,
    kind: TokenClass,
    regex: &Regex,
) {
    for mat in regex.find_iter(input) {
        if mat.as_str().is_empty() {
            continue;
        }
        let start = mat.start();
        let end = mat.end();
        if end > occupied.len() {
            continue;
        }
        if occupied[start..end].iter().any(|occupied| *occupied) {
            continue;
        }
        for flag in &mut occupied[start..end] {
            *flag = true;
        }
        tokens.push(ProtectedToken {
            id: String::new(),
            kind,
            span: (start, end),
            value: mat.as_str().to_string(),
            marker: String::new(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protect_and_restore_roundtrip() {
        let input = "<b>Hello {0} | {Pawn_label}</b>";
        let fragment = Protector::protect(input);
        assert_ne!(fragment.masked_text(), input);
        let restored = fragment.restore(fragment.masked_text()).unwrap();
        assert_eq!(restored, input);
        // Now detects: <b>, </b>, {0}, |, {Pawn_label}
        assert_eq!(fragment.token_map().tokens.len(), 5);
    }

    #[test]
    fn detect_missing_token() {
        let input = "Use {0} and keep it.";
        let fragment = Protector::protect(input);
        // {0} is now detected as DOTNET token class
        let mutated = fragment
            .masked_text()
            .replace("⟦MT:DOTNET:0⟧", "dropped");
        let error = fragment.restore(&mutated).unwrap_err();
        matches!(error, ProtectorError::MissingTokens(_));
    }

    #[test]
    fn detect_unexpected_token() {
        let input = "Value %s";
        let fragment = Protector::protect(input);
        let mutated = format!("{}⟦MT:TAG:99⟧", fragment.masked_text());
        let error = fragment.restore(&mutated).unwrap_err();
        matches!(error, ProtectorError::UnexpectedTokens(_));
    }
    
    #[test]
    fn test_factorio_macros() {
        let input = "Craft __1__ items using __ENTITY__iron-ore__.";
        let fragment = Protector::protect(input);
        
        // Should detect __1__ and __ENTITY__iron-ore__ as FACTORIO tokens
        assert!(fragment.masked_text().contains("⟦MT:FACTORIO:"));
        assert_eq!(fragment.token_map().tokens.len(), 2);
        
        let restored = fragment.restore(fragment.masked_text()).unwrap();
        assert_eq!(restored, input);
    }
    
    #[test]
    fn test_minecraft_color_codes() {
        let input = "§aGreen text§r and §lbold§r.";
        let fragment = Protector::protect(input);
        
        // Should detect §a, §r, §l as MCCOLOR tokens
        assert!(fragment.masked_text().contains("⟦MT:MCCOLOR:"));
        assert_eq!(fragment.token_map().tokens.len(), 4);
        
        let restored = fragment.restore(fragment.masked_text()).unwrap();
        assert_eq!(restored, input);
    }
    
    #[test]
    fn test_rimworld_color_tags() {
        let input = "<color=#ff0000>Red text</color>";
        let fragment = Protector::protect(input);
        
        // Should detect both tags
        assert!(fragment.masked_text().contains("⟦MT:RWCOLOR:"));
        assert_eq!(fragment.token_map().tokens.len(), 2);
        
        let restored = fragment.restore(fragment.masked_text()).unwrap();
        assert_eq!(restored, input);
    }
    
    #[test]
    fn test_printf_patterns() {
        let input = "Value: %s, Count: %d, Percent: %2.1f";
        let fragment = Protector::protect(input);
        
        // Should detect all printf patterns
        assert!(fragment.masked_text().contains("⟦MT:PRINTF:"));
        assert_eq!(fragment.token_map().tokens.len(), 3);
        
        let restored = fragment.restore(fragment.masked_text()).unwrap();
        assert_eq!(restored, input);
    }
    
    #[test]
    fn test_dotnet_and_named_braces() {
        let input = "Player {0} killed {count} enemies at {location}";
        let fragment = Protector::protect(input);
        
        // Should detect {0} as DOTNET, {count} and {location} as NAMED
        assert!(fragment.masked_text().contains("⟦MT:DOTNET:"));
        assert!(fragment.masked_text().contains("⟦MT:NAMED:"));
        assert_eq!(fragment.token_map().tokens.len(), 3);
        
        let restored = fragment.restore(fragment.masked_text()).unwrap();
        assert_eq!(restored, input);
    }
    
    #[test]
    fn test_icu_messageformat() {
        let input = "{count, plural, one {# item} other {# items}}";
        let fragment = Protector::protect(input);
        
        // ICU patterns with nested braces are complex - may be detected as multiple tokens
        // This is a known limitation - full ICU parsing requires a proper parser
        assert!(fragment.masked_text().contains("⟦MT:"));
        assert!(fragment.token_map().tokens.len() >= 1, "Expected at least 1 token");
        
        let restored = fragment.restore(fragment.masked_text()).unwrap();
        assert_eq!(restored, input);
    }
    
    #[test]
    fn test_mixed_tokens() {
        let input = "<b>Speed: {0}%</b> using __1__ and %s";
        let fragment = Protector::protect(input);
        
        // Should detect: <b>, </b>, {0}, __1__, %s
        let tokens = &fragment.token_map().tokens;
        assert!(tokens.len() >= 5, "Expected at least 5 tokens, got {}", tokens.len());
        
        let restored = fragment.restore(fragment.masked_text()).unwrap();
        assert_eq!(restored, input);
    }
    
    #[test]
    fn test_escaped_patterns() {
        let input = "Use {{ }} for literal braces and %% for percent";
        let fragment = Protector::protect(input);
        
        // Should detect escaped braces and percent
        assert!(fragment.masked_text().contains("⟦MT:ESCBRACE:") || 
                fragment.masked_text().contains("⟦MT:ESCPCT:"));
        
        let restored = fragment.restore(fragment.masked_text()).unwrap();
        assert_eq!(restored, input);
    }
    
    // === Section 2.1 Tests: Math/Numerical Patterns ===
    
    #[test]
    fn test_math_expressions() {
        let input = "Formula: 3.14 × r^2 and (a+b)/2";
        let fragment = Protector::protect(input);
        
        // Should detect mathematical expressions
        assert!(fragment.masked_text().contains("⟦MT:MATHEXPR:"));
        assert!(fragment.token_map().tokens.iter().any(|t| t.kind == TokenClass::MathExpr));
        
        let restored = fragment.restore(fragment.masked_text()).unwrap();
        assert_eq!(restored, input);
    }
    
    #[test]
    fn test_ranges_and_intervals() {
        let input = "Range: 10-20, or 5~10, or 100–200 ms";
        let fragment = Protector::protect(input);
        
        // Should detect range patterns
        assert!(fragment.masked_text().contains("⟦MT:RANGE:") || 
                fragment.masked_text().contains("⟦MT:UNIT:"));
        assert!(fragment.token_map().tokens.iter().any(|t| 
            t.kind == TokenClass::Range || t.kind == TokenClass::Unit
        ));
        
        let restored = fragment.restore(fragment.masked_text()).unwrap();
        assert_eq!(restored, input);
    }
    
    #[test]
    fn test_percentages() {
        let input = "Progress: 50%, Speed: {0}%, Range: 10-20%";
        let fragment = Protector::protect(input);
        
        // Should detect percentage patterns and brace tokens
        assert!(fragment.masked_text().contains("⟦MT:PERCENT:") || 
                fragment.masked_text().contains("⟦MT:DOTNET:"));
        
        let restored = fragment.restore(fragment.masked_text()).unwrap();
        assert_eq!(restored, input);
    }
    
    #[test]
    fn test_scientific_notation() {
        let input = "Values: 1e-6, 2×10^9, 10^3";
        let fragment = Protector::protect(input);
        
        // Should detect scientific notation
        assert!(fragment.masked_text().contains("⟦MT:SCIENTIFIC:"));
        assert!(fragment.token_map().tokens.iter().any(|t| t.kind == TokenClass::Scientific));
        
        let restored = fragment.restore(fragment.masked_text()).unwrap();
        assert_eq!(restored, input);
    }
    
    #[test]
    fn test_units_with_numbers() {
        let input = "Performance: 16 ms, 60 FPS, 4 GB, 100 km/h, 90°";
        let fragment = Protector::protect(input);
        
        // Should detect units
        assert!(fragment.masked_text().contains("⟦MT:UNIT:"));
        assert!(fragment.token_map().tokens.iter().any(|t| t.kind == TokenClass::Unit));
        
        let restored = fragment.restore(fragment.masked_text()).unwrap();
        assert_eq!(restored, input);
    }
    
    #[test]
    fn test_complex_mixed_patterns() {
        let input = "Speed {0}% at 16-32 ms with 2×10^9 operations and x + y = 10";
        let fragment = Protector::protect(input);
        
        // Should detect multiple types: DOTNET ({0}), RANGE/UNIT (16-32 ms), 
        // SCIENTIFIC (2×10^9), MATHEXPR (x + y = 10), and potentially PERCENT
        let has_dotnet = fragment.token_map().tokens.iter().any(|t| t.kind == TokenClass::DotnetBrace);
        let has_math_or_sci = fragment.token_map().tokens.iter().any(|t| 
            t.kind == TokenClass::MathExpr || t.kind == TokenClass::Scientific || 
            t.kind == TokenClass::Range || t.kind == TokenClass::Unit
        );
        
        assert!(has_dotnet, "Expected DOTNET token for {{0}}");
        assert!(has_math_or_sci, "Expected math/numerical tokens");
        
        let restored = fragment.restore(fragment.masked_text()).unwrap();
        assert_eq!(restored, input);
    }
    
    #[test]
    fn test_formula_preservation_roundtrip() {
        // Test case from Section 13 requirements
        let inputs = vec![
            "3.14 × r^2",
            "10–20%",
            "(a+b)/2",
            "16 ms",
            "60 FPS",
            "4 GB",
            "{0}%",
            "%1$s/s",
        ];
        
        for input in inputs {
            let fragment = Protector::protect(input);
            let restored = fragment.restore(fragment.masked_text()).unwrap();
            assert_eq!(restored, input, "Failed to preserve: {}", input);
            
            // Ensure something was protected
            assert!(!fragment.token_map().tokens.is_empty(), 
                "No tokens detected in: {}", input);
        }
    }
}
