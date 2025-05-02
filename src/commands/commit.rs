use clap::{Arg, Command};
use std::{env, fs};

use crate::utils;

pub fn get_commit_command() -> Command {
    Command::new("commit")
        .about("Record changes to the repository")
        .arg(
            Arg::new("message")
                .short('m')
                .long("message")
                .required(false)
                .value_name("MESSAGE")
                .help("Specify the commit message"),
        )
}

pub fn commit(message: Option<String>) {
    let commit_message = match message {
        Some(msg) => msg,
        None => utils::get_commit_message_from_editor("Updated:"),
    };

    if commit_message.is_empty() {
        return;
    }

    let vit_dir = env::current_dir().unwrap().join(".vit");
    let mut index_entries = utils::read_index().unwrap();
    let head_ref = fs::read_to_string(vit_dir.join("HEAD")).unwrap();
    let current_branch_ref = head_ref.trim_start_matches("ref: ").trim();
    let commit_ref = vit_dir.join(current_branch_ref);

    let prev_commit_hash = if commit_ref.exists() {
        let prev_hash_str = fs::read_to_string(&commit_ref).unwrap();
        hex::decode(prev_hash_str.trim()).unwrap()
    } else {
        vec![0u8; 32]
    };
    let tree_hash = utils::build_tree(&index_entries);
    let commit_hash: [u8; 32] = utils::build_commit(tree_hash, &prev_commit_hash, &commit_message);

    utils::update_head(commit_hash, &commit_ref);
    let author_name = "Vivek";
    let author_email = "vivek@example.com";
    utils::write_log_entry(
        &prev_commit_hash,
        &commit_hash,
        author_name,
        author_email,
        &commit_message,
        current_branch_ref
    );

    index_entries.retain(|entry| entry.status != utils::FileStatus::Deleted);
    for entry in index_entries.iter_mut() {
        entry.status = utils::FileStatus::Unchanged
    }

    utils::write_index(&index_entries, vit_dir.join("index").to_str().unwrap()).unwrap();
    println!("Changes commited successfully")
}
