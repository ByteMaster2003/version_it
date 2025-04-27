use version_it::cli;
use version_it::commands::{add, clone, init, status};

fn main() {
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("init", _sub_matches)) => {
            match init() {
                Err(err) => {
                    println!("{}", err.to_string())
                }
                Ok(()) => {}
            };
        }
        Some(("status", _sub_matches)) => {
            status();
        }
        Some(("clone", _sub_matches)) => {
            clone();
        }
        Some(("add", sub_matches)) => {
            let paths: Vec<String> = sub_matches
                .get_many::<String>("paths")
                .unwrap()
                .cloned()
                .collect();

            add(&paths);
        }
        _ => unreachable!("Unknown subcommand!"),
    }
}
