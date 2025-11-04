# Universal Mod Translator Architecture

## Overview

The Universal Mod Translator is a comprehensive system for translating Steam Workshop mods while preserving all code structures, placeholders, and formatting tokens.

## Core Principles

1. **100% Code Preservation**: Tags, placeholders, escapes, and entities must be preserved exactly
2. **Format-Agnostic**: Support XML, JSON, YAML, PO, INI, CFG, CSV, Properties, Lua, TXT/Markdown
3. **Game-Specific Rules**: Plugin system for game-specific detection and rules
4. **Selective Rollback**: Failed keys rollback, successful keys merge
5. **Streaming for Large Files**: Handle multi-megabyte files without exhausting memory
6. **Atomic Writes**: Backup originals, write to temp, then swap

## Architecture Components

### 1. Format Handlers (`core/src/formats/`)

Each format has a handler implementing the `FormatHandler` trait:

- `extract()`: Pull translatable key-value pairs from file
- `merge()`: Reinsert translations while preserving structure

**Implemented:**
- JSON: Full implementation with nested object support
- INI/CFG: Section-aware key=value parsing
- XML, YAML, PO, CSV, Properties, Lua, TXT: Stub implementations

**Key Principle:** Only extract and translate *values*, never keys/tags/structure.

### 2. File Scanner (`core/src/scanner.rs`)

Scans mod directories with configurable rules:

- **Include Paths**: `Languages/`, `locale/`, `i18n/`, `strings/`, `text/`
- **Exclude Patterns**: Binary files (`.dll`, `.exe`, `.png`, etc.)
- **Binary Detection**: 20% non-ASCII threshold
- **Size Limits**: Default 20MB per file

### 3. Game Profiles (`core/src/profiles/`)

Plugin system for game-specific translation rules:

**Implemented Profiles:**
- **RimWorld**: Detects `About/About.xml`, protects `{PAWN_*}` tokens
- **Factorio**: Detects `info.json`, protects `__ENTITY__` tokens, uses `locale/*.cfg`
- **Stardew Valley**: Detects `manifest.json`, uses `i18n/*.json`
- **Generic**: Fallback for unrecognized mods

**Profile Components:**
- Detection rules (folder patterns, manifest signatures)
- Include/exclude paths
- Extra placeholder patterns
- Terminology dictionary

### 4. Protection System (`core/src/protector.rs`)

Replaces protected tokens with markers before translation:

**Protected Token Types:**
- **Tags**: HTML/XML tags, BBCode
- **Placeholders**: `{0}`, `{name}`, `%s`, `%1$d`, `$VAR$`
- **ICU MessageFormat**: `{count, plural, one {1 item} other {# items}}`
- **Mustache**: `{{variable}}`
- **Rich Text**: `<color=#ff0000>`, `<sprite=icon>`
- **Entities**: `&lt;`, `&#10;`, `&nbsp;`
- **Escapes**: `\n`, `\r`, `\t`, `\"`
- **Pipes**: `|` (count must match)
- **Paths**: `data/core/items.xml`

**Process:**
1. Scan for all protected patterns
2. Replace with unique markers: `⟦MT:PLACEHOLDER:0⟧`
3. Translate masked text
4. Restore markers to original tokens
5. Validate 1:1 correspondence

### 5. Validation System (`core/src/validator.rs`)

Multi-gate validation ensures quality:

**Error Codes:**
- `STRUCTURE_MISMATCH`: XML/JSON structure changed
- `TAG_SET_MISMATCH`: Tag set doesn't match
- `PLACEHOLDER_MISMATCH`: Token count/order mismatch
- `PIPE_DELIM_MISMATCH`: Pipe delimiter count changed
- `ESCAPE_ENTITY_DRIFT`: Entity/escape altered
- `EMPTY_VALUE`: Result is empty
- `OVERLONG_DELTA`: Length >4x original (warning)
- `ILLEGAL_BACKTICK`: Backtick in code context

**Validation Flow:**
1. Check non-empty
2. Verify token preservation
3. Validate pipe count
4. Check length ratio
5. Fail fast on errors, warn on concerns

### 6. Encoding Preservation (`core/src/encoding.rs`)

Maintains original file characteristics:

**Encoding Support:**
- UTF-8 (with or without BOM)
- UTF-16 LE/BE
- Latin-1 (fallback)

**Features:**
- Auto-detection with confidence scoring
- BOM preservation
- Newline style detection (LF vs CRLF)
- Round-trip guarantee

### 7. Translation Pipeline

**Flow:**
1. **Scan**: Find translatable files using scanner
2. **Detect**: Identify game profile and format
3. **Load**: Read file with encoding detection
4. **Extract**: Pull translatable entries via format handler
5. **Protect**: Mask all protected tokens
6. **Translate**: Send masked text to AI provider
7. **Validate**: Check all validation gates
8. **Restore**: Unmask tokens
9. **Merge**: Reinsert translations into original structure
10. **Write**: Save with original encoding/newline style

**Rollback Strategy:**
- Key-level granularity
- Failed keys: Keep original, log error
- Successful keys: Merge to output
- Retry failed keys once with enhanced prompt
- If retry fails: Keep original, mark as untranslatable

### 8. Backup Strategy

**Before Translation:**
- Copy original to `<name>.orig`
- Create timestamped backup directory
- Never overwrite originals

**During Translation:**
- Write to temporary file
- Validate output
- Atomic rename to final location

**After Translation:**
- Generate change report
- Log translation statistics
- Preserve backup for rollback

## Implementation Status

### ✅ Completed
- Format handler framework
- JSON and INI handlers
- File scanner with exclusion rules
- Game profile system (RimWorld, Factorio, Stardew)
- Enhanced protector (ICU, Mustache, RichText)
- Validation system with error codes
- Encoding preservation

### 🚧 In Progress
- XML handler (SAX parser needed)
- YAML handler
- PO handler
- CSV handler with configurable columns
- Lua string literal extraction

### 📋 Planned
- CLI interface with all options
- Streaming for 20MB+ files
- Batch translation queue
- Progress reporting
- Resume capability for interrupted jobs
- Rate limiting integration
- Terminology enforcement

## Usage Example

```rust
use mod_translator_core::{
    scanner::{FileScanner, ScanConfig},
    profiles::GameProfile,
    formats::{FileFormat, get_handler},
    encoding::FileMetadata,
    protector::Protector,
    validator::Validator,
};

// 1. Scan mod directory
let scanner = FileScanner::new(ScanConfig::default());
let files = scanner.scan(&mod_path)?;

// 2. Detect game
let profile = GameProfile::detect(&mod_path)
    .unwrap_or_else(|| GameProfile::generic());

// 3. Process each file
for file in files {
    // Load with encoding detection
    let (content, metadata) = FileMetadata::read_file(&file.path)?;
    
    // Extract translatable entries
    let handler = get_handler(file.format).unwrap();
    let entries = handler.extract(&content)?;
    
    // Protect and translate each entry
    for entry in entries {
        let fragment = Protector::protect(&entry.source);
        let masked = fragment.masked_text();
        
        // ... send to translation API ...
        let translated = translate(masked)?;
        
        // Validate
        let validation = Validator::validate_all(
            &entry.source,
            &fragment,
            &translated
        );
        
        if !validation.passed {
            // Rollback to original
            continue;
        }
        
        // Restore tokens
        let restored = fragment.restore(&translated)?;
        
        // Merge back
        // ...
    }
    
    // Write with original encoding
    FileMetadata::write_file(&file.path, &result, &metadata)?;
}
```

## Error Handling

All operations follow consistent error patterns:

1. **Parse Errors**: Format-specific parse failures → skip file, log error
2. **Validation Errors**: Token mismatch → rollback to original, retry once
3. **API Errors**: Rate limits → exponential backoff, resume
4. **IO Errors**: File write failures → restore from backup, report

## Performance Considerations

- **Memory**: Streaming for files >5MB
- **Concurrency**: Parallel file processing (respecting rate limits)
- **Caching**: Deduplicate identical strings
- **Incremental**: Only translate changed files

## Security

- Never commit secrets
- Validate all file paths (no directory traversal)
- Sandbox binary execution
- Validate output before writing
- Maintain audit logs

## Testing Strategy

- Unit tests for each module
- Format handler round-trip tests
- Protection/restoration tests
- Encoding preservation tests
- Integration tests with real mod samples
- Fuzzing for parser robustness
