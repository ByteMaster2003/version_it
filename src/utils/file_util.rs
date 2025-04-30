use crate::utils::IndexEntry;
use flate2::{Compression, write::ZlibEncoder};
use hex;
use ignore::WalkBuilder;
use sha2::{Digest, Sha256};
use std::{
    env,
    fs::{self, File},
    io::{Result, Write},
    path::{Path, PathBuf},
};

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

        if dir_entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
            if let Some(path_str) = dir_entry.path().to_str() {
                files.push(path_str.to_string());
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
