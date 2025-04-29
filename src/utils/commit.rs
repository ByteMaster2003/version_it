use crate::utils::IndexEntry;
use chrono::Local;
use flate2::{ Compression, write::ZlibEncoder };
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    env, fs,
    fs::{File, OpenOptions},
    io::{Error, ErrorKind, Read, Write},
    path::{Path, PathBuf},
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
        for entry in entries {
            let mode = entry.mode.to_string(); // Regular file for now
            let filename = std::path::Path::new(&entry.path)
                .file_name()
                .unwrap()
                .to_str()
                .unwrap();

            tree_content.extend_from_slice(mode.as_bytes());
            tree_content.push(b' ');
            tree_content.extend_from_slice(filename.as_bytes());
            tree_content.push(0); // NULL separator
            tree_content.extend_from_slice(&entry.sha256[..]);
        }
    }

    // Recursively handle subdirectories
    for (dir_path, _dir_entries) in tree_map.iter() {
        if Path::new(dir_path).parent().map(|p| p.to_str().unwrap()) == Some(path) {
            let sub_tree_hash = build_tree_recursive(dir_path, tree_map);

            let mode = "040000"; // Directory
            let dirname = Path::new(dir_path)
                .to_str()
                .unwrap();

            tree_content.extend_from_slice(mode.as_bytes());
            tree_content.push(b' ');
            tree_content.extend_from_slice(dirname.as_bytes());
            tree_content.push(0);
            tree_content.extend_from_slice(&sub_tree_hash[..]);
        }
    }

    // Finally, hash this tree and store it as an object
    let tree_hash = save_tree_object(&tree_content).unwrap();
    tree_hash
}

fn save_tree_object(content: &[u8]) -> Result<[u8; 32], Error> {
    let path_to_vit: PathBuf = env::current_dir()?.join(".vit");

    if !path_to_vit.exists() {
        return Err(Error::new(ErrorKind::NotFound, "Vit directory not found!"));
    }

    let mut hasher = Sha256::new();
    hasher.update(b"tree ");
    hasher.update(content.len().to_string().as_bytes());
    hasher.update(b"\0");
    hasher.update(content);
    let tree_hash = hasher.finalize();

    let mut tree_id = [0u8; 32];
    tree_id.copy_from_slice(&tree_hash[..]);

    // Save object to .vit/objects
    let object_dir = path_to_vit.join(format!("objects/{:02x}", tree_id[0]));
    let object_file = object_dir.join(hex::encode(&tree_id[1..]));

    fs::create_dir_all(&object_dir).unwrap();
    fs::write(&object_file, content).unwrap();

    Ok(tree_id)
}

pub fn build_commit(tree_hash: [u8; 32], parent_hash: &[u8], message: &str) -> Vec<u8> {
    let mut commit_content = Vec::new();

    commit_content.extend_from_slice(b"tree ");
    commit_content.extend_from_slice(hex::encode(tree_hash).as_bytes());
    commit_content.push(b'\n');

    commit_content.extend_from_slice(b"parent ");
    commit_content.extend_from_slice(&parent_hash[..]);
    commit_content.push(b'\n');

    let author = "Vivek <vivek@example.com>";
    let timestamp = chrono::Utc::now().timestamp();

    commit_content.extend_from_slice(b"author ");
    commit_content.extend_from_slice(author.as_bytes());
    commit_content.extend_from_slice(format!(" {}", timestamp).as_bytes());
    commit_content.push(b'\n');

    commit_content.extend_from_slice(b"committer ");
    commit_content.extend_from_slice(author.as_bytes());
    commit_content.extend_from_slice(format!(" {}", timestamp).as_bytes());
    commit_content.push(b'\n');

    commit_content.push(b'\n'); // Blank line before message
    commit_content.extend_from_slice(message.as_bytes());

    commit_content
}

pub fn save_commit_object(content: &[u8]) -> [u8; 32] {
    let str_data = format!("commit {}\0", content.len());
    let mut full_data = str_data.into_bytes();
    full_data.extend_from_slice(content);

    let mut hasher = Sha256::new();
    hasher.update(&full_data[..]);

    let hash_result = hasher.finalize();
    let mut commit_hash = [0u8; 32];
    commit_hash.copy_from_slice(&hash_result[..]);

    let object_dir = format!(".vit/objects/{:02x}", commit_hash[0]);
    let object_file = format!("{}/{}", object_dir, hex::encode(&commit_hash[1..]));

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&full_data).unwrap();
    let compressed = encoder.finish().unwrap();

    fs::create_dir_all(&object_dir).unwrap();
    fs::write(object_file, compressed).unwrap();

    commit_hash
}

pub fn update_head(commit_hash: [u8; 32]) {
    let current_dir = env::current_dir().unwrap();
    let vit_dir = current_dir.join(".vit");

    if !vit_dir.exists() {
        panic!("Version_it repository not initialized!");
    }

    let ref_dir = vit_dir.join("refs/heads/main");
    fs::write(ref_dir, hex::encode(commit_hash)).expect("Failed to update reference!");
}

pub fn write_log_entry(
    old_commit: &[u8],
    new_commit: &[u8],
    author_name: &str,
    author_email: &str,
    message: &str,
) {
    let current_dir = env::current_dir().unwrap();
    let vit_dir = current_dir.join(".vit");

    if !vit_dir.exists() {
        panic!("Version_it repository not initialized!");
    }

    let now = Local::now(); // Local time
    let timestamp = now.timestamp();
    let offset = now.offset().local_minus_utc();
    let hours = offset / 3600;
    let minutes = (offset % 3600) / 60;
    let timezone = format!("{:+03}{}", hours, format!("{:02}", minutes.abs()));

    let log_entry = format!(
        "{} {} {} <{}> {} {} commit: {}\n",
        hex::encode(old_commit),
        hex::encode(new_commit),
        author_name,
        author_email,
        timestamp,
        timezone,
        message,
    );

    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(vit_dir.join("logs/refs/heads/main"))
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
