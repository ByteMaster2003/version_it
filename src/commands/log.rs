use std::{env, fs};

use clap::Command;
use colored::{Colorize, control::set_override};
use pager::Pager;

use crate::utils::read_commit_file;

pub fn get_log_command() -> Command {
    Command::new("log").about("Display commit logs")
}

pub fn log() {
    let current_dir = env::current_dir().unwrap();
    let vit_dir = current_dir.join(".vit");
    let object_dir = vit_dir.join("objects");
    if !vit_dir.exists() {
        return eprintln!(".vit directory not found!");
    }

    let head_path = vit_dir.join("HEAD");
    let head_ref = fs::read_to_string(&head_path).unwrap();
    let current_branch_ref = head_ref.trim_start_matches("ref: ").trim();

    let mut commit_hash = fs::read_to_string(vit_dir.join(current_branch_ref)).unwrap();
    let start_commit = hex::encode([0u8; 32]);

    loop {
        // Setup pager
        set_override(true);
        Pager::with_pager("less -R -F -X").setup();

        if commit_hash == start_commit {
            break;
        }
        let commit_path = object_dir.join(&commit_hash[..2]).join(&commit_hash[2..]);
        let commit_entry = read_commit_file(&commit_path).unwrap();

        println!("{} {}", "commit".yellow(), commit_hash.yellow());
        println!("Author: {}", commit_entry.author);
        println!("Date:   {} {}", commit_entry.timestamp, commit_entry.timezone);
        println!();
        println!("    {}", commit_entry.message);
        println!();

        commit_hash = hex::encode(commit_entry.parent);
    }
}
