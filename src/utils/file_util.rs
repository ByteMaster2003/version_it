use crate::utils::IndexEntry;
use flate2::{Compression, bufread::ZlibDecoder, write::ZlibEncoder};
use hex;
use ignore::WalkBuilder;
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    fs::File,
    io::{Cursor, Read, Result, Write},
    path::{Path, PathBuf},
};

use super::parse_tree_entries;

pub fn write_index(entries: &[IndexEntry], path: &str) -> Result<()> {
    let mut file = File::create(path)?;

    for entry in entries {
        entry.write(&mut file)?;
    }

    Ok(())
}

pub fn read_index() -> Result<Vec<IndexEntry>> {
    let current_dir: PathBuf = env::current_dir().expect("Directory not found!");
    let path_to_index: PathBuf = current_dir.join(".vit/index");

    let mut file = File::open(&path_to_index)?;
    let mut entries = Vec::new();

    while let Some(entry) = IndexEntry::read(&mut file)? {
        entries.push(entry);
    }

    Ok(entries)
}

pub fn expand_paths(paths: &[String]) -> Vec<String> {
    let mut all_files = Vec::new();

    for path in paths {
        let path_obj = Path::new(path);

        if !path_obj.exists() {
            all_files.push(path.clone());
        }
        if path_obj.is_dir() {
            all_files.extend(list_files_recursively(path_obj));
        } else if path_obj.is_file() {
            all_files.push(path.clone());
        }
    }

    all_files
}

pub fn list_files_recursively(root: &Path) -> Vec<String> {
    let mut files = Vec::new();

    for result in WalkBuilder::new(root)
        .standard_filters(true)
        .add_custom_ignore_filename(".vitignore")
        .build()
    {
        let dir_entry = match result {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        if dir_entry
            .file_type()
            .map(|ft| ft.is_file())
            .unwrap_or(false)
        {
            if let Some(path_str) = dir_entry.path().to_str() {
                files.push(path_str.trim_start_matches("./").to_string());
            }
        }
    }

    files
}

pub fn hash_file(path: &str) -> ([u8; 32], Vec<u8>) {
    let mut hasher = Sha256::new();
    let content = std::fs::read(path).expect("Unable to read file");

    // Create header
    let header = format!("blob {}\0", content.len());
    hasher.update(header.as_bytes());
    hasher.update(&content);

    let sha256_result = hasher.finalize();
    let mut sha256 = [0u8; 32];
    sha256.copy_from_slice(&sha256_result[..]);

    (sha256, content) // Returning hash AND raw content
}

pub fn store_object(git_dir: &Path, sha256: [u8; 32], content: Vec<u8>) -> Result<[u8; 32]> {
    let sha256_hex = hex::encode(sha256);
    let (dir_name, file_name) = sha256_hex.split_at(2);

    let object_dir = git_dir.join("objects").join(dir_name);
    let object_path = object_dir.join(file_name);

    if object_path.exists() {
        return Ok(sha256);
    }

    // Create full_data with header again
    let header = format!("blob {}\0", content.len());
    let mut full_data = header.into_bytes();
    full_data.extend_from_slice(&content);

    // Compress
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&full_data)?;
    let compressed = encoder.finish()?;

    fs::create_dir_all(&object_dir)?;
    fs::write(object_path, compressed)?;

    Ok(sha256)
}

pub fn decompress_file_content(file_path: &Path) -> Result<Vec<u8>> {
    let compressed_data = std::fs::read(file_path)?;

    let mut decoder = ZlibDecoder::new(Cursor::new(compressed_data));
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}

pub fn clear_current_tree(root: &Path) {
    for result in WalkBuilder::new(root)
        .standard_filters(true)
        .add_custom_ignore_filename(".vitignore")
        .build()
    {
        let dir_entry = match result {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        if dir_entry.file_type().unwrap().is_dir() {
            if dir_entry.path().to_str() != root.to_str() {
                fs::remove_dir_all(&dir_entry.path()).unwrap();
            }
        } else {
            fs::remove_file(&dir_entry.path()).unwrap();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Action {
    Create = 0,
    Restore = 1,
    Delete = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FileType {
    Blob = 0,
    Tree = 1,
}

#[derive(Debug)]
pub struct FileChange {
    pub path: String,
    pub file_type: FileType,
    pub action: Action,
    pub sha256: [u8; 32]
}

pub fn calculate_diff(
    current_hash: &str,
    target_hash: &str,
    base_path: &Path,
    list_of_changes: &mut Vec<FileChange>,
) -> Result<()> {
    let current_dir = env::current_dir().unwrap();
    let vit_dir = current_dir.join(".vit");
    let objects_dir = vit_dir.join("objects");

    // Calculate object path
    let current_hash_path = objects_dir
        .join(&current_hash[..2])
        .join(&current_hash[2..]);
    let target_hash_path = objects_dir.join(&target_hash[..2]).join(&target_hash[2..]);

    // Parse Tree Entries
    let current_tree = parse_tree_entries(&current_hash_path).unwrap();
    let target_tree = parse_tree_entries(&target_hash_path).unwrap();

    for tt_entry in &target_tree {
        let name = base_path.join(&tt_entry.name);
        let relative_name = name
            .to_string_lossy()
            .replace(&current_dir.to_string_lossy().to_string(), ".")
            .trim_start_matches("./")
            .to_string();

        if relative_name == "" {
            continue;
        }
        let hash_str = hex::encode(&tt_entry.sha256);
        let file_type = if &tt_entry.mode == "040000" {
            FileType::Tree
        } else {
            FileType::Blob
        };
        let ct_entry = current_tree
            .iter()
            .find(|entry| entry.name == tt_entry.name);
        match ct_entry {
            Some(entry) => {
                if entry.sha256 != tt_entry.sha256 {
                    if file_type == FileType::Blob {
                        list_of_changes.push(FileChange {
                            path: relative_name,
                            file_type,
                            action: Action::Restore,
                            sha256: tt_entry.sha256
                        });
                    } else {
                        calculate_diff(
                            &hex::encode(entry.sha256),
                            &hash_str,
                            &name,
                            list_of_changes,
                        )
                        .unwrap();
                    }
                }
            }
            None => {
                list_of_changes.push(FileChange {
                    path: relative_name,
                    file_type,
                    action: Action::Create,
                    sha256: tt_entry.sha256
                });
            }
        };
    }

    for ct_entry in &current_tree {
        let name = base_path.join(&ct_entry.name);
        let relative_name = name
            .to_string_lossy()
            .replace(&current_dir.to_string_lossy().to_string(), ".")
            .trim_start_matches("./")
            .to_string();

        if relative_name == "" {
            continue;
        }
        let file_type = if &ct_entry.mode == "040000" {
            FileType::Tree
        } else {
            FileType::Blob
        };

        if let None = &target_tree
            .iter()
            .find(|entry| entry.name == ct_entry.name)
        {
            list_of_changes.push(FileChange {
                path: relative_name,
                file_type,
                action: Action::Delete,
                sha256: ct_entry.sha256
            });
        }
    }

    Ok(())
}
