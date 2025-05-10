use std::{
    env, fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    commands::restore_file,
    utils::{
        self, FileChange, FileStatus, TreeEntry, build_commit, read_commit_file, save_tree_object,
        write_log_entry,
    },
};
use clap::{Arg, Command};

use super::restore_tree;

pub fn get_stash_command() -> Command {
    Command::new("stash")
        .about("Save changes temporary")
        .subcommand_required(false) // Don't require it here
        .arg_required_else_help(false)
        .subcommand(
            Command::new("save")
                .about("Save stash with message")
                .arg(Arg::new("message").required(true))
                .arg_required_else_help(true),
        )
        .subcommand(Command::new("list").about("List stashed changes"))
        .subcommand(Command::new("pop").about("Apply latest stashed changes"))
        .subcommand(Command::new("clear").about("clear stash"))
        .subcommand(
            Command::new("apply")
                .about("Apply specific stashed changes")
                .arg(Arg::new("index").required(true))
                .arg_required_else_help(true),
        )
}

pub fn stash(message: Option<String>) {
    let mut index_entries: Vec<utils::IndexEntry> = utils::read_index().unwrap();
    let mut list_of_files: Vec<String> = Vec::new();
    let mut deleted_files: Vec<FileChange> = Vec::new();
    let current_dir = env::current_dir().unwrap();
    let vit_dir = current_dir.join(".vit");
    let objects_path = vit_dir.join("objects");
    let stash_ref = vit_dir.join("refs/stash");

    let head_ref = fs::read_to_string(vit_dir.join("HEAD")).unwrap();
    let current_branch_ref = head_ref.trim_start_matches("ref: ").trim();
    let commit_ref = vit_dir.join(current_branch_ref);
    let branch_name = commit_ref.file_name().unwrap().to_str().unwrap();

    for entry in &index_entries {
        if entry.status == FileStatus::Modified {
            list_of_files.push(entry.path.clone());
        }
    }

    let current_files: Vec<String> = utils::expand_paths(&[".".to_string()]);
    let mut stash_content: Vec<u8> = Vec::new();

    for file_path in current_files {
        match index_entries
            .iter_mut()
            .find(|entry| entry.path == file_path)
        {
            Some(existing_entry) => {
                if !Path::new(&file_path).exists() {
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

                    let (file_hash, _object) = utils::hash_file(&file_path);
                    if existing_entry.sha256 != file_hash {
                        if !list_of_files.contains(&file_path) {
                            let tree_entry = TreeEntry {
                                mode: existing_entry.mode.to_string(),
                                name: file_path,
                                sha256: file_hash,
                            };
                            stash_content.extend_from_slice(&tree_entry.to_bytes());
                        }
                    }
                }
            }
            None => {}
        }
    }

    for entry in index_entries.iter() {
        if !Path::new(entry.path.as_str()).exists() {
            let file_change = FileChange {
                path: entry.path.clone(),
                file_type: utils::FileType::Blob,
                action: utils::Action::Create,
                sha256: entry.sha256,
            };
            deleted_files.push(file_change);
        }
    }

    if list_of_files.is_empty() && deleted_files.is_empty() {
        println!("No Updates to stash!");
        return;
    };

    let prev_stash_hash = if stash_ref.exists() {
        let prev_hash_str = fs::read_to_string(&stash_ref).unwrap();
        hex::decode(prev_hash_str.trim()).unwrap()
    } else {
        vec![0u8; 32]
    };

    // Save tree object
    let tree_hash = save_tree_object(&stash_content).unwrap();
    let stash_message: String = match message.clone() {
        Some(mes) => mes,
        None => format!("WIP in progress on branch {}", branch_name),
    };
    let stash_hash = build_commit(tree_hash, &prev_stash_hash, &stash_message);

    // Update stash head
    fs::write(stash_ref, hex::encode(stash_hash)).expect("Failed to update reference!");

    // Write Log Entry
    let author_name = "Vivek";
    let author_email = "vivek@example.com";
    let log_entry_path = "refs/stash";
    write_log_entry(
        &prev_stash_hash,
        &stash_hash,
        &author_name,
        &author_email,
        &stash_message,
        log_entry_path,
    );

    // Reset Current Commit
    let head_ref = fs::read_to_string(vit_dir.join("HEAD")).unwrap();
    let current_branch_ref = head_ref.trim_start_matches("ref: ").trim();
    let commit_ref = vit_dir.join(current_branch_ref);
    let current_commit_hash = fs::read_to_string(&commit_ref).unwrap();

    let current_commit_path = objects_path
        .join(&current_commit_hash[..2])
        .join(&current_commit_hash[2..]);

    let commit_entry = read_commit_file(&current_commit_path).unwrap();
    let current_tree_hash = hex::encode(commit_entry.tree);
    let current_tree_path = objects_path
        .join(&current_tree_hash[..2])
        .join(&current_tree_hash[2..]);

    restore_tree(
        &current_tree_path,
        &current_dir,
        &objects_path,
        &mut index_entries,
    );

    // Restore all deleted files
    for change in deleted_files {
        restore_file(&change, &current_dir, &objects_path, &mut index_entries);
    }
}

pub fn apply(_index: u8) {}

pub fn pop() {}

pub fn list() {}

pub fn clear() {}
