use crate::utils;
use clap::{Arg, Command};
use std::{
    env,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub fn get_add_command() -> Command {
    Command::new("add")
        .about("Add file to staging")
        .arg(
            Arg::new("paths")
                .required(true)
                .num_args(1..) // Accept 1 or more paths
                .help("Files or directories to add"),
        )
        .arg_required_else_help(true)
}

pub fn add(paths: &[String]) {
    let current_dir: PathBuf = env::current_dir().expect("Directory not found!");
    let path_to_vit: PathBuf = current_dir.join(".vit");
    let mut is_something_updated: bool = false;

    if !path_to_vit.exists() {
        return println!("Version_it repository not initialized!");
    }

    let files_to_add: Vec<String> = utils::expand_paths(paths);
    let mut index_entries: Vec<utils::IndexEntry> = utils::read_index().unwrap();
    let add_deleted_files: bool = paths[0] == ".";

    for file_path in files_to_add {
        match index_entries
            .iter_mut()
            .find(|entry| entry.path == file_path)
        {
            Some(existing_entry) => {
                let is_deleted_file = Path::new(&file_path);
                if !is_deleted_file.exists() {
                    existing_entry.status = utils::FileStatus::Deleted;
                    return;
                }

                // Step 3a: Get file metadata (timestamp, size, etc.)
                let metadata = std::fs::metadata(&file_path).expect("Cannot stat file");

                let mtime = metadata
                    .modified()
                    .unwrap_or(SystemTime::now())
                    .duration_since(UNIX_EPOCH)
                    .unwrap();

                if existing_entry.mtime_secs != mtime.as_secs() as u32 {
                    existing_entry.mtime_nsecs = mtime.as_nanos() as u32;
                    existing_entry.mtime_secs = mtime.as_secs() as u32;

                    let (file_hash, object) = utils::hash_file(&file_path);
                    if existing_entry.sha256 != file_hash {
                        // Mark file as modified
                        existing_entry.status = utils::FileStatus::Modified;
                        existing_entry.sha256 = file_hash;

                        // Store file object
                        let _ = utils::store_object(path_to_vit.as_path(), file_hash, object);
                        is_something_updated = true;
                        println!("Added file: {}", file_path);
                    }
                }
            }
            None => {
                println!("Added file: {}", file_path);
                // Create IndexEntry
                let new_entry = utils::IndexEntry::create(&file_path);
                index_entries.push(new_entry);

                // Hash the file
                let (file_hash, object) = utils::hash_file(&file_path);

                // Store file object
                let _ = utils::store_object(path_to_vit.as_path(), file_hash, object);
                is_something_updated = true;
            }
        }
    }

    if !is_something_updated {
        println!("Everything is up to date");
    }

    if add_deleted_files {
        for entry in index_entries.iter_mut() {
            if !Path::new(entry.path.as_str()).exists() {
                entry.status = utils::FileStatus::Deleted;
            }
        }
    }

    utils::write_index(&index_entries, path_to_vit.join("index").to_str().unwrap()).unwrap();
}
