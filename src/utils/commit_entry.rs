use std::{io, path::Path};
use crate::utils::decompress_file_content;

#[derive(Debug)]
pub struct CommitEntry {
    pub tree: [u8; 32],    // SHA-256 of the tree object
    pub parent: [u8; 32],  // Optional for the first commit
    pub author: String,    // "Name <email>"
    pub committer: String, // "Name <email>"
    pub timestamp: i64,    // UNIX timestamp
    pub timezone: String,  // e.g., "+0530"
    pub message: String,   // Commit message
}

impl CommitEntry {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut content = Vec::new();

        content.extend_from_slice(b"tree ");
        content.extend_from_slice(hex::encode(self.tree).as_bytes());
        content.push(b'\n');

        content.extend_from_slice(b"parent ");
        content.extend_from_slice(hex::encode(self.parent).as_bytes());
        content.push(b'\n');

        content.extend_from_slice(b"author ");
        content.extend_from_slice(self.author.as_bytes());
        content.push(b' ');
        content.extend_from_slice(self.timestamp.to_string().as_bytes());
        content.push(b' ');
        content.extend_from_slice(self.timezone.as_bytes());
        content.push(b'\n');

        content.extend_from_slice(b"committer ");
        content.extend_from_slice(self.committer.as_bytes());
        content.push(b' ');
        content.extend_from_slice(self.timestamp.to_string().as_bytes());
        content.push(b' ');
        content.extend_from_slice(self.timezone.as_bytes());
        content.push(b'\n');

        content.push(b'\n');
        content.extend_from_slice(self.message.as_bytes());

        content
    }
}

pub fn read_commit_file(path: &Path) -> io::Result<CommitEntry> {
    let decompressed = decompress_file_content(path).unwrap();
    let content = &decompressed[decompressed.iter().position(|&b| b == 0).unwrap() + 1..];
    let content_str = String::from_utf8_lossy(content).into_owned();
    let mut lines = content_str.lines();

    let mut tree = [0u8; 32];
    let mut parent = [0u8; 32];
    let mut author = String::new();
    let mut committer = String::new();
    let mut timestamp = 0;
    let mut timezone = String::new();

    while let Some(line) = lines.next() {
        if line.starts_with("tree ") {
            let hash = &line[5..];
            let hash_bytes = hex::decode(hash).expect("Invalid tree hash");
            tree.copy_from_slice(&hash_bytes);
        } else if line.starts_with("parent ") {
            let hash = &line[7..];
            let hash_bytes = hex::decode(hash).expect("Invalid parent hash");
            parent.copy_from_slice(&hash_bytes);
        } else if line.starts_with("author ") {
            let author_info = &line[7..];
            let info: Vec<&str> = author_info.split(" ").collect();

            let author_name = info.get(0).cloned().unwrap_or("");
            let author_email = info.get(1).cloned().unwrap_or("");
            author = author_name.to_string() + " " + &author_email.to_string();

            timestamp = info.get(2).cloned().unwrap_or("0").parse().unwrap();
            timezone = info.get(3).cloned().unwrap_or("").to_string();
        } else if line.starts_with("committer ") {
            let committer_info = &line[10..];
            let info: Vec<&str> = committer_info.split(" ").collect();

            let committer_name = info.get(0).cloned().unwrap_or("");
            let committer_email = info.get(1).cloned().unwrap_or("");
            committer = committer_name.to_string() + " " + &committer_email.to_string();

        } else if line.is_empty() {
            break; // message follows after this
        }
    }

    let message: String = lines.collect::<Vec<_>>().join("\n");

    Ok(CommitEntry {
        tree,
        parent,
        author,
        committer,
        timestamp,
        timezone,
        message,
    })
}
