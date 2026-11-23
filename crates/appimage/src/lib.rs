//! Minimal AppImage (type 2) packing and running, with no external tools.
//!
//! Layout, as in the AppImage spec: an ELF runtime with the magic bytes
//! `AI\x02` at offset 8, followed directly by a squashfs image of the AppDir.
//! The squashfs starts where the runtime's ELF ends (end of its section
//! header table), so standard tools such as `unsquashfs -o OFFSET` and
//! `--appimage-offset` agree on where it is.
//!
//! The runtime here extracts the image once into a cache directory keyed by
//! the image's contents and runs `AppRun` from there, the same as the
//! reference runtime's extract-and-run mode, so FUSE is not needed.

use std::fs;
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom, Write};
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use backhand::compression::Compressor;
use backhand::{FilesystemCompressor, FilesystemReader, FilesystemWriter, InnerNode, NodeHeader};

/// `AI` plus image type 2, written at ELF offset 8 (`EI_ABIVERSION` and
/// padding, which the kernel ignores).
pub const MAGIC: [u8; 3] = [0x41, 0x49, 0x02];
const MAGIC_OFFSET: usize = 8;

/// Where the appended payload starts: the end of the ELF section header
/// table, which linkers put last in the file.
pub fn elf_end(elf: &[u8]) -> Result<u64> {
    ensure!(elf.len() >= 64, "file too short for an ELF header");
    ensure!(&elf[..4] == b"\x7fELF", "not an ELF file");
    ensure!(elf[4] == 2, "only 64-bit ELF runtimes are supported");
    ensure!(elf[5] == 1, "only little-endian ELF runtimes are supported");
    let u64_at = |at: usize| u64::from_le_bytes(elf[at..at + 8].try_into().unwrap());
    let u16_at = |at: usize| u16::from_le_bytes(elf[at..at + 2].try_into().unwrap()) as u64;
    let sh_offset = u64_at(0x28);
    let sh_entry_size = u16_at(0x3a);
    let sh_count = u16_at(0x3c);
    Ok(sh_offset + sh_entry_size * sh_count)
}

/// True when the ELF carries the type-2 AppImage magic.
pub fn has_magic(elf: &[u8]) -> bool {
    elf.get(MAGIC_OFFSET..MAGIC_OFFSET + 3) == Some(&MAGIC[..])
}

/// FNV-1a over the payload: a cheap content key for the extraction cache.
pub fn content_key(mut reader: impl Read) -> Result<String> {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut buffer = vec![0u8; 1 << 16];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        for &byte in &buffer[..read] {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    Ok(format!("{hash:016x}"))
}

/// Build the squashfs image of `appdir` (gzip, root-owned, permissions and
/// symlinks kept).
pub fn squash(appdir: &Path, mod_time: u32) -> Result<Vec<u8>> {
    ensure!(
        appdir.join("AppRun").is_file(),
        "{} has no AppRun",
        appdir.display()
    );
    let mut writer = FilesystemWriter::default();
    writer.set_compressor(FilesystemCompressor::new(Compressor::Gzip, None)?);
    writer.set_time(mod_time);
    writer.set_root_mode(0o755);
    writer.set_only_root_id();
    let mut entries = Vec::new();
    collect(appdir, appdir, &mut entries)?;
    entries.sort();
    for relative in entries {
        let source = appdir.join(&relative);
        let meta = fs::symlink_metadata(&source)?;
        let header = NodeHeader::new((meta.permissions().mode() & 0o7777) as u16, 0, 0, mod_time);
        let target = Path::new("/").join(&relative);
        if meta.file_type().is_symlink() {
            writer.push_symlink(fs::read_link(&source)?, &target, header)?;
        } else if meta.is_dir() {
            writer.push_dir(&target, header)?;
        } else if meta.is_file() {
            writer.push_file_from_path(source, &target, header)?;
        } else {
            bail!("unsupported file type at {}", source.display());
        }
    }
    let mut image = Cursor::new(Vec::new());
    writer.write(&mut image)?;
    Ok(image.into_inner())
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let path = entry?.path();
        out.push(path.strip_prefix(root)?.to_path_buf());
        if fs::symlink_metadata(&path)?.is_dir() {
            collect(root, &path, out)?;
        }
    }
    Ok(())
}

/// Join `runtime` and the squashed `appdir` into an executable AppImage.
pub fn pack(runtime: &[u8], appdir: &Path, out: &Path, mod_time: u32) -> Result<u64> {
    let end = elf_end(runtime)?;
    ensure!(
        end == runtime.len() as u64,
        "runtime has {} bytes after its section headers; the payload must start at the ELF end",
        runtime.len() as u64 - end.min(runtime.len() as u64)
    );
    let mut head = runtime.to_vec();
    head[MAGIC_OFFSET..MAGIC_OFFSET + 3].copy_from_slice(&MAGIC);
    let image = squash(appdir, mod_time)?;
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    let partial = out.with_extension("part");
    {
        let mut file = fs::File::create(&partial)?;
        file.write_all(&head)?;
        file.write_all(&image)?;
    }
    fs::set_permissions(&partial, fs::Permissions::from_mode(0o755))?;
    fs::rename(&partial, out)?;
    Ok(end)
}

/// Extract the squashfs found at `offset` in `image_path` into `dest`.
pub fn extract(image_path: &Path, offset: u64, dest: &Path) -> Result<()> {
    let file = fs::File::open(image_path)?;
    let reader = FilesystemReader::from_reader_with_offset(BufReader::new(file), offset)
        .with_context(|| format!("reading the squashfs at offset {offset}"))?;
    fs::create_dir_all(dest)?;
    let mut modes = Vec::new();
    for node in reader.files() {
        let relative = node.fullpath.strip_prefix("/").unwrap_or(&node.fullpath);
        if relative.as_os_str().is_empty() {
            continue;
        }
        ensure!(
            relative
                .components()
                .all(|c| matches!(c, std::path::Component::Normal(_))),
            "unsafe path in image: {}",
            node.fullpath.display()
        );
        let path = dest.join(relative);
        let mode = node.header.permissions as u32 & 0o7777;
        match &node.inner {
            InnerNode::Dir(_) => {
                fs::create_dir_all(&path)?;
                modes.push((path, mode));
            }
            InnerNode::File(file) => {
                let mut out = fs::File::create(&path)?;
                std::io::copy(&mut reader.file(file).reader(), &mut out)?;
                fs::set_permissions(&path, fs::Permissions::from_mode(mode))?;
            }
            InnerNode::Symlink(link) => symlink(&link.link, &path)?,
            _ => {}
        }
    }
    // Directory modes last, so a read-only directory is filled first.
    for (path, mode) in modes.into_iter().rev() {
        fs::set_permissions(&path, fs::Permissions::from_mode(mode | 0o700))?;
    }
    Ok(())
}

/// Hash the payload of the AppImage at `path`, starting at `offset`.
pub fn payload_key(path: &Path, offset: u64) -> Result<String> {
    let mut file = fs::File::open(path)?;
    file.seek(SeekFrom::Start(offset))?;
    content_key(BufReader::new(file))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stand-in ELF: a 64-byte header whose section headers (one 64-byte
    /// entry) end exactly at the end of the file.
    fn fake_runtime() -> Vec<u8> {
        let mut elf = vec![0u8; 128];
        elf[..4].copy_from_slice(b"\x7fELF");
        elf[4] = 2;
        elf[5] = 1;
        elf[0x28..0x30].copy_from_slice(&64u64.to_le_bytes());
        elf[0x3a..0x3c].copy_from_slice(&64u16.to_le_bytes());
        elf[0x3c..0x3e].copy_from_slice(&1u16.to_le_bytes());
        elf
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rift-appimage-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_payload_starts_at_the_end_of_the_section_headers() {
        assert_eq!(elf_end(&fake_runtime()).unwrap(), 128);
        assert!(
            elf_end(b"not an elf at all, just some bytes padding it out to 64 bytes...").is_err()
        );
    }

    #[test]
    fn packing_then_extracting_keeps_files_modes_and_symlinks() {
        let root = scratch("roundtrip");
        let appdir = root.join("App.AppDir");
        fs::create_dir_all(appdir.join("usr/bin")).unwrap();
        fs::write(appdir.join("AppRun"), "#!/bin/sh\necho hi\n").unwrap();
        fs::set_permissions(appdir.join("AppRun"), fs::Permissions::from_mode(0o755)).unwrap();
        fs::write(appdir.join("usr/bin/data.txt"), vec![7u8; 300_000]).unwrap();
        symlink("usr/bin/data.txt", appdir.join(".DirIcon")).unwrap();

        let out = root.join("App.AppImage");
        let offset = pack(&fake_runtime(), &appdir, &out, 1_700_000_000).unwrap();
        assert_eq!(offset, 128);
        let bytes = fs::read(&out).unwrap();
        assert!(has_magic(&bytes));
        assert_eq!(&bytes[128..132], b"hsqs");

        let dest = root.join("extracted");
        extract(&out, offset, &dest).unwrap();
        assert_eq!(
            fs::read_to_string(dest.join("AppRun")).unwrap(),
            "#!/bin/sh\necho hi\n"
        );
        let mode = fs::metadata(dest.join("AppRun"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o755);
        assert_eq!(
            fs::read(dest.join("usr/bin/data.txt")).unwrap(),
            vec![7u8; 300_000]
        );
        assert_eq!(
            fs::read_link(dest.join(".DirIcon")).unwrap(),
            Path::new("usr/bin/data.txt")
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_runtime_with_trailing_bytes_is_refused() {
        let root = scratch("trailing");
        let appdir = root.join("A.AppDir");
        fs::create_dir_all(&appdir).unwrap();
        fs::write(appdir.join("AppRun"), "x").unwrap();
        let mut runtime = fake_runtime();
        runtime.extend_from_slice(b"junk");
        assert!(pack(&runtime, &appdir, &root.join("o"), 0).is_err());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_content_key_changes_with_the_content() {
        let a = content_key(&b"payload one"[..]).unwrap();
        let b = content_key(&b"payload two"[..]).unwrap();
        assert_ne!(a, b);
        assert_eq!(a, content_key(&b"payload one"[..]).unwrap());
    }
}
