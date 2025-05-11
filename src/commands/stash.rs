use std::{
    env,
    fs::{self, OpenOptions},
    io::Read,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    commands::restore_file,
    utils::{
        self, FileChange, FileStatus, TreeEntry, build_commit, decompress_file_content,
        parse_tree_entries, read_commit_file, save_tree_object, write_log_entry,
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
    let current_dir = env::current_dir().unwrap();
    let vit_dir = current_dir.join(".vit");
    if !vit_dir.exists() {
        return eprintln!("vit repository not initialized!");
    }

    let mut index_entries: Vec<utils::IndexEntry> = utils::read_index().unwrap();
    let mut list_of_files: Vec<String> = Vec::new();
    let mut deleted_files: Vec<FileChange> = Vec::new();
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
                            list_of_files.push(file_path.clone());

                            let tree_entry = TreeEntry {
                                mode: existing_entry.mode.to_string(),
                                name: file_path.clone(),
                                sha256: file_hash,
                            };
                            stash_content.extend_from_slice(&tree_entry.to_bytes());

                            // Hash the file
                            let (file_hash, file_content) = utils::hash_file(&file_path);

                            // Store file object
                            utils::store_object(&vit_dir, file_hash, file_content).unwrap();
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

    // Restore all deleted files
    for change in deleted_files {
        restore_file(&change, &current_dir, &objects_path, &mut index_entries);
    }

    if list_of_files.is_empty() {
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
}

pub fn apply(index: usize) {
    let current_dir = env::current_dir().unwrap();
    let vit_dir = current_dir.join(".vit");
    if !vit_dir.exists() {
        return eprintln!("vit repository not initialized!");
    }

    let objects_path = vit_dir.join("objects");
    let stash_path = vit_dir.join("logs/refs/stash");

    if !stash_path.exists() {
        return;
    }

    let logs_data = fs::read_to_string(&stash_path).unwrap();
    let lines: Vec<&str> = logs_data.lines().rev().collect();

    if index >= lines.len() {
        return eprintln!("Invalid stash index!");
    }

    let stash_log = lines[index];
    let parts: Vec<&str> = stash_log.splitn(8, ' ').collect();
    let current_commit_hash = parts[1].to_string();
    let current_commit_path = objects_path
        .join(&current_commit_hash[..2])
        .join(&current_commit_hash[2..]);

    let stash_entry = read_commit_file(&current_commit_path).unwrap();
    let stash_tree_hash = hex::encode(&stash_entry.tree);
    let current_tree_hash = objects_path
        .join(&stash_tree_hash[..2])
        .join(&stash_tree_hash[2..]);

    // Restore stashed files
    let tree_entries = parse_tree_entries(&current_tree_hash).unwrap();
    for entry in tree_entries {
        let name = current_dir.join(&entry.name);
        let hash_str = hex::encode(&entry.sha256);

        let blob_path = objects_path.join(&hash_str[..2]).join(&hash_str[2..]);
        let decompressed = decompress_file_content(&blob_path).unwrap();
        let blob_data = &decompressed[decompressed.iter().position(|&b| b == 0).unwrap() + 1..];

        // Make sure all parent directories are present
        fs::create_dir_all(name.parent().unwrap()).unwrap();

        // Write file data
        fs::write(&name, blob_data).unwrap();
    }
}

pub fn pop() {
    let current_dir = env::current_dir().unwrap();
    let vit_dir = current_dir.join(".vit");
    if !vit_dir.exists() {
        return eprintln!("vit repository not initialized!");
    }

    let objects_path = vit_dir.join("objects");
    let stash_ref = vit_dir.join("refs/stash");
    let stash_path = vit_dir.join("logs/refs/stash");

    if !stash_path.exists() {
        return;
    }

    let logs_data = fs::read_to_string(&stash_path).unwrap();
    let mut lines: Vec<&str> = logs_data.lines().collect();

    let current_stash = lines.pop().unwrap();
    let parts: Vec<&str> = current_stash.splitn(8, ' ').collect();
    let current_commit_hash = parts[1].to_string();
    let current_commit_path = objects_path
        .join(&current_commit_hash[..2])
        .join(&current_commit_hash[2..]);

    let stash_entry = read_commit_file(&current_commit_path).unwrap();
    let stash_tree_hash = hex::encode(&stash_entry.tree);
    let current_tree_hash = objects_path
        .join(&stash_tree_hash[..2])
        .join(&stash_tree_hash[2..]);

    // Restore stashed files
    let tree_entries = parse_tree_entries(&current_tree_hash).unwrap();
    for entry in tree_entries {
        let name = current_dir.join(&entry.name);
        let hash_str = hex::encode(&entry.sha256);

        let blob_path = objects_path.join(&hash_str[..2]).join(&hash_str[2..]);
        let decompressed = decompress_file_content(&blob_path).unwrap();
        let blob_data = &decompressed[decompressed.iter().position(|&b| b == 0).unwrap() + 1..];

        // Make sure all parent directories are present
        fs::create_dir_all(name.parent().unwrap()).unwrap();

        // Write file data
        fs::write(&name, blob_data).unwrap();
        fs::remove_file(blob_path).unwrap();
    }

    // Remove tree entry
    fs::remove_file(current_tree_hash).unwrap();

    // Remove stash commit entry
    fs::remove_file(current_commit_path).unwrap();

    if lines.is_empty() {
        // Remove stash head reference
        if stash_ref.exists() {
            fs::remove_file(&stash_ref).unwrap();
        }

        // Remove stash head reference
        if stash_path.exists() {
            fs::remove_file(&stash_path).unwrap();
        }
    } else {
        // Update stash head reference
        let next_stash = lines.pop().unwrap();
        let next_parts: Vec<&str> = next_stash.splitn(8, ' ').collect();
        let current_commit_hash = next_parts[1].to_string();

        fs::write(&stash_ref, current_commit_hash).unwrap();
        delete_last_line(&stash_path).unwrap();
    }
}

pub fn list() {
    let current_dir = env::current_dir().unwrap();
    let vit_dir = current_dir.join(".vit");
    if !vit_dir.exists() {
        return eprintln!("vit repository not initialized!");
    }

    let stash_path = vit_dir.join("logs/refs/stash");

    if !stash_path.exists() {
        return;
    }

    let logs_data = fs::read_to_string(stash_path).unwrap();
    let lines: Vec<&str> = logs_data.lines().collect();

    for (i, line) in lines.iter().rev().enumerate() {
        let parts: Vec<&str> = line.splitn(8, ' ').collect();
        if parts.len() < 8 {
            continue;
        }

        let message = parts[7].trim();
        let hash = parts[1].to_string();
        println!("stash@{{{}}}: {}: {}", i, &hash[..8], message);
    }
}

pub fn clear() {
    let current_dir = env::current_dir().unwrap();
    let vit_dir = current_dir.join(".vit");
    if !vit_dir.exists() {
        return eprintln!("vit repository not initialized!");
    }

    let objects_path = vit_dir.join("objects");
    let stash_ref = vit_dir.join("refs/stash");
    let stash_path = vit_dir.join("logs/refs/stash");

    if !stash_path.exists() {
        return;
    }

    let logs_data = fs::read_to_string(&stash_path).unwrap();
    let lines: Vec<&str> = logs_data.lines().collect();

    for current_stash in lines {
        let parts: Vec<&str> = current_stash.splitn(8, ' ').collect();
        let current_commit_hash = parts[1].to_string();
        let current_commit_path = objects_path
            .join(&current_commit_hash[..2])
            .join(&current_commit_hash[2..]);

        let stash_entry = read_commit_file(&current_commit_path).unwrap();
        let stash_tree_hash = hex::encode(&stash_entry.tree);
        let current_tree_hash = objects_path
            .join(&stash_tree_hash[..2])
            .join(&stash_tree_hash[2..]);

        // Restore stashed files
        let tree_entries = parse_tree_entries(&current_tree_hash).unwrap();
        for entry in tree_entries {
            let hash_str = hex::encode(&entry.sha256);
            let blob_path = objects_path.join(&hash_str[..2]).join(&hash_str[2..]);

            if blob_path.exists() {
                fs::remove_file(blob_path).unwrap();
            }
        }

        // Remove tree entry
        if current_tree_hash.exists() {
            fs::remove_file(current_tree_hash).unwrap();
        }

        // Remove stash commit entry
        if current_commit_path.exists() {
            fs::remove_file(current_commit_path).unwrap();
        }

        // Remove stash head reference
        if stash_ref.exists() {
            fs::remove_file(&stash_ref).unwrap();
        }

        // Remove stash head reference
        if stash_path.exists() {
            fs::remove_file(&stash_path).unwrap();
        }
    }
}

fn delete_last_line(path: &Path) -> std::io::Result<()> {
    let mut file = OpenOptions::new().read(true).write(true).open(path)?;
    let mut content = Vec::new();
    file.read_to_end(&mut content)?;

    // Find the position of the second-last newline
    if let Some(pos) = content.iter().rposition(|&b| b == b'\n') {
        let truncate_pos = if pos == 0 {
            0 // file had only one line
        } else {
            content[..pos]
                .iter()
                .rposition(|&b| b == b'\n')
                .map_or(0, |p| p + 1)
        };
        file.set_len(truncate_pos as u64)?;
    }

    Ok(())
}
