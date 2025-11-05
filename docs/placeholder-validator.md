# Enhanced Placeholder Validator

This document describes the enhanced placeholder validator system for XML translation, designed to preserve protected tokens and format placeholders during translation.

## Overview

The placeholder validator ensures that special tokens are preserved during translation:
- **Protected tokens**: `⟦MT:TAG:0⟧`, `⟦MT:CODE:1⟧`, etc.
- **Format tokens**: `{0}`, `{1}`, etc.
- **Combined patterns**: `{0}%`, `{1}%`, etc.

## Architecture

### Core Components

1. **PlaceholderValidator** (`core/src/placeholder_validator.rs`)
   - Main validation engine
   - Auto-recovery mechanisms
   - Failure report generation

2. **Configuration** (`core/src/config.rs`)
   - Validator options
   - UI options
   - JSON/YAML serialization

3. **UI Components** (`apps/desktop/src/ui/ValidationFailureCard.tsx`)
   - Failure visualization
   - Token diff display
   - Download/export functionality

## Validation Pipeline

```
1. Pre-processing (before translation)
   ├─ Extract text nodes from XML
   ├─ Replace tags/code with protected tokens
   └─ Detect format tokens

2. Translation
   └─ LLM translates masked text

3. Post-processing (validation)
   ├─ Parse tokens from translated text
   ├─ Compare multisets (counts)
   └─ Check order preservation

4. Auto-recovery (if validation fails)
   ├─ Step 1: Reinject missing tokens
   ├─ Step 2: Balance opening/closing pairs
   ├─ Step 3: Remove excess tokens
   ├─ Step 4: Correct format tokens
   └─ Step 5: Preserve {n}% patterns

5. Partial Retry (if auto-recovery fails)
   ├─ Retry with enhanced prompt
   └─ Max 1 retry attempt

6. Failure Report (if all else fails)
   └─ Generate detailed report for UI
```

## Error Codes

| Code | Description |
|------|-------------|
| `PLACEHOLDER_MISMATCH` | Token counts don't match |
| `PAIR_UNBALANCED` | Opening/closing tags unbalanced |
| `FORMAT_TOKEN_MISSING` | Format token {n} missing |
| `XML_MALFORMED_AFTER_RESTORE` | XML structure broken after token restoration |
| `RETRY_FAILED` | Partial retry failed |

## Recovery Steps

| Step | Description |
|------|-------------|
| `REINJECT_MISSING_PROTECTED` | Adds missing protected tokens at relative positions |
| `PAIR_BALANCE_CHECK` | Ensures opening/closing tags are balanced |
| `REMOVE_EXCESS_TOKENS` | Removes tokens that shouldn't be there |
| `CORRECT_FORMAT_TOKENS` | Adds missing format tokens |
| `PRESERVE_PERCENT_BINDING` | Ensures {n}% patterns stay together |

## Configuration

### Default Configuration

```json
{
  "validator": {
    "enableAutofix": true,
    "retryOnFail": true,
    "retryLimit": 1,
    "strictPairing": true,
    "preservePercentBinding": true,
    "reportDownload": true,
    "jsonlLogging": true
  },
  "ui": {
    "showFailedLine": true,
    "allowClipboardCopy": true,
    "allowAutofixRetry": true
  }
}
```

### Configuration Options

#### Validator Options

- **enableAutofix**: Enable automatic recovery mechanisms
- **retryOnFail**: Allow partial retries when auto-recovery fails
- **retryLimit**: Maximum number of retries (default: 1)
- **strictPairing**: Enforce strict opening/closing tag pairing
- **preservePercentBinding**: Keep {n}% patterns together
- **reportDownload**: Allow downloading failure reports
- **jsonlLogging**: Enable JSONL logging of failures

#### UI Options

- **showFailedLine**: Show failed lines in UI
- **allowClipboardCopy**: Enable copy-to-clipboard buttons
- **allowAutofixRetry**: Enable retry button in UI

## Usage Example

### Rust

```rust
use mod_translator_core::{PlaceholderValidator, Segment, ValidatorConfig};

// Create validator with default config
let validator = PlaceholderValidator::with_default_config();

// Create segment
let segment = Segment::new(
    "Settings.xml".to_string(),
    32,
    "WBR.HookupRateTip".to_string(),
    "<WBR.HookupRateTip>Text</WBR.HookupRateTip>".to_string(),
    "⟦MT:TAG:0⟧Text⟦MT:TAG:1⟧".to_string(),
);

// Validate translated text
match validator.validate(&segment, translated_text) {
    Ok(recovered) => {
        // Validation passed or was auto-recovered
        println!("Success: {}", recovered);
    }
    Err(report) => {
        // Validation failed
        println!("Failed: {:?}", report.code);
        println!("Expected: {:?}", report.expected_protected);
        println!("Found: {:?}", report.found_protected);
    }
}
```

### TypeScript/React

```typescript
import { ValidationFailureList } from './ui/ValidationFailureCard'
import type { ValidationFailureReport } from './types/core'

function MyComponent() {
  const [failures, setFailures] = useState<ValidationFailureReport[]>([])

  const handleRetry = (report: ValidationFailureReport) => {
    // Trigger retry with auto-recovery
    console.log('Retrying:', report.key)
  }

  const handleDownloadJson = () => {
    const json = JSON.stringify(failures, null, 2)
    // Download logic
  }

  return (
    <ValidationFailureList
      failures={failures}
      onRetry={handleRetry}
      onDownloadJson={handleDownloadJson}
    />
  )
}
```

## Test Cases

The validator includes comprehensive tests:

1. **Simple token omission**: Missing tokens are reinjected
2. **Format token with percent**: `{0}%` patterns are preserved
3. **Pair imbalance**: Opening/closing tags are balanced
4. **Order swap**: Token order mismatches are detected
5. **Example from issue**: "Relative frequency for hookups … entirely."

## Integration

The validator is integrated into the translation pipeline at `core/src/jobs.rs`:

```rust
// After LLM translation
let placeholder_validator = PlaceholderValidator::with_default_config();
let validator_segment = ValidatorSegment::new(/* ... */);

match placeholder_validator.validate(&validator_segment, &translated) {
    Ok(recovered) => {
        // Use recovered text
    }
    Err(failure_report) => {
        // Log failure and rollback
        warn!("Placeholder validation failed: {:?}", failure_report.code);
        // Failure report can be sent to UI
    }
}
```

## Future Enhancements

1. **Machine Learning**: Learn common token patterns
2. **Context-aware recovery**: Use surrounding context for better recovery
3. **Batch validation**: Validate multiple segments at once
4. **Custom rules**: Allow users to define custom validation rules
5. **Metrics dashboard**: Real-time validation metrics

## Performance Considerations

- Validation runs in-line with translation (adds ~1-5ms per segment)
- Auto-recovery typically completes in <10ms
- Failure reports are generated on-demand
- JSONL logging is buffered for efficiency

## Logging

Failures are logged in JSONL format:

```jsonl
{"timestamp":"2025-11-05T06:54:35Z","code":"PLACEHOLDER_MISMATCH","file":"Settings.xml","line":32,"key":"WBR.HookupRateTip"}
```

Log files are stored in the application data directory and can be downloaded from the UI.
