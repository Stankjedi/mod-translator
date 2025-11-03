use chrono::Local;
use serde::Serialize;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BackupOutcome {
    pub backup_path: Option<PathBuf>,
    pub temporary_path: PathBuf,
    pub final_path: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum BackupError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("백업 파일을 생성하지 못했습니다: {0}")]
    BackupCreate(String),
}

pub fn backup_and_swap(target: &Path, contents: &[u8]) -> Result<BackupOutcome, BackupError> {
    let parent = target
        .parent()
        .ok_or_else(|| BackupError::BackupCreate("대상 경로의 상위 디렉터리가 없습니다.".into()))?;
    fs::create_dir_all(parent)?;

    let backup_path = if target.exists() {
        let timestamp = Local::now().format("%Y%m%d%H%M%S");
        let mut candidate = target.with_extension(format!(
            "{}{}",
            target
                .extension()
                .map(|ext| format!("{}.bak", ext.to_string_lossy()))
                .unwrap_or_else(|| "bak".into()),
            format!(".{timestamp}")
        ));

        if !candidate.starts_with(parent) {
            candidate = parent.join(target.file_name().unwrap_or_default());
            candidate.set_extension(format!("bak.{timestamp}"));
        }

        fs::copy(target, &candidate).map_err(|err| BackupError::BackupCreate(err.to_string()))?;
        Some(candidate)
    } else {
        None
    };

    let temp_path = build_temp_path(target);
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&temp_path)?;
    file.write_all(contents)?;
    file.sync_all()?;
    drop(file);

    #[cfg(target_os = "windows")]
    {
        use std::io::ErrorKind;
        if let Err(err) = fs::rename(&temp_path, target) {
            if err.kind() == ErrorKind::AlreadyExists {
                fs::remove_file(target)?;
                fs::rename(&temp_path, target)?;
            } else {
                return Err(BackupError::Io(err));
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        fs::rename(&temp_path, target)?;
    }

    Ok(BackupOutcome {
        backup_path,
        temporary_path: temp_path,
        final_path: target.to_path_buf(),
    })
}

fn build_temp_path(target: &Path) -> PathBuf {
    let mut temp = target.to_path_buf();
    let pid = std::process::id();
    let suffix = format!("__tmp__pid_{}", pid);
    match temp.file_name() {
        Some(name) => {
            let mut os_string = name.to_os_string();
            os_string.push(suffix);
            temp.set_file_name(os_string);
        }
        None => {
            temp.push(format!("temp_{pid}"));
        }
    }
    temp
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use tempfile::tempdir;

    #[test]
    fn writes_backup_and_swaps() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("file.txt");
        fs::write(&target, b"original").unwrap();
        let outcome = backup_and_swap(&target, b"translated").unwrap();
        assert!(outcome.backup_path.is_some());
        let mut file = File::open(&target).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        assert_eq!(contents, "translated");
    }
}
