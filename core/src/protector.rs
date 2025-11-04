use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

static TAG_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"<[^>]+>").expect("valid tag regex"));

static BB_TAG_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\[(?:/?[a-zA-Z0-9]+|img|/?url)(?:=[^\]]+)?\]").expect("valid bbcode tag regex")
});

static PLACEHOLDER_REGEX: Lazy<Regex> = Lazy::new(|| {
    // Basic placeholders, excluding patterns now handled by specialized regexes
    Regex::new(r"(\{\w+\}|\{\d+\}|%\d*\$?[sd]|%s|%d|%\d+\$d|\$[A-Z0-9_]+\$|\{Pawn_[^}]+\})")
        .expect("valid placeholder regex")
});

// ICU MessageFormat patterns: {var, plural, ...}, {var, select, ...}
// Note: This is a simplified pattern. Full ICU parsing requires a proper parser
// due to nested braces. This catches common simple cases.
static ICU_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\{[^}]+,\s*(?:plural|select|selectordinal)\s*,\s*[^}]+\}").expect("valid ICU regex")
});

// Mustache/Handlebars: {{var}}
static MUSTACHE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\{\{[^}]+\}\}").expect("valid mustache regex")
});

// Unity/RimWorld rich text: <color=#abc>, <sprite=name>
static RICH_TEXT_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"<(?:color|size|sprite|material)(?:=[^>]+)?/?>").expect("valid rich text regex")
});

static ENTITY_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"&(?:[a-zA-Z]+|#x?[0-9a-fA-F]+);").expect("valid entity regex"));

static ESCAPE_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\\[ntr]").expect("valid escape regex"));

static PIPE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\|").expect("valid pipe regex"));

static ID_PATH_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[A-Za-z0-9_.-]+/(?:[A-Za-z0-9_.-]+/?)+").expect("valid id/path regex")
});

static MARKER_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"⟦MT:([A-Z_]+):([0-9]+)⟧").expect("valid marker regex"));

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TokenClass {
    Tag,
    Attr,
    Key,
    Placeholder,
    Icu,
    Mustache,
    RichText,
    Entity,
    Escape,
    Pipe,
    IdPath,
}

impl TokenClass {
    fn code(&self) -> &'static str {
        match self {
            TokenClass::Tag => "TAG",
            TokenClass::Attr => "ATTR",
            TokenClass::Key => "KEY",
            TokenClass::Placeholder => "PLACEHOLDER",
            TokenClass::Icu => "ICU",
            TokenClass::Mustache => "MUSTACHE",
            TokenClass::RichText => "RICHTEXT",
            TokenClass::Entity => "ENTITY",
            TokenClass::Escape => "ESCAPE",
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

        // Collect tokens in priority order (more specific first)
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
            TokenClass::Tag,
            &BB_TAG_REGEX,
        );
        collect_tokens(
            &mut tokens,
            &mut occupied,
            input,
            TokenClass::Placeholder,
            &PLACEHOLDER_REGEX,
        );
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
        assert_eq!(fragment.token_map().tokens.len(), 4);
    }

    #[test]
    fn detect_missing_token() {
        let input = "Use {0} and keep it.";
        let fragment = Protector::protect(input);
        let mutated = fragment
            .masked_text()
            .replace("⟦MT:PLACEHOLDER:1⟧", "dropped");
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
}
