use crate::commands::{
    get_add_command, get_branch_command, get_checkout_command,
    get_commit_command, get_init_command, get_log_command, get_stash_command, get_status_command,
};
use clap::Command;

pub fn cli() -> Command {
    Command::new("vit")
        .about("A simpler versioning tool using rust")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .allow_external_subcommands(false)
        .subcommand(get_init_command())
        .subcommand(get_status_command())
        .subcommand(get_add_command())
        .subcommand(get_commit_command())
        .subcommand(get_branch_command())
        .subcommand(get_checkout_command())
        .subcommand(get_stash_command())
        .subcommand(get_log_command())
}
