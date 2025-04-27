use clap::{Command, arg};

pub fn get_clone_command() -> Command {
    Command::new("clone")
        .about("Clone remote repository")
        .arg(arg!(<REMOTE> "The remote to clone"))
        .arg_required_else_help(true)
}

pub fn clone() {
	println!("Cloning repository with version_it")
}
