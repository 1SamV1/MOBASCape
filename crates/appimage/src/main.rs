//! `rift-appimage RUNTIME APPDIR OUT`: pack an AppDir into an AppImage.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};

fn main() -> Result<()> {
    let args: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    let [runtime, appdir, out] = args.as_slice() else {
        bail!("usage: rift-appimage RUNTIME APPDIR OUT");
    };
    let runtime_bytes =
        std::fs::read(runtime).with_context(|| format!("reading {}", runtime.display()))?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as u32;
    let offset = rift_appimage::pack(&runtime_bytes, appdir, out, now)?;
    let size = std::fs::metadata(out)?.len();
    println!(
        "wrote {} ({:.1} MiB, squashfs at offset {offset})",
        out.display(),
        size as f64 / (1024.0 * 1024.0)
    );
    Ok(())
}
