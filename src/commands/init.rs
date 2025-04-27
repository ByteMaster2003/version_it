use clap::Command;
use std::{env, io, path::PathBuf, fs};

pub fn get_init_command() -> Command {
    Command::new("init").about("Initialize version_it")
}

pub fn init() -> io::Result<()> {
    let path_to_vit: PathBuf = env::current_dir()?.join(".vit");

    if path_to_vit.exists() {
        println!("Vit repository already initialized!");
        return Ok(());
    }

		// Create Required Directories
		fs::create_dir_all(path_to_vit.join("objects"))?;
    fs::create_dir_all(path_to_vit.join("refs/heads"))?;
    fs::create_dir_all(path_to_vit.join("refs/tags"))?;
    fs::create_dir_all(path_to_vit.join("info"))?;
    fs::create_dir_all(path_to_vit.join("hooks"))?;
    fs::create_dir_all(path_to_vit.join("logs/refs/heads"))?;

		// Create Required Files
		fs::write(path_to_vit.join("index"), "")?;
		fs::write(path_to_vit.join("HEAD"), "ref: refs/heads/main\n")?;
    fs::write(path_to_vit.join("config"), "[core]\n\trepositoryformatversion = 0\n\tfilemode = true\n\tbare = false\n")?;
    fs::write(path_to_vit.join("description"), "Unnamed repository; edit this file 'description' to name the repository.\n")?;

    println!("Initialized empty Vit repository in {}", path_to_vit.display());

    return Ok(());
}
