use crate::error::{Result, WorkspaceError};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

/// Atomik yazma: tmp dosyaya yaz → fsync → rename.
/// Aynı dizinde tmp kullanır; çapraz-FS rename riskini önler.
pub fn atomic_write(path: &Path, content: &[u8]) -> Result<()> {
    let parent = path.parent().ok_or(WorkspaceError::InvalidPath)?;
    fs::create_dir_all(parent)?;

    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let tmp = path.with_file_name(format!(
        ".{}.aetheros-write-{}.tmp",
        name,
        std::process::id()
    ));

    let write_result = (|| -> Result<()> {
        let mut f = File::create(&tmp)?;
        f.write_all(content)?;
        f.sync_all()?;
        Ok(())
    })();

    if let Err(e) = write_result {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }

    if let Err(e) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(e.into());
    }
    Ok(())
}
