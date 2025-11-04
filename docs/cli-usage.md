# CLI Usage Guide

## Installation

```bash
cargo build --release
```

The binary will be available at `target/release/mod-translator`.

## Basic Usage

```bash
# Translate a mod directory
mod-translator translate /path/to/mod --target-lang ko

# Use specific game profile
mod-translator translate /path/to/mod --profile rimworld --target-lang ko

# Dry run (preview without changes)
mod-translator translate /path/to/mod --dry-run --target-lang ko
```

## Command Reference

### `translate`

Translate files in a mod directory.

**Options:**

- `--profile <PROFILE>` - Game profile to use
  - `auto` (default): Auto-detect
  - `rimworld`: RimWorld mods
  - `factorio`: Factorio mods
  - `stardew`: Stardew Valley mods
  - `generic`: Generic fallback

- `--target-lang <LANG>` - Target language code (required)
  - `ko`: Korean
  - `ja`: Japanese
  - `zh`: Chinese
  - `es`: Spanish
  - etc.

- `--mode <MODE>` - Validation mode
  - `strict` (default): Fail on any validation error
  - `lenient`: Allow minor issues with warnings

- `--include <PATTERNS>` - File patterns to include
  - `**/*.xml **/*.json`
  - Default: format-specific patterns

- `--exclude <PATTERNS>` - File patterns to exclude
  - `**/*.dll **/*.png`
  - Default: common binaries

- `--encoding <ENCODING>` - Override encoding detection
  - `auto` (default): Auto-detect
  - `utf-8`
  - `utf-16le`
  - `utf-16be`
  - `latin1`

- `--newline <STYLE>` - Override newline style
  - `auto` (default): Auto-detect
  - `lf`: Unix-style (\\n)
  - `crlf`: Windows-style (\\r\\n)

- `--max-retry <N>` - Maximum retry attempts per key (default: 1)

- `--dry-run` - Preview changes without writing files

- `--report <PATH>` - Output detailed report to JSON file

- `--provider <PROVIDER>` - AI provider
  - `gemini`
  - `gpt`
  - `claude`
  - `grok`

- `--api-key <KEY>` - API key for provider (or set env: `TRANSLATOR_API_KEY`)

- `--model <MODEL>` - Specific model to use

## Examples

### RimWorld Mod

```bash
mod-translator translate ~/RimWorld/Mods/MyMod \
  --profile rimworld \
  --target-lang ko \
  --provider gemini \
  --api-key $GEMINI_API_KEY
```

### Factorio Mod

```bash
mod-translator translate ~/Factorio/mods/my-mod \
  --profile factorio \
  --target-lang ja \
  --include "locale/**/*.cfg" \
  --provider gpt \
  --api-key $OPENAI_API_KEY
```

### Custom Patterns

```bash
mod-translator translate /path/to/mod \
  --target-lang ko \
  --include "**/*.json **/*.txt" \
  --exclude "**/test/** **/*.bak" \
  --mode lenient
```

### Dry Run with Report

```bash
mod-translator translate /path/to/mod \
  --target-lang ko \
  --dry-run \
  --report translation-preview.json
```

## Configuration File

Create `.mod-translator.toml` in the mod directory or home directory:

```toml
[default]
profile = "auto"
mode = "strict"
max-retry = 1

[scan]
include = ["**/*.xml", "**/*.json"]
exclude = ["**/*.dll", "**/*.png"]
max-file-size = 20971520  # 20MB

[encoding]
auto-detect = true
preserve-bom = true
preserve-newline = true

[validation]
check-placeholders = true
check-pipe-count = true
max-length-multiplier = 4
warn-on-backticks = true

[profiles.rimworld]
include = ["Languages/**/*.xml", "Defs/**/*.xml"]
exclude = ["Assemblies/**", "Textures/**"]
extra-placeholders = ["{PAWN_*}", "{[A-Z_]+}"]

[profiles.factorio]
include = ["locale/**/*.cfg"]
extra-placeholders = ["__[A-Z_]+__", "%\\d*s"]

[api]
provider = "gemini"
# api-key = "..."  # Better to use env var
model = "gemini-2.5-flash"
rate-limit-rpm = 60
timeout-seconds = 30
```

## Environment Variables

- `TRANSLATOR_API_KEY` - API key for translation provider
- `TRANSLATOR_CONFIG` - Path to config file
- `TRANSLATOR_LOG_LEVEL` - Logging level (trace, debug, info, warn, error)
- `TRANSLATOR_CACHE_DIR` - Cache directory for translations

## Output

### Success

```
✓ Scanned 24 files
✓ Detected profile: RimWorld
✓ Translated 156 entries
  - Successful: 152
  - Failed: 4 (kept original)
✓ Validated all outputs
✓ Wrote 24 files
✓ Backup: /path/to/mod.backup-20250104-120000

Translation complete! Review the changes and test your mod.
```

### Errors

```
✗ Translation failed for: Languages/Korean/Keyed/Misc.xml
  Error: PLACEHOLDER_MISMATCH
  Key: "MissingItem"
  Source: "Cannot find item {0}"
  Translation: "항목을 찾을 수 없습니다"
  Issue: Missing placeholder {0}
  
✗ 4 files failed validation
  See report.json for details

Partial translation saved. 20/24 files succeeded.
Backup available at: /path/to/mod.backup-20250104-120000
```

## Report Format

```json
{
  "summary": {
    "files_scanned": 24,
    "files_translated": 20,
    "files_failed": 4,
    "entries_total": 156,
    "entries_success": 152,
    "entries_failed": 4,
    "tokens_protected": 487,
    "duration_seconds": 45.3
  },
  "profile": {
    "id": "rimworld",
    "name": "RimWorld",
    "detected": true
  },
  "failures": [
    {
      "file": "Languages/Korean/Keyed/Misc.xml",
      "key": "MissingItem",
      "error": "PLACEHOLDER_MISMATCH",
      "details": "Missing placeholder {0}"
    }
  ],
  "warnings": [
    {
      "file": "Defs/Items.xml",
      "key": "LongDescription",
      "warning": "Translation is 5x longer than source"
    }
  ]
}
```

## Troubleshooting

### "Unsupported format"

- Check file extension matches supported formats
- Verify file is not binary (use `--include` to force)

### "PLACEHOLDER_MISMATCH"

- Translation removed or changed placeholders
- Try `--mode lenient` or manually review the entry
- Check game profile has correct placeholder patterns

### "ENCODING_ERROR"

- File may have mixed encodings
- Try `--encoding utf-8` to force
- Check for BOM issues

### "Rate limit exceeded"

- Reduce concurrency or add delays
- Check API quota with provider
- Use `--max-retry` to handle transient errors

### "Binary file skipped"

- Adjust `--binary-threshold` in config
- Use `--include` to force specific files
- Verify file is actually text

## Best Practices

1. **Always backup** before translating (automatic, but verify location)
2. **Test in dry-run** first to preview changes
3. **Use specific profiles** for better accuracy
4. **Review warnings** - they often indicate real issues
5. **Validate in-game** after translation
6. **Keep API keys secure** - use environment variables, not CLI args
7. **Use reports** to track translation quality over time
8. **Start with small mods** to verify setup

## Advanced Usage

### Batch Processing

```bash
#!/bin/bash
for mod in ~/RimWorld/Mods/*; do
  echo "Translating $mod..."
  mod-translator translate "$mod" \
    --target-lang ko \
    --report "reports/$(basename $mod).json"
done
```

### Resume After Interruption

```bash
# The tool automatically resumes from last successful file
mod-translator translate /path/to/mod --target-lang ko
```

### Custom Placeholders

```bash
mod-translator translate /path/to/mod \
  --target-lang ko \
  --extra-placeholder '\{CUSTOM_[A-Z]+\}' \
  --extra-placeholder '<<[^>]+>>'
```

## Getting Help

```bash
# Show all commands
mod-translator --help

# Show command-specific help
mod-translator translate --help

# Show version
mod-translator --version

# Validate config
mod-translator config --validate
```

## Support

- Documentation: `/docs/`
- Issues: GitHub Issues
- Examples: `/examples/`
