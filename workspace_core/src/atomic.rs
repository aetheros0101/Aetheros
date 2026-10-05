use crate::error::Result;
use std::{fs::{self, OpenOptions}, io::{self, Write}, path::Path};

/// Writes content to a temporary sibling, flushes it, then renames it over the destination.
pub fn atomic_write(path: &Path, content: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
    let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("file");
    let tmp = path.with_file_name(format!(".{file_name}.aetheros-{}.tmp", std::process::id()));
    {
        let mut file = OpenOptions::new().create_new(true).write(true).open(&tmp)?;
        file.write_all(content)?;
        file.flush()?;
        file.sync_all()?;
    }
    match fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(e) => { let _ = fs::remove_file(&tmp); Err(io::Error::new(e.kind(), format!("atomic rename failed: {e}")).into()) }
    }
}

