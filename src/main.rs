use version_it::cli;
use version_it::commands;

fn main() {
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("init", _)) => {
            match commands::init() {
                Err(err) => {
                    println!("{}", err.to_string())
                }
                Ok(()) => {}
            };
        }
        Some(("status", _)) => {
            commands::status();
        }
        Some(("clone", _)) => {
            commands::clone();
        }
        Some(("add", sub_matches)) => {
            let paths: Vec<String> = sub_matches
                .get_many::<String>("paths")
                .unwrap()
                .cloned()
                .collect();

            commands::add(&paths);
        }
        Some(("commit", sub_matches)) => {
            let message = sub_matches.get_one::<String>("message").cloned();

            commands::commit(message);
        }
        Some(("branch", sub_matches)) => {
            let brnach_name = sub_matches.get_one::<String>("name").cloned();
            let is_deleting = sub_matches.get_flag("delete");

            commands::branch(brnach_name, is_deleting);
        }
        Some(("checkout", sub_matches)) => {
            let branch_name = sub_matches.get_one::<String>("name").cloned().unwrap();

            commands::checkout(&branch_name);
        }
        Some(("stash", sub_matches)) => match sub_matches.subcommand() {
            Some(("save", save_matches)) => {
                let message = save_matches.get_one::<String>("message").cloned();

                commands::stash(message);
            }
            Some(("pop", _)) => {
                commands::pop();
            }
            Some(("apply", apply_matches)) => {
                let index = apply_matches.get_one::<u8>("message").cloned().unwrap();

                commands::apply(index);
            }
            Some(("list", _)) => {
                commands::list();
            }
            Some(("clear", _)) => {
                commands::clear();
            }
            _ => {
                commands::stash(Option::None);
            }
        },
        _ => unreachable!("Unknown subcommand!"),
    }
}
