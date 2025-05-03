use crate::utils::{
    clear_current_tree, decompress_file_content, parse_tree_entries, read_commit_file,
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
    let vit_dir = env::current_dir().unwrap().join(".vit");
    if !vit_dir.exists() {
        return println!("Vit not initialized");
    }

    let heads_dir = vit_dir.join("refs/heads");
    let head_path = vit_dir.join("HEAD");
    let head_ref = fs::read_to_string(&head_path).unwrap();
    let current_branch_ref = head_ref.trim_start_matches("ref: ").trim();

    if Path::new(vit_dir.join(current_branch_ref).as_path())
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string()
        == name
    {
        return println!("{}", "Branch is already Active!".red());
    }

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
    let tree_path = vit_dir
        .join("objects")
        .join(&tree_hash[..2])
        .join(&tree_hash[2..]);

    let base_path = env::current_dir().unwrap();
    clear_current_tree(&base_path);
    restore_tree(&tree_path, &base_path, &vit_dir.join("objects"));

    println!("Checkout to branch --> {}", &name);
}

fn restore_tree(tree_path: &Path, base_path: &Path, objects_path: &Path) {
    let tree_entries = parse_tree_entries(&tree_path).unwrap();

    for entry in tree_entries {
        let mode = entry.mode;
        let name = base_path.join(&entry.name);
        let hash = &entry.sha256;
        let hash_str = hex::encode(hash);

        if mode == "040000" {
            fs::create_dir_all(&name).unwrap();

            let sub_tree_path = objects_path.join(&hash_str[..2]).join(&hash_str[2..]);
            let sub_base_path = base_path.join(&name);

            restore_tree(&sub_tree_path, &sub_base_path, objects_path);
        } else {
            let blob_path = objects_path.join(&hash_str[..2]).join(&hash_str[2..]);
            let decompressed = decompress_file_content(&blob_path).unwrap();
            let blob_data = &decompressed[decompressed.iter().position(|&b| b == 0).unwrap() + 1..];

            fs::write(&name, blob_data).unwrap();
        }
    }
}
