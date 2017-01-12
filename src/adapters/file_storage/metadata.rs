// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

extern crate crypto;
extern crate memmap;
extern crate mime_guess;

use self::crypto::digest::Digest;
use self::crypto::sha1::Sha1;
use self::memmap::{Mmap, Protection};
use self::mime_guess::guess_mime_type;
use std;
use std::fs::File;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum FileKind {
    File,
    Directory,
}

#[derive(Clone, Debug, Serialize)]
pub struct FileMetadata {
    pub path: PathBuf,
    pub mime: String,
    pub size: u64,
    pub kind: FileKind,
    pub hash: String, // TODO: switch to a byte array?
}

// Unused for now, but may allow finer tracking of file changes.
#[allow(dead_code)]
fn get_file_hash(path: &Path) -> Result<String, std::io::Error> {
    let mut algo = Sha1::new();
    let file = Mmap::open_path(&path, Protection::Read)?;
    let bytes: &[u8] = unsafe { file.as_slice() };
    algo.reset();
    algo.input(bytes);

    Ok(algo.result_str())
}

pub fn get_path_hash(path: &Path) -> String {
    let mut algo = Sha1::new();
    let s = format!("{}", path.display());
    algo.reset();
    algo.input(s.as_bytes());

    algo.result_str()
}

pub fn get_file_content(path: &Path) -> Result<memmap::MmapView, std::io::Error> {
    let file = Mmap::open_path(&path, Protection::Read)?;

    Ok(file.into_view())
}

pub fn get_file_metadata(path: &Path) -> Result<FileMetadata, std::io::Error> {
    let file = File::open(&path)?;
    let fmeta = file.metadata()?;

    let kind = if fmeta.is_dir() {
        FileKind::Directory
    } else {
        FileKind::File
    };

    let hash = get_path_hash(path);

    let mut mpath = PathBuf::new();
    mpath.push(path);

    Ok(FileMetadata {
        path: mpath,
        mime: format!("{}", guess_mime_type(path)),
        size: fmeta.len(),
        kind: kind,
        hash: hash,
    })
}

#[test]
fn test_unknown_file_hash() {
    let hash = get_file_hash(Path::new("./unknown.file"));
    assert!(hash.is_err());
}

#[test]
fn test_valid_file_hash() {
    let hash = get_file_hash(Path::new("./package.json")).unwrap();
    assert_eq!(hash, "103bfbff447830258032deedfc53d73ec87543c4");
}

#[test]
fn test_unknown_file_metadata() {
    let meta = get_file_metadata(Path::new("./unknown.file"));
    assert!(meta.is_err());
}

#[test]
fn test_valid_file_metadata() {
    let path = Path::new("./package.json");
    let meta = get_file_metadata(path).unwrap();
    assert_eq!(meta.path, path);
    assert_eq!(meta.mime, "application/json");
    assert_eq!(meta.size, 1557);
    assert_eq!(meta.kind, FileKind::File);
    assert_eq!(meta.hash,
               "9d106f6bdf655116e339da03a9b5609276c92191".to_owned());
}

#[test]
fn test_valid_directory_metadata() {
    let path = Path::new("./src");
    let meta = get_file_metadata(path).unwrap();
    assert_eq!(meta.path, path);
    assert_eq!(meta.mime, "application/octet-stream");
    assert_eq!(meta.size, 4096);
    assert_eq!(meta.kind, FileKind::Directory);
    assert_eq!(meta.hash,
               "07901fccf6a039e8ea7ac1a1fecb3a710125e149".to_owned());
}