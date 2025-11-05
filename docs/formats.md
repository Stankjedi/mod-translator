# Supported File Formats

The mod-translator supports various file formats commonly used in game modding. Each format has specialized handling to ensure correct translation while preserving structure and non-translatable elements.

## Overview

| Format | Extensions | Status | Scanner | Handler |
|--------|-----------|--------|---------|---------|
| XML | `.xml` | ✅ Full Support | Built-in | XmlHandler |
| JSON | `.json`, `.jsonl` | ✅ Full Support | Built-in | JsonHandler |
| YAML | `.yaml`, `.yml` | ✅ Full Support | Built-in | YamlHandler |
| PO (gettext) | `.po`, `.pot` | ✅ Full Support | Built-in | PoHandler |
| INI/CFG | `.ini`, `.cfg` | ✅ Full Support | Built-in | IniHandler |
| CSV | `.csv`, `.tsv` | ✅ Full Support | Built-in | CsvHandler |
| Properties | `.properties` | ✅ Full Support | PropertiesScanner | PropertiesHandler |
| Lua | `.lua` | ✅ Full Support | LuaScanner | LuaHandler |
| Markdown | `.md`, `.markdown` | ✅ Full Support | MarkdownScanner | MarkdownHandler |
| Plain Text | `.txt` | ✅ Full Support | - | TxtHandler |

## Format Details

### Markdown

**Extensions:** `.md`, `.markdown`

**What gets translated:**
- Natural text in paragraphs
- Headers
- List items
- Blockquotes

**What is protected (non-translatable):**
- Code spans: `` `code` ``
- Code fences: ` ```language\ncode\n``` `
- Links and URLs: `[text](url)`
- Images: `![alt](url)`
- Inline math: `$x = y$`, `\(formula\)`
- Display math: `\[equation\]`
- HTML tags: `<tag>content</tag>`
- Reference links: `[label]: url`

**Validation:**
- Code fences must be balanced (opening and closing)
- No structural validation beyond fence balance

**Example:**
```markdown
# Welcome Guide

This is a paragraph with `code` and [a link](https://example.com).

```python
# Code is not translated
def hello():
    return "Hello"
```

Regular text resumes here.
```

### Properties (Java .properties format)

**Extensions:** `.properties`

**What gets translated:**
- Value part of `key=value` pairs
- Value part of `key:value` pairs

**What is protected (non-translatable):**
- Keys
- Comments (`#` and `!` prefix)
- Unicode escapes: `\uXXXX`
- Format tokens: `%s`, `%d`, `{0}`, `{1:0.##}`
- Line continuations (backslash at end of line)

**Validation:**
- Each line must be empty, a comment, or valid key=value structure
- Unicode escapes must be valid hex digits
- Key=value structure preserved

**Example:**
```properties
# Application messages
welcome.message=Hello, %s!
item.count=You have {0} items
unicode.text=\u4f60\u597d World
```

After translation:
```properties
# Application messages
welcome.message=안녕하세요, %s!
item.count=당신은 {0}개의 항목을 가지고 있습니다
unicode.text=\u4f60\u597d 세계
```

### Lua

**Extensions:** `.lua`

**What gets translated:**
- String literal values
- Single quote strings: `'text'`
- Double quote strings: `"text"`
- Long bracket strings: `[[text]]`

**What is protected (non-translatable):**
- Variable and function names
- Keys in tables
- Comments: `--` and `--[[...]]`
- Escape sequences: `\n`, `\"`, `\\`
- Format tokens: `%s`, `%d`, `{0}`
- ICU MessageFormat blocks

**Validation:**
- String quotes must be balanced
- Comments properly closed

**Example:**
```lua
-- Localization table
local L = {
    greeting = "Hello, %s!",
    farewell = 'Goodbye',
    multiline = [[This is a
    multiline string]],
    formatted = "Score: {0}"
}

return L
```

After translation:
```lua
-- Localization table
local L = {
    greeting = "안녕하세요, %s!",
    farewell = '안녕히 가세요',
    multiline = [[이것은
    여러 줄 문자열입니다]],
    formatted = "점수: {0}"
}

return L
```

## Token Protection System

All formats use a unified token protection system that:

1. **Identifies non-translatable elements** using format-specific scanners
2. **Replaces them with protected tokens**: `⟦MT:TYPE:N⟧`
3. **Sends masked text to translation API**
4. **Restores original tokens** in translated output
5. **Validates token preservation** with auto-recovery

### Token Types

| Token Type | Description | Example |
|-----------|-------------|---------|
| `CODE_FENCE` | Markdown code fence | ` ```python...``` ` |
| `CODE_SPAN` | Markdown inline code | `` `code` `` |
| `MATHEXPR` | LaTeX math expression | `$E = mc^2$` |
| `MARKDOWN_LINK` | Markdown link syntax | `[text](url)` |
| `MARKDOWN_IMAGE` | Markdown image syntax | `![alt](url)` |
| `TAG` | HTML/XML tags | `<tag>` |
| `ESCAPE` | Escape sequences | `\n`, `\uXXXX` |
| `PRINTF` | Printf-style tokens | `%s`, `%d` |
| `DOTNET` | .NET format tokens | `{0}`, `{1:0.##}` |
| `ICU` | ICU MessageFormat | `{count, plural, ...}` |

## Format Detection

Formats are automatically detected by:

1. **File extension** (primary method)
2. **Content signature** (fallback)
3. **Manual specification** (override)

### Extension Mapping

```rust
FileFormat::from_extension(ext) -> FileFormat
```

Examples:
- `.md` → `FileFormat::Markdown`
- `.properties` → `FileFormat::Properties`
- `.lua` → `FileFormat::Lua`

### Content-Based Detection

When extension is ambiguous or missing, the scanner examines content patterns:

- **Markdown**: Code fences, link syntax, header markers
- **Properties**: High ratio of `key=value` or `key:value` lines
- **Lua**: Presence of `local`, `return`, `--` comments, string literals

## Best Practices

### Markdown
- Use consistent code fence markers (``` or ~~~)
- Keep link text translatable, URLs protected
- Separate code examples from narrative text
- Use proper heading hierarchy

### Properties
- Use UTF-8 encoding for modern files
- Escape special characters properly
- Keep keys consistent across languages
- Use format tokens for dynamic values

### Lua
- Use localization tables for all strings
- Keep string concatenation minimal
- Avoid complex string operations in source files
- Use consistent quote style

## Limitations

### Current Limitations

**Markdown:**
- Does not split by paragraphs (treats file as single unit)
- Limited nesting support in complex documents
- No special handling for tables

**Properties:**
- Line continuations are preserved but not optimized
- Limited support for property files with custom delimiters

**Lua:**
- Merge function returns original (position tracking TBD)
- No support for string.format style interpolation detection
- Long bracket strings with custom separators not fully supported

### Planned Enhancements

- Paragraph-level segmentation for Markdown
- Full merge support for Lua with position tracking
- Support for TOML format
- Support for Gettext PO/POT format extensions
- Enhanced table parsing for Markdown

## Error Handling

### Validation Errors

Each format has specific validation error codes:

| Error Code | Format | Description |
|-----------|--------|-------------|
| `MARKDOWN_UNBALANCED_FENCE` | Markdown | Code fences not properly closed |
| `PROPERTIES_ESCAPE_INVALID` | Properties | Invalid unicode escape sequence |
| `LUA_STRING_UNBALANCED` | Lua | String quotes not balanced |
| `PARSER_ERROR` | All | Generic parsing failure |

### Auto-Recovery

The validator attempts automatic recovery:
1. Reinject missing protected tokens
2. Balance opening/closing pairs
3. Remove excess tokens
4. Preserve format-specific structures

## Testing

### Running Format Tests

```bash
# Test all formats
cargo test formats::

# Test specific format
cargo test formats::markdown::tests
cargo test formats::properties::tests
cargo test formats::lua::tests

# Test scanners
cargo test scanners::

# Test validators
cargo test format_validator::
```

### Sample Test Files

Test files are located in:
- `core/tests/fixtures/markdown/`
- `core/tests/fixtures/properties/`
- `core/tests/fixtures/lua/`

## Integration

### Using Format Handlers

```rust
use mod_translator_core::formats::{FileFormat, get_handler};

// Detect format
let format = FileFormat::from_extension("md");

// Get handler
let handler = get_handler(format).unwrap();

// Extract translatable entries
let entries = handler.extract(content)?;

// Translate entries...
// translations = translate(entries)

// Merge back
let output = handler.merge(content, &translations)?;
```

### Using Scanners

```rust
use mod_translator_core::scanners::{MarkdownScanner, PropertiesScanner, LuaScanner};

// Markdown
let mut md_scanner = MarkdownScanner::new();
let result = md_scanner.scan(text);

// Properties
let mut props_scanner = PropertiesScanner::new();
let entries = props_scanner.parse_file(content);
let result = props_scanner.scan_value(&value);

// Lua
let mut lua_scanner = LuaScanner::new();
let literals = lua_scanner.parse_file(content);
let result = lua_scanner.scan_string(&string_content);
```

## Configuration

Format-specific options can be configured in profiles:

```yaml
# profiles/default.yaml
preserve_markdown_links: true
preserve_code_fences: true
preserve_properties_unicode: true
preserve_lua_strings: true
preserve_percent_binding: true
```

## See Also

- [Placeholder Validator](./placeholder-validator.md)
- [Universal Token Validator](./universal-token-validator.md)
- [Implementation Summary](./IMPLEMENTATION_SUMMARY.md)
- [Codex Implementation](./CODEX_IMPLEMENTATION.md)
