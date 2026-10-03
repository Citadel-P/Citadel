use super::*;
use std::{fs as disk, os::unix::fs::symlink};

#[test]
fn lists_metadata_and_enforces_both_limits() {
    let root = tempfile::tempdir().unwrap();
    disk::write(root.path().join("binary"), [0, 255, 1]).unwrap();
    disk::create_dir(root.path().join("folder")).unwrap();
    symlink("binary", root.path().join("link")).unwrap();
    let listing = listing(root.path(), "/", 1000, MAX_PAYLOAD).unwrap();
    let entries = listing["Entries"].as_array().unwrap();
    assert_eq!(entries.len(), 3);
    let file = entries.iter().find(|e| e["Name"] == "binary").unwrap();
    assert_eq!(file["Size"], 3);
    assert_eq!(file["Path"], "/binary");
    assert_eq!(file["Type"], "file");
    assert!(file["ModifiedAt"].as_str().unwrap().contains('T'));
    assert_eq!(
        entries.iter().find(|e| e["Name"] == "link").unwrap()["LinkTarget"],
        "binary"
    );
    assert_eq!(
        super::listing(root.path(), "/", 1, MAX_PAYLOAD).unwrap()["IsTruncated"],
        true
    );
    assert_eq!(
        error_code(&super::listing(root.path(), "/", 1000, 1).unwrap_err()),
        "VolumeDirectoryListingTooLarge"
    );
    assert_eq!(
        error_code(&inspect(root.path(), "/missing").unwrap_err()),
        "VolumePathNotFound"
    );
    assert_eq!(
        error_code(&super::listing(root.path(), "/binary", 1000, MAX_PAYLOAD).unwrap_err()),
        "VolumePathIsNotDirectory"
    );
}

#[test]
fn rejects_traversal_symlinks_and_special_files() {
    let root = tempfile::tempdir().unwrap();
    symlink("/etc", root.path().join("outside")).unwrap();
    for path in [
        "../etc", "/../etc", "/a/../b", "/a//b", "/a/", "/a\\b", "/\0",
    ] {
        assert!(resolve(root.path(), path).is_err(), "{path:?}");
    }
    for path in ["/outside", "/outside/passwd"] {
        assert_eq!(
            error_code(&resolve(root.path(), path).unwrap_err()),
            "VolumePathIsSymlink"
        );
    }
    fs::mknodat(
        fs::CWD,
        root.path().join("fifo"),
        FileType::Fifo,
        Mode::RUSR,
        0,
    )
    .unwrap();
    assert!(resolve(root.path(), "/fifo").is_err());
}

#[test]
fn opened_directory_stays_pinned_when_path_is_replaced() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    disk::create_dir(root.path().join("folder")).unwrap();
    disk::write(root.path().join("folder/file"), "inside").unwrap();
    disk::write(outside.path().join("file"), "outside").unwrap();
    let directory = resolve(root.path(), "/folder").unwrap();
    disk::rename(root.path().join("folder"), root.path().join("moved")).unwrap();
    symlink(outside.path(), root.path().join("folder")).unwrap();
    let mut content = String::new();
    open_child(&directory, "file")
        .unwrap()
        .read_to_string(&mut content)
        .unwrap();
    assert_eq!(content, "inside");
    assert!(resolve(root.path(), "/folder/file").is_err());
}

#[test]
fn streams_binary_files_and_archives_links_without_following_them() {
    let root = tempfile::tempdir().unwrap();
    disk::create_dir(root.path().join("folder")).unwrap();
    disk::write(root.path().join("folder/file"), [0, 255, 10, 13]).unwrap();
    symlink("/etc/passwd", root.path().join("folder/link")).unwrap();
    let mut bytes = Vec::new();
    stream_file(root.path(), "/folder/file", &mut bytes).unwrap();
    assert_eq!(bytes, [0, 255, 10, 13]);
    bytes.clear();
    stream_directory(root.path(), "/folder", &mut bytes).unwrap();
    let mut archive = tar::Archive::new(bytes.as_slice());
    let mut paths = Vec::new();
    for entry in archive.entries().unwrap() {
        let mut entry = entry.unwrap();
        let path = entry.path().unwrap().into_owned();
        if path == Path::new("folder/file") {
            let mut contents = Vec::new();
            entry.read_to_end(&mut contents).unwrap();
            assert_eq!(contents, [0, 255, 10, 13]);
        }
        if path == Path::new("folder/link") {
            assert!(entry.header().entry_type().is_symlink());
            assert_eq!(
                entry.link_name().unwrap().unwrap(),
                Path::new("/etc/passwd")
            );
            assert_eq!(entry.size(), 0);
        }
        paths.push(path);
    }
    paths.sort();
    assert_eq!(
        paths,
        ["folder", "folder/file", "folder/link"].map(PathBuf::from)
    );
    assert!(stream_directory(root.path(), "/", &mut Vec::new()).is_err());
}
