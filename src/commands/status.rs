use crate::utils;
use clap::Command;
use std::{
    env,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use colored::*;

pub fn get_status_command() -> Command {
    Command::new("status").about("Check the status of changes")
}

pub fn status() {
    let current_dir: PathBuf = env::current_dir().expect("Directory not found!");
    let path_to_vit: PathBuf = current_dir.join(".vit");

    if !path_to_vit.exists() {
        return println!("Version_it repository not initialized!");
    }

    let files_to_add: Vec<String> = utils::expand_paths(&[String::from(".")]);
    let mut index_entries: Vec<utils::IndexEntry> = utils::read_index().unwrap();

    let mut untracked_files: Vec<String> = Vec::new();
    let mut added_files: Vec<String> = Vec::new();
    let mut modified_files: Vec<String> = Vec::new();
    let mut deleted_files: Vec<String> = Vec::new();

    for file_path in files_to_add {
        // Step 3a: Get file metadata (timestamp, size, etc.)
        let metadata = std::fs::metadata(&file_path).expect("Cannot stat file");

        let mtime = metadata
            .modified()
            .unwrap_or(SystemTime::now())
            .duration_since(UNIX_EPOCH)
            .unwrap();

        match index_entries
            .iter_mut()
            .find(|entry| entry.path == file_path)
        {
            Some(existing_entry) => {
                if existing_entry.mtime_secs != mtime.as_secs() as u32 {
                    let (file_hash, _object) = utils::hash_file(&file_path);
                    existing_entry.mtime_nsecs = mtime.as_nanos() as u32;
                    existing_entry.mtime_secs = mtime.as_secs() as u32;

                    if existing_entry.sha256 != file_hash {
                        modified_files.push(file_path.clone());
                    }
                } else {
                    added_files.push(file_path.clone());
                }
            }
            None => {
                untracked_files.push(file_path.clone());
            }
        }
    }

    for entry in index_entries.iter() {
        if !Path::new(entry.path.as_str()).exists() {
            deleted_files.push(entry.path.clone());
        }
    }

    if !added_files.is_empty() {
        println!("Changes to be committed:");
        for file in added_files {
            println!("  {} {}", "modified:".green(), file.green())
        }
        println!();
    }
    if !modified_files.is_empty() {
        println!("Changes not staged for commit:");
        for file in modified_files {
            println!("  {} {}", "modified:".red(), file.red())
        }
        println!();
    }
    if !deleted_files.is_empty() {
        println!("Deleted files not staged for commit:");
        for file in deleted_files {
            println!("  {} {}", "deleted:".red(), file.red())
        }
        println!();
    }
    if !untracked_files.is_empty() {
        println!("Untracked files:");
        for file in untracked_files {
            println!("  {}", file.red())
        }
    }

    utils::write_index(&index_entries, path_to_vit.join("index").to_str().unwrap()).unwrap();
}
