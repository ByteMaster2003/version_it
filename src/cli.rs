use crate::commands::{get_add_command, get_clone_command, get_init_command, get_status_command};
use clap::Command;

pub fn cli() -> Command {
    Command::new("version_it")
        .about("A simpler versioning tool using rust")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .allow_external_subcommands(false)
        .subcommand(get_clone_command())
        .subcommand(get_init_command())
        .subcommand(get_status_command())
        .subcommand(get_add_command())
}
