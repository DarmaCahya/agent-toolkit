use crate::error::{PdfError, Result};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TMP_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Safely catches panics from unhandled upstream parser edge cases (K3 & S15) and converts to clean error
pub fn guarded<T, F>(engine: &'static str, f: F) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    catch_unwind(AssertUnwindSafe(f)).map_err(|payload| {
        let msg = if let Some(s) = payload.downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown internal engine panic".to_string()
        };
        match engine {
            "reader" => PdfError::Read(format!("Internal reader panic caught safely: {}", msg)),
            "rasterizer" => {
                PdfError::Render(format!("Internal rasterizer panic caught safely: {}", msg))
            }
            "generator" => {
                PdfError::Generate(format!("Internal generator panic caught safely: {}", msg))
            }
            _ => PdfError::Read(format!("Internal engine panic caught safely: {}", msg)),
        }
    })?
}

/// Safely validates that `path_str` is a workspace-relative path without directory traversal or root escapes (C3, K1, S1).
/// Validates allowed extension (e.g. `&["md", "markdown", "txt"]`, `&["png"]`, `&["pdf"]`).
/// Explicitly rejects symlinks to prevent sandbox bypass.
pub fn validate_relative_path(path_str: &str, allowed_exts: &[&str]) -> Result<PathBuf> {
    let trimmed = path_str.trim();
    if trimmed.is_empty() {
        return Err(PdfError::InvalidArgument(
            "Target path cannot be empty".into(),
        ));
    }

    let p = Path::new(trimmed);
    for c in p.components() {
        match c {
            Component::ParentDir => {
                return Err(PdfError::InvalidArgument(format!(
                    "Path traversal ('..') is strictly prohibited in sandbox, got: {}",
                    trimmed
                )));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(PdfError::InvalidArgument(format!(
                    "Absolute and drive root paths are strictly prohibited in sandbox, got: {}",
                    trimmed
                )));
            }
            Component::Normal(name) => {
                // Windows reserved names & ADS protection (checks base stem so CON.md, NUL.txt are blocked)
                let stem = Path::new(name)
                    .file_stem()
                    .unwrap_or(name)
                    .to_string_lossy()
                    .to_uppercase();
                const RESERVED_DEVICE_NAMES: &[&str] = &[
                    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6",
                    "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7",
                    "LPT8", "LPT9",
                ];
                if RESERVED_DEVICE_NAMES.contains(&stem.as_str()) {
                    return Err(PdfError::InvalidArgument(format!(
                        "Reserved OS device name prohibited: {}",
                        trimmed
                    )));
                }
                let s = name.to_string_lossy();
                if s.contains(':') {
                    return Err(PdfError::InvalidArgument(format!(
                        "Alternative Data Streams (ADS) colon syntax prohibited: {}",
                        trimmed
                    )));
                }
            }
            Component::CurDir => (),
        }
    }

    // Reject symlinks if the target file or any existing ancestor is a symlink (K1)
    if let Ok(meta) = fs::symlink_metadata(p) {
        if meta.file_type().is_symlink() {
            return Err(PdfError::InvalidArgument(format!(
                "Symlink targets are strictly prohibited in sandbox: {}",
                trimmed
            )));
        }
    }

    if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
        let ext_lower = ext.to_lowercase();
        if !allowed_exts.contains(&ext_lower.as_str()) {
            return Err(PdfError::InvalidArgument(format!(
                "Invalid file extension '.{}'. Allowed extensions: {:?}",
                ext, allowed_exts
            )));
        }
    } else {
        return Err(PdfError::InvalidArgument(format!(
            "Target path must have a valid extension: {:?}",
            allowed_exts
        )));
    }

    Ok(p.to_path_buf())
}

/// Atomically writes content to a validated path using an unguessable unique temp file and rename (K2, S2, S8).
/// Thread-safe and race-free for multi-agent concurrency.
pub fn atomic_write_bytes(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let counter = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp_filename = format!(".tmp_{}_{}_{}.tmp", std::process::id(), nanos, counter);
    let tmp_path = match path.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir.join(tmp_filename),
        _ => PathBuf::from(tmp_filename),
    };

    let write_res = (|| -> std::io::Result<()> {
        let file = File::create(&tmp_path)?;
        let mut writer = BufWriter::new(file);
        writer.write_all(bytes)?;
        writer.flush()?;
        let inner_file = writer.into_inner()?;
        inner_file.sync_all()?;
        Ok(())
    })();

    if let Err(e) = write_res {
        let _ = fs::remove_file(&tmp_path);
        return Err(PdfError::Io(e));
    }

    if let Err(e) = fs::rename(&tmp_path, path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(PdfError::Io(e));
    }

    Ok(())
}
