//! Read-only volume access inside an isolated, short-lived container.
use rustix::fs::{self, AtFlags, Dir, FileType, Mode, OFlags};
use serde_json::{Value, json};
use std::{
    fs::File,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

const MAX_ENTRIES: usize = 1000;
const MAX_PAYLOAD: usize = 1024 * 1024;
const OPEN: OFlags = OFlags::RDONLY
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC)
    .union(OFlags::NONBLOCK);

fn invalid(code: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, code)
}

fn validate(path: &str) -> io::Result<()> {
    if path.len() > 4096
        || !path.starts_with('/')
        || path.contains('\\')
        || path.chars().any(char::is_control)
        || (path != "/"
            && path[1..]
                .split('/')
                .any(|part| matches!(part, "" | "." | "..")))
    {
        return Err(invalid("VolumePathInvalid"));
    }
    Ok(())
}

fn kind(stat: &fs::Stat) -> FileType {
    FileType::from_raw_mode(stat.st_mode)
}

fn open_child(parent: &File, name: &str) -> io::Result<File> {
    let file = File::from(fs::openat(parent, name, OPEN, Mode::empty())?);
    if !matches!(
        kind(&fs::fstat(&file)?),
        FileType::RegularFile | FileType::Directory
    ) {
        return Err(invalid("VolumePathInvalid"));
    }
    Ok(file)
}

// Hold each directory descriptor while opening its child. Replacing a path with
// a symlink during a download cannot redirect reads outside the mounted volume.
fn resolve(root: &Path, path: &str) -> io::Result<File> {
    validate(path)?;
    let mut file = File::from(fs::open(root, OPEN | OFlags::DIRECTORY, Mode::empty())?);
    if path != "/" {
        for part in path[1..].split('/') {
            file = open_child(&file, part)?;
        }
    }
    Ok(file)
}

fn type_name(stat: &fs::Stat) -> &'static str {
    match kind(stat) {
        FileType::RegularFile => "file",
        FileType::Directory => "directory",
        FileType::Symlink => "symlink",
        _ => "other",
    }
}

fn names(directory: &File) -> io::Result<impl Iterator<Item = io::Result<String>>> {
    Ok(Dir::read_from(directory)?.filter_map(|entry| match entry {
        Ok(entry) if matches!(entry.file_name().to_bytes(), b"." | b"..") => None,
        Ok(entry) => Some(
            entry
                .file_name()
                .to_str()
                .map(str::to_owned)
                .map_err(|_| invalid("VolumePathInvalid")),
        ),
        Err(error) => Some(Err(error.into())),
    }))
}

fn listing(root: &Path, path: &str, limit: usize, payload: usize) -> io::Result<Value> {
    let directory = resolve(root, path)?;
    let mut entries = Vec::new();
    let mut truncated = false;
    for name in names(&directory)? {
        if entries.len() >= limit.min(MAX_ENTRIES) {
            truncated = true;
            break;
        }
        let name = name?;
        let stat = fs::statat(&directory, name.as_str(), AtFlags::SYMLINK_NOFOLLOW)?;
        let target = if kind(&stat) == FileType::Symlink {
            Some(
                fs::readlinkat(&directory, name.as_str(), Vec::new())?
                    .to_string_lossy()
                    .into_owned(),
            )
        } else {
            None
        };
        entries.push(json!({
            "Name": name,
            "Path": format!("{}/{}", path.trim_end_matches('/'), name),
            "Type": type_name(&stat),
            "Size": if kind(&stat) == FileType::RegularFile { Some(stat.st_size.max(0) as u64) } else { None },
            "ModifiedAt": chrono::DateTime::from_timestamp(stat.st_mtime, stat.st_mtime_nsec as u32),
            "LinkTarget": target,
        }));
    }
    let value = json!({"Path": path, "Entries": entries, "IsTruncated": truncated});
    if serde_json::to_vec(&value)?.len() > payload.min(MAX_PAYLOAD) {
        return Err(invalid("VolumeDirectoryListingTooLarge"));
    }
    Ok(value)
}

fn inspect(root: &Path, path: &str) -> io::Result<Value> {
    let file = resolve(root, path)?;
    Ok(json!({"Exists": true, "Type": type_name(&fs::fstat(&file)?)}))
}

fn stream_file(root: &Path, path: &str, writer: &mut impl Write) -> io::Result<()> {
    let mut file = resolve(root, path)?;
    let stat = fs::fstat(&file)?;
    if kind(&stat) != FileType::RegularFile {
        return Err(invalid("VolumePathInvalid"));
    }
    // A file growing during the download must not make the stream unbounded.
    io::copy(&mut (&mut file).take(stat.st_size.max(0) as u64), writer)?;
    Ok(())
}

fn archive<W: Write>(
    builder: &mut tar::Builder<W>,
    directory: &File,
    path: &Path,
    depth: usize,
    remaining: &mut usize,
) -> io::Result<()> {
    if depth > 128 {
        return Err(invalid("VolumeDirectoryListingTooLarge"));
    }
    for name in names(directory)? {
        let name = name?;
        *remaining = remaining
            .checked_sub(1)
            .ok_or_else(|| invalid("VolumeDirectoryListingTooLarge"))?;
        let stat = fs::statat(directory, name.as_str(), AtFlags::SYMLINK_NOFOLLOW)?;
        let child_path = path.join(&name);
        if kind(&stat) == FileType::Symlink {
            let target = fs::readlinkat(directory, name.as_str(), Vec::new())?;
            let mut header = header(&stat, tar::EntryType::Symlink, 0);
            builder.append_link(
                &mut header,
                child_path,
                Path::new(std::ffi::OsStr::from_bytes(target.to_bytes())),
            )?;
        } else if matches!(kind(&stat), FileType::Directory | FileType::RegularFile) {
            let mut child = open_child(directory, &name)?;
            // Use metadata from the opened descriptor, not the earlier directory
            // entry: a concurrent writer may have replaced that entry.
            let stat = fs::fstat(&child)?;
            if kind(&stat) == FileType::Directory {
                builder.append_data(
                    &mut header(&stat, tar::EntryType::Directory, 0),
                    &child_path,
                    io::empty(),
                )?;
                archive(builder, &child, &child_path, depth + 1, remaining)?;
            } else {
                let size = stat.st_size.max(0) as u64;
                builder.append_data(
                    &mut header(&stat, tar::EntryType::Regular, size),
                    child_path,
                    (&mut child).take(size),
                )?;
            }
        }
    }
    Ok(())
}

use std::os::unix::ffi::OsStrExt;

fn header(stat: &fs::Stat, kind: tar::EntryType, size: u64) -> tar::Header {
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(kind);
    header.set_size(size);
    header.set_mode(if kind.is_dir() { 0o500 } else { 0o400 });
    header.set_mtime(stat.st_mtime.max(0) as u64);
    header
}

fn stream_directory(root: &Path, path: &str, writer: &mut impl Write) -> io::Result<()> {
    if path == "/" {
        return Err(invalid("VolumePathInvalid"));
    }
    let directory = resolve(root, path)?;
    let stat = fs::fstat(&directory)?;
    if kind(&stat) != FileType::Directory {
        return Err(invalid("VolumePathIsNotDirectory"));
    }
    let name = Path::new(path)
        .file_name()
        .ok_or_else(|| invalid("VolumePathInvalid"))?;
    let mut builder = tar::Builder::new(writer);
    builder.append_data(
        &mut header(&stat, tar::EntryType::Directory, 0),
        name,
        io::empty(),
    )?;
    archive(&mut builder, &directory, Path::new(name), 0, &mut 100_000)?;
    builder.finish()
}

fn error_code(error: &io::Error) -> &'static str {
    match error
        .raw_os_error()
        .map(rustix::io::Errno::from_raw_os_error)
    {
        Some(rustix::io::Errno::NOENT) => "VolumePathNotFound",
        Some(rustix::io::Errno::LOOP) => "VolumePathIsSymlink",
        Some(rustix::io::Errno::NOTDIR) => "VolumePathIsNotDirectory",
        _ if error.kind() == io::ErrorKind::InvalidInput => match error.to_string().as_str() {
            "VolumeDirectoryListingTooLarge" => "VolumeDirectoryListingTooLarge",
            "VolumePathIsNotDirectory" => "VolumePathIsNotDirectory",
            _ => "VolumePathInvalid",
        },
        _ => "VolumePathUnavailable",
    }
}

fn run() -> io::Result<()> {
    let mut args = std::env::args().skip(1);
    let operation = args.next().unwrap_or_default();
    let operation = if operation == "volume-helper" {
        args.next().unwrap_or_default()
    } else {
        operation
    };
    if operation == "idle" {
        std::thread::sleep(Duration::from_secs(31 * 60));
        return Ok(());
    }
    let mut root = PathBuf::from("/data");
    let mut path = "/".to_owned();
    let mut limit = MAX_ENTRIES;
    let mut payload = MAX_PAYLOAD;
    while let Some(flag) = args.next() {
        let value = args.next().ok_or_else(|| invalid("VolumePathInvalid"))?;
        match flag.as_str() {
            "--root" => root = value.into(),
            "--path" => path = value,
            "--max-entries" => limit = value.parse().map_err(|_| invalid("VolumePathInvalid"))?,
            "--max-payload-bytes" => {
                payload = value.parse().map_err(|_| invalid("VolumePathInvalid"))?
            }
            _ => return Err(invalid("VolumePathInvalid")),
        }
    }
    let mut stdout = io::stdout().lock();
    match operation.as_str() {
        "list" | "inspect" => {
            let result = if operation == "list" {
                listing(&root, &path, limit, payload)
            } else {
                inspect(&root, &path)
            };
            let value = result.unwrap_or_else(|error| {
                json!({
                    "Path": path, "Entries": [], "IsTruncated": false,
                    "Exists": false, "Type": "other", "ErrorCode": error_code(&error),
                })
            });
            serde_json::to_writer(&mut stdout, &value)?;
        }
        "stream-file" => stream_file(&root, &path, &mut stdout)?,
        "stream-directory" => stream_directory(&root, &path, &mut stdout)?,
        _ => return Err(invalid("VolumePathInvalid")),
    }
    stdout.flush()
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{}", error_code(&error));
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests;
