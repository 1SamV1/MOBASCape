//! AppImage runtime: the ELF at the front of the AppImage.
//!
//! It finds the squashfs appended to itself, extracts it once into
//! `$XDG_CACHE_HOME/rift-appimage/<content key>` (older versions there are
//! removed), and runs `AppRun` with the usual AppImage environment
//! (`APPIMAGE`, `APPDIR`, `OWD`, `ARGV0`).
//!
//! Flags handled here and not passed on:
//! `--appimage-extract [DEST]` (default `./squashfs-root`),
//! `--appimage-offset`, `--appimage-version`.

use std::ffi::OsString;
use std::fs;
use std::io::Read;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

fn main() {
    if let Err(error) = run() {
        eprintln!("AppImage runtime: {error:#}");
        std::process::exit(127);
    }
}

fn run() -> Result<()> {
    let image = fs::read_link("/proc/self/exe").context("locating the AppImage")?;
    let mut head = vec![0u8; 64];
    fs::File::open(&image)?.read_exact(&mut head)?;
    let offset = rift_appimage::elf_end(&head)?;

    let mut args = std::env::args_os();
    let argv0 = args.next().unwrap_or_default();
    let rest: Vec<OsString> = args.collect();
    match rest.first().and_then(|a| a.to_str()) {
        Some("--appimage-offset") => {
            println!("{offset}");
            return Ok(());
        }
        Some("--appimage-version") => {
            println!("rift-appimage runtime {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        Some("--appimage-extract") => {
            let dest = rest
                .get(1)
                .map(PathBuf::from)
                .unwrap_or_else(|| "squashfs-root".into());
            rift_appimage::extract(&image, offset, &dest)?;
            println!("extracted to {}", dest.display());
            return Ok(());
        }
        _ => {}
    }

    let appdir = ensure_extracted(&image, offset)?;
    let owd = std::env::current_dir().unwrap_or_else(|_| "/".into());
    let error = Command::new(appdir.join("AppRun"))
        .args(&rest)
        .env("APPIMAGE", &image)
        .env("APPDIR", &appdir)
        .env("OWD", owd)
        .env("ARGV0", argv0)
        .exec();
    bail!("running AppRun: {error}")
}

fn ensure_extracted(image: &Path, offset: u64) -> Result<PathBuf> {
    let cache_root = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .context("no HOME or XDG_CACHE_HOME")?
        .join("rift-appimage");
    let key = rift_appimage::payload_key(image, offset)?;
    let dir = cache_root.join(&key);
    if dir.join("AppRun").is_file() {
        return Ok(dir);
    }
    fs::create_dir_all(&cache_root)?;
    let staging = cache_root.join(format!(".{key}.{}", std::process::id()));
    let _ = fs::remove_dir_all(&staging);
    rift_appimage::extract(image, offset, &staging)?;
    if fs::rename(&staging, &dir).is_err() {
        // Another launch won the race; use its copy.
        let _ = fs::remove_dir_all(&staging);
        if !dir.join("AppRun").is_file() {
            bail!("could not place the extracted image at {}", dir.display());
        }
    }
    // Drop older extractions: names are exactly 16 hex digits.
    if let Ok(entries) = fs::read_dir(&cache_root) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name != key && name.len() == 16 && name.bytes().all(|b| b.is_ascii_hexdigit()) {
                let _ = fs::remove_dir_all(entry.path());
            }
        }
    }
    Ok(dir)
}
