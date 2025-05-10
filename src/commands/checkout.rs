use crate::utils::{
    self, Action, FileChange, FileStatus, FileType, IndexEntry, calculate_diff,
    decompress_file_content, parse_tree_entries, read_commit_file, read_index, write_index,
};
use clap::{Arg, Command};
use colored::Colorize;
use std::{env, fs, path::Path};

pub fn get_checkout_command() -> Command {
    Command::new("checkout")
        .about("Switch to some other branch")
        .arg(
            Arg::new("name")
                .required(true)
                .help("Name of branch to checkout"),
        )
}

pub fn checkout(name: &str) {
    let current_dir = env::current_dir().unwrap();
    let vit_dir = current_dir.join(".vit");
    if !vit_dir.exists() {
        return println!("Vit not initialized");
    }

    let heads_dir = vit_dir.join("refs/heads");
    let head_path = vit_dir.join("HEAD");
    let head_ref = fs::read_to_string(&head_path).unwrap();
    let current_branch_ref = head_ref.trim_start_matches("ref: ").trim();
    let mut index_entries: Vec<IndexEntry> = read_index().unwrap();

    if Path::new(vit_dir.join(current_branch_ref).as_path())
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string()
        == name
    {
        return println!("{}", "Branch is already Active!".red());
    }

    let current_commit_hash = fs::read_to_string(vit_dir.join(current_branch_ref)).unwrap();
    // Read commit object and get tree hash
    let current_commit_path = vit_dir
        .join("objects")
        .join(&current_commit_hash[..2])
        .join(&current_commit_hash[2..]);
    let current_commit_entry = read_commit_file(&current_commit_path).unwrap();
    let current_tree_hash = hex::encode(current_commit_entry.tree);

    // Get commit hash from current branch
    let commit_hash: String = if heads_dir.join(name).exists() {
        fs::write(&head_path, format!("ref: refs/heads/{}", name)).unwrap();
        fs::read_to_string(heads_dir.join(name)).unwrap()
    } else {
        return println!("Branch does not exist");
    };

    // Read commit object and get tree hash
    let commit_path = vit_dir
        .join("objects")
        .join(&commit_hash[..2])
        .join(&commit_hash[2..]);
    let commit_entry = read_commit_file(&commit_path).unwrap();

    let tree_hash = hex::encode(commit_entry.tree);
    let base_path = env::current_dir().unwrap();
    let mut list_of_changes: Vec<FileChange> = Vec::new();

    calculate_diff(
        &current_tree_hash,
        &tree_hash,
        &base_path,
        &mut list_of_changes,
    )
    .unwrap();

    for change in &list_of_changes {
        match &change.action {
            Action::Delete => {
                delete_files(&change, &current_dir, &mut index_entries);
            }
            Action::Restore => {
                restore_file(
                    &change,
                    &current_dir,
                    &vit_dir.join("objects"),
                    &mut index_entries,
                );
            }
            Action::Create => {
                create_files(
                    &change,
                    &current_dir,
                    &vit_dir.join("objects"),
                    &mut index_entries,
                );
            }
        }
    }

    write_index(&index_entries, vit_dir.join("index").to_str().unwrap()).unwrap();
    println!("Checkout to branch --> {}", &name);
}

pub fn restore_tree(
    tree_path: &Path,
    base_path: &Path,
    objects_path: &Path,
    index_entries: &mut Vec<IndexEntry>,
) {
    let tree_entries = parse_tree_entries(&tree_path).unwrap();

    for entry in tree_entries {
        let mode = entry.mode;
        let name = base_path.join(&entry.name);
        let file_path = name
            .to_string_lossy()
            .to_string()
            .replace(env::current_dir().unwrap().to_str().unwrap(), ".")
            .trim_start_matches("./")
            .to_string();
        let hash = &entry.sha256;
        let hash_str = hex::encode(hash);

        if mode == "040000" {
            fs::create_dir_all(&name).unwrap();

            let sub_tree_path = objects_path.join(&hash_str[..2]).join(&hash_str[2..]);
            let sub_base_path = base_path.join(&name);

            restore_tree(&sub_tree_path, &sub_base_path, objects_path, index_entries);
        } else {
            let blob_path = objects_path.join(&hash_str[..2]).join(&hash_str[2..]);
            let decompressed = decompress_file_content(&blob_path).unwrap();
            let blob_data = &decompressed[decompressed.iter().position(|&b| b == 0).unwrap() + 1..];

            // Make sure all parent directories are present
            fs::create_dir_all(name.parent().unwrap()).unwrap();

            // Write file data
            fs::write(&name, blob_data).unwrap();

            // Update the index entry
            if let Some(i_entry) = index_entries.iter_mut().find(|i| i.path == file_path) {
                i_entry.sha256 = entry.sha256;
                i_entry.status = FileStatus::Unchanged;
            } else {
                let mut new_entry = IndexEntry::create(&file_path);
                new_entry.status = FileStatus::Unchanged;
                index_entries.push(new_entry);
            }
        }
    }
}

pub fn restore_file(
    change: &FileChange,
    current_dir: &Path,
    objects_path: &Path,
    index_entries: &mut Vec<IndexEntry>,
) {
    let file_path = current_dir.join(&change.path);
    let hash_str = hex::encode(&change.sha256);

    let blob_path = objects_path.join(&hash_str[..2]).join(&hash_str[2..]);
    let decompressed = decompress_file_content(&blob_path).unwrap();
    let blob_data = &decompressed[decompressed.iter().position(|&b| b == 0).unwrap() + 1..];

    // Write file data
    fs::write(&file_path, blob_data).unwrap();

    // Update the index entry
    if let Some(i_entry) = index_entries.iter_mut().find(|i| i.path == change.path) {
        i_entry.sha256 = change.sha256;
        i_entry.status = FileStatus::Unchanged;
    }
}

fn delete_files(change: &FileChange, current_dir: &Path, index_entries: &mut Vec<IndexEntry>) {
    match &change.file_type {
        FileType::Blob => {
            let file_path = current_dir.join(&change.path);
            if file_path.exists() {
                fs::remove_file(&file_path).unwrap();
                if let Some(pos) = index_entries
                    .iter()
                    .position(|entry| entry.path == change.path)
                {
                    index_entries.remove(pos);
                }
            }
        }
        FileType::Tree => {
            let dir_name = &change.path;
            let list_of_files = utils::expand_paths(&[change.path.clone()]);

            for file in list_of_files {
                let file_path = current_dir.join(&file);
                if file_path.exists() {
                    fs::remove_file(&file_path).unwrap();
                    if let Some(pos) = index_entries.iter().position(|entry| entry.path == file) {
                        index_entries.remove(pos);
                    }
                }
            }
            fs::remove_dir_all(dir_name).unwrap();
        }
    }
}

fn create_files(
    change: &FileChange,
    current_dir: &Path,
    objects_path: &Path,
    index_entries: &mut Vec<IndexEntry>,
) {
    match &change.file_type {
        FileType::Blob => {
            let file_path = current_dir.join(&change.path);
            let hash_str = hex::encode(&change.sha256);

            let blob_path = objects_path.join(&hash_str[..2]).join(&hash_str[2..]);
            let decompressed = decompress_file_content(&blob_path).unwrap();
            let blob_data = &decompressed[decompressed.iter().position(|&b| b == 0).unwrap() + 1..];

            // Write file data
            fs::write(&file_path, blob_data).unwrap();

            // Update the index entry
            let mut new_entry = IndexEntry::create(&change.path);
            new_entry.status = FileStatus::Unchanged;
            index_entries.push(new_entry);
        }
        FileType::Tree => {
            let hash_str = hex::encode(&change.sha256);
            let tree_path = objects_path.join(&hash_str[..2]).join(&hash_str[2..]);
            let bash_path = current_dir.join(&change.path);

            restore_tree(&tree_path, &bash_path, objects_path, index_entries);
        }
    }
}
