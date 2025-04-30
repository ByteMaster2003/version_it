use clap::{Arg, ArgAction, Command};
use colored::Colorize;
use std::{env, fs, path::Path};

pub fn get_branch_command() -> Command {
    Command::new("branch")
        .about("Record changes to the repository")
        .arg(
            Arg::new("name")
                .required(false)
                .help("Name of branch to be created"),
        )
        .arg(
            Arg::new("delete")
                .short('D')
                .long("delete")
                .required(false)
                .action(ArgAction::SetTrue)
                .help("Delete the specified branch"),
        )
}

pub fn branch(name: Option<String>, is_delete: bool) {
    let vit_dir = env::current_dir().unwrap().join(".vit");
    if !vit_dir.exists() {
        return println!("Vit not initialized");
    }

    let heads_dir = vit_dir.join("refs/heads");
    let head_ref = fs::read_to_string(vit_dir.join("HEAD")).unwrap(); // "ref: refs/heads/main"
    let current_branch_ref = head_ref.trim_start_matches("ref: ").trim();

    if let Some(branch_name) = name {
        let current_commit = fs::read_to_string(vit_dir.join(current_branch_ref)).unwrap();

        let new_branch_path = heads_dir.join(&branch_name);

        let does_exists = new_branch_path.exists();
        if is_delete {
            if !does_exists {
                return println!("{} {}", &branch_name.red(), "Branch does not exist!".red());
            }
            if Path::new(vit_dir.join(current_branch_ref).as_path())
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string()
                == branch_name
            {
                return println!("{}", "Can not delete active branch".red());
            }

            fs::remove_file(new_branch_path).unwrap();
            println!("Branch '{}' deleted", branch_name);
        } else {
            if does_exists {
                return println!("{} {}", &branch_name.red(), "Branch already exists!".red());
            }

            fs::write(new_branch_path, current_commit).unwrap();
            println!("Branch '{}' created", branch_name);
        }
    } else {
        for entry in fs::read_dir(heads_dir).unwrap() {
            let path = entry.unwrap().path();
            if let Some(name) = path.file_name() {
                if Path::new(vit_dir.join(current_branch_ref).as_path())
                    .file_name()
                    .unwrap()
                    == name
                {
                    println!("{} {}", name.to_str().unwrap(), "*".green());
                } else {
                    println!("{}", name.to_str().unwrap());
                }
            }
        }
    }
}
