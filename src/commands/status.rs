use crate::utils;
use clap::Command;
use colored::*;
use std::{
    env,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

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
    let index_entries: Vec<utils::IndexEntry> = utils::read_index().unwrap();

    let mut untracked_files: Vec<String> = Vec::new();
    let mut added_files: Vec<String> = Vec::new();
    let mut changed_files: Vec<String> = Vec::new();

    for file_path in files_to_add {
        // Step 3a: Get file metadata (timestamp, size, etc.)
        let metadata = std::fs::metadata(&file_path).expect("Cannot stat file");

        let mtime = metadata
            .modified()
            .unwrap_or(SystemTime::now())
            .duration_since(UNIX_EPOCH)
            .unwrap();

        match index_entries
            .iter()
            .find(|entry| entry.path == file_path)
        {
            Some(existing_entry) => {
                if existing_entry.mtime_secs != mtime.as_secs() as u32 {
                    let (file_hash, _object) = utils::hash_file(&file_path);

                    if existing_entry.sha256 != file_hash {
                        changed_files.push(format!(
                            "  {} {}",
                            "modified:".red(),
                            file_path.clone().red()
                        ));
                    }
                } else {
                    let status = existing_entry.status;
                    let status_message = if status == utils::FileStatus::New {
                        "new file:"
                    } else if status == utils::FileStatus::Modified {
                        "modified:"
                    } else if status == utils::FileStatus::Deleted {
                        "deleted: "
                    } else {
                        "Unchanged"
                    };

                    if status_message != "Unchanged" {
                        added_files.push(format!(
                            "  {} {}",
                            status_message.green(),
                            file_path.clone().green()
                        ));
                    }
                }
            }
            None => {
                untracked_files.push(file_path.clone());
            }
        }
    }

    for entry in index_entries.iter() {
        if !Path::new(entry.path.as_str()).exists() {
            if entry.status != utils::FileStatus::Deleted {
                changed_files.push(format!(
                    "  {} {}",
                    "deleted: ".red(),
                    entry.path.clone().red()
                ));
            } else {
                added_files.push(format!(
                    "  {} {}",
                    "deleted: ".green(),
                    entry.path.clone().green()
                ));
            }
        }
    }

    if added_files.is_empty() && changed_files.is_empty() && untracked_files.is_empty() {
        println!("Everything is up to date on main branch");
    }

    if !added_files.is_empty() {
        println!("Changes to be committed:");
        for file in added_files {
            println!("{}", file)
        }
        println!();
    }
    if !changed_files.is_empty() {
        println!("Changes not staged for commit:");
        for file in changed_files {
            println!("{}", file)
        }
        println!();
    }
    if !untracked_files.is_empty() {
        println!("Untracked files:");
        for file in untracked_files {
            println!("  {}", file.red())
        }
    }

}
