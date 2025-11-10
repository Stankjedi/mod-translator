use std::path::Path;

use anyhow::{bail, Result};

pub fn translate_file(path: &Path, _from: &str, _to: &str) -> Result<()> {
    if !path.exists() {
        bail!("file not found");
    }

    let text = std::fs::read_to_string(path)?;
    let out = text;

    let backup = path.with_extension("bak");
    if !backup.exists() {
        std::fs::copy(path, &backup)?;
    }

    std::fs::write(path, out)?;
    Ok(())
}
