# Version_it

A lightweight version control system implemented in Rust, inspired by Git. This tool provides basic version control operations for tracking changes in your codebase.

## Installation

1. Make sure you have Rust installed on your system
2. Clone this repository
3. Build the project:
```sh
cargo build --release
```
4. The binary will be available at `target/release/version_it`

## Commands

### Initialize a Repository
```sh
version_it init
```
Creates a new version_it repository in the current directory.

### Check Status
```sh
version_it status
```
Shows the current status of working directory including:
- Untracked files
- Modified files
- Staged changes

### Add Files
```sh
version_it add <file_path>
```
Add specific files to staging area:
```sh
version_it add src/main.rs          # Add single file
version_it add .                    # Add all files
```

### Commit Changes
```sh
version_it commit -m "commit message"
```
Commits staged changes with the specified message. If `-m` is not provided, opens default editor for message input.

### Branch Operations
```sh
version_it branch                   # List all branches
version_it branch <name>           # Create new branch
version_it branch -D <name>        # Delete branch
```

### Switch Branches
```sh
version_it checkout <branch-name>
```
Switches to the specified branch.

### View Commit History
```sh
version_it log
```
Shows commit history with details like commit hash, author, date, and message.

### Stash Operations
```sh
version_it stash                   # Save current changes to stash
version_it stash save "message"    # Save with custom message
version_it stash list             # List stashed changes
version_it stash pop              # Apply and remove latest stash
version_it stash apply <index>    # Apply specific stash
version_it stash clear           # Clear all stashes
```

## Features

- File tracking and versioning
- Commit history
- Branching support
- Stash functionality
- .vitignore support for excluding files
- Colored output for better visibility

## Ignored Files

Create a `.vitignore` file in your repository to specify patterns for files to ignore. Example:
```
/target
.vit
*.log
```

## Development Status

This is a learning project implementing basic version control functionality. While functional, it's recommended for educational purposes rather than production use.

## License

This project is open source and available under the MIT License.