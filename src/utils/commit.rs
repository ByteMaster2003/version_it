use crate::utils::{CommitEntry, IndexEntry, TreeEntry, decompress_file_content};
use chrono::Local;
use core::str;
use flate2::{Compression, write::ZlibEncoder};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    env,
    ffi::OsStr,
    fs::{self, File, OpenOptions},
    io::{Error, ErrorKind, Read, Write},
    path::Path,
    process::Command,
};

pub fn build_tree(index_entries: &[IndexEntry]) -> [u8; 32] {
    let mut tree_map: BTreeMap<String, Vec<&IndexEntry>> = BTreeMap::new();

    // Group files by their parent directory
    for entry in index_entries {
        let parent_dir = std::path::Path::new(&entry.path)
            .parent()
            .unwrap_or(std::path::Path::new(""))
            .to_str()
            .unwrap()
            .to_string();

        tree_map
            .entry(parent_dir)
            .or_insert_with(Vec::new)
            .push(entry);
    }

    // Now recursively build trees
    build_tree_recursive("", &tree_map)
}

fn build_tree_recursive(path: &str, tree_map: &BTreeMap<String, Vec<&IndexEntry>>) -> [u8; 32] {
    let mut tree_content = Vec::new();

    if let Some(entries) = tree_map.get(path) {
        for entry in entries.clone() {
            let filename = Path::new(&entry.path)
                .file_name()
                .unwrap()
                .to_str()
                .unwrap();

            let tree_entry = TreeEntry {
                mode: entry.mode.to_string(),
                name: filename.to_string(),
                sha256: entry.sha256,
            };
            tree_content.extend_from_slice(&tree_entry.to_bytes());
        }
    }

    // Recursively handle subdirectories
    for (dir_path, _dir_entries) in tree_map.iter() {
        if Path::new(dir_path).parent().map(|p| p.to_str().unwrap()) == Some(path) {
            let sub_tree_hash = build_tree_recursive(dir_path, tree_map);
            let mode = "040000"; // Directory
            let dirname = Path::new(dir_path)
                .file_name()
                .unwrap_or(OsStr::new(""))
                .to_str()
                .unwrap();

            let tree_entry = TreeEntry {
                mode: mode.to_string(),
                name: dirname.to_string(),
                sha256: sub_tree_hash,
            };
            tree_content.extend_from_slice(&tree_entry.to_bytes());
        }
    }

    // Finally, hash this tree and store it as an object
    let tree_hash = save_tree_object(&tree_content).unwrap();
    tree_hash
}

pub fn save_tree_object(content: &[u8]) -> Result<[u8; 32], Error> {
    let current_dir = env::current_dir().unwrap();
    let vit_dir = current_dir.join(".vit");
    if !vit_dir.exists() {
        return Err(Error::new(ErrorKind::NotFound, "vit repository not initialized!"));
    }

    let mut full_data: Vec<u8> = Vec::new();
    let str_data = format!("tree {}\0", content.len());
    full_data.extend_from_slice(str_data.as_bytes());
    full_data.extend_from_slice(content);

    let mut hasher = Sha256::new();
    hasher.update(&full_data[..]);
    let tree_hash = hasher.finalize();

    let mut tree_id = [0u8; 32];
    tree_id.copy_from_slice(&tree_hash[..]);

    // Save object to .vit/objects
    let tree_hash_str = hex::encode(tree_id);
    let object_dir = vit_dir.join(format!("objects/{}", &tree_hash_str[..2]));
    let object_file = object_dir.join(&tree_hash_str[2..]);

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&full_data).unwrap();
    let compressed = encoder.finish().unwrap();

    fs::create_dir_all(&object_dir).unwrap();
    fs::write(object_file, compressed).unwrap();

    Ok(tree_id)
}

pub fn parse_tree_entries(path: &Path) -> Result<Vec<TreeEntry>, Error> {
    let decompressed = decompress_file_content(path).unwrap();
    let data = &decompressed[decompressed.iter().position(|&b| b == 0).unwrap() + 1..];
    let mut tree_entries: Vec<TreeEntry> = Vec::new();
    let mut cursor = 0;

    while cursor < data.len() {
        let mode_end = data[cursor..].iter().position(|&b| b == b' ').unwrap();
        let mode: String = str::from_utf8(&data[cursor..cursor + mode_end])
            .unwrap()
            .to_string();
        cursor += mode_end + 1;

        let name_end = data[cursor..].iter().position(|&b| b == 0).unwrap();
        let name = str::from_utf8(&data[cursor..cursor + name_end])
            .unwrap()
            .to_string();
        cursor += name_end + 1;

        let mut sha256: [u8; 32] = [0u8; 32];
        sha256.copy_from_slice(&data[cursor..cursor + 32]);
        cursor += 32;

        tree_entries.push(TreeEntry { mode, name, sha256 });
    }

    Ok(tree_entries)
}

pub fn build_commit(tree_hash: [u8; 32], parent_hash: &[u8], message: &str) -> [u8; 32] {
    let commit_entry = CommitEntry {
        tree: tree_hash,
        parent: parent_hash.try_into().unwrap(),
        author: "Vivek <vivek@example.com>".to_string(),
        committer: "Vivek <vivek@example.com>".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        timezone: "".to_string(),
        message: message.to_string(),
    };

    save_commit_object(&commit_entry.to_bytes())
}

fn save_commit_object(content: &[u8]) -> [u8; 32] {
    let str_data = format!("commit {}\0", content.len());
    let mut full_data = str_data.into_bytes();
    full_data.extend_from_slice(content);

    let mut hasher = Sha256::new();
    hasher.update(&full_data[..]);

    let hash_result = hasher.finalize();
    let mut commit_hash = [0u8; 32];
    commit_hash.copy_from_slice(&hash_result[..]);

    let commit_hash_str = hex::encode(commit_hash);
    let object_dir = format!(".vit/objects/{}", &commit_hash_str[..2]);
    let object_file = format!("{}/{}", object_dir, &commit_hash_str[2..]);

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&full_data).unwrap();
    let compressed = encoder.finish().unwrap();

    fs::create_dir_all(&object_dir).unwrap();
    fs::write(object_file, compressed).unwrap();

    commit_hash
}

pub fn update_head(commit_hash: [u8; 32], commit_ref: &Path) {
    let current_dir = env::current_dir().unwrap();
    let vit_dir = current_dir.join(".vit");
    if !vit_dir.exists() {
        panic!("vit repository not initialized!");
    }

    fs::write(commit_ref, hex::encode(commit_hash)).expect("Failed to update reference!");
}

pub fn write_log_entry(
    old_commit: &[u8],
    new_commit: &[u8],
    author_name: &str,
    author_email: &str,
    message: &str,
    current_branch_ref: &str,
) {
    let current_dir = env::current_dir().unwrap();
    let vit_dir = current_dir.join(".vit");
    if !vit_dir.exists() {
        return eprintln!("vit repository not initialized!");
    }

    let now = Local::now(); // Local time
    let timestamp = now.timestamp();
    let offset = now.offset().local_minus_utc();
    let hours = offset / 3600;
    let minutes = (offset % 3600) / 60;
    let timezone = format!("{:+03}{}", hours, format!("{:02}", minutes.abs()));

    let log_entry: String;
    if current_branch_ref.contains("/stash") {
        log_entry = format!(
            "{} {} {} <{}> {} {} stash: {}\n",
            hex::encode(old_commit),
            hex::encode(new_commit),
            author_name,
            author_email,
            timestamp,
            timezone,
            message,
        );
    } else {
        log_entry = format!(
            "{} {} {} <{}> {} {} commit: {}\n",
            hex::encode(old_commit),
            hex::encode(new_commit),
            author_name,
            author_email,
            timestamp,
            timezone,
            message,
        );
    }

    let log_path = vit_dir.join("logs").join(current_branch_ref);
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(vit_dir.join(log_path))
        .unwrap()
        .write_all(log_entry.as_bytes())
        .unwrap();
}

pub fn get_commit_message_from_editor(status: &str) -> String {
    let vit_dir = env::current_dir().unwrap().join(".vit");

    // 1. Create/open temporary file
    let temp_path = vit_dir.join("COMMIT_EDITMSG");
    let temp_path = temp_path.as_path(); // similar to Git
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(temp_path)
        .expect("Cannot open COMMIT_EDITMSG");

    // 2. Write status info with '#' into file
    writeln!(
        file,
        "\n# Please enter the commit message above.\n#\n# Changes to be committed:\n# {}\n",
        status
    )
    .expect("Cannot write commit message template");

    // 3. Open user's default editor
    let editor = env::var("EDITOR").unwrap_or_else(|_| "vim".to_string()); // fallback to vim
    let status = Command::new(editor)
        .arg(temp_path)
        .status()
        .expect("Failed to open editor");

    if !status.success() {
        panic!("Editor failed");
    }

    // 4. Read the file back
    let mut contents = String::new();
    File::open(temp_path)
        .expect("Cannot open COMMIT_EDITMSG for reading")
        .read_to_string(&mut contents)
        .expect("Cannot read COMMIT_EDITMSG");

    // 5. Filter out commented lines
    let final_message: String = contents
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();

    if final_message.is_empty() {
        panic!("Aborting commit due to empty commit message.");
    }

    final_message
}
