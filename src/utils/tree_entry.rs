#[derive(Debug)]
pub struct TreeEntry {
    pub mode: String,           // e.g., "100644" or "040000"
    pub name: String,           // e.g., "main.rs" or "src"
    pub sha256: [u8; 32],       // SHA of the blob/tree
}

impl TreeEntry {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut entry = Vec::new();
        entry.extend_from_slice(self.mode.as_bytes());
        entry.push(b' ');

        entry.extend_from_slice(self.name.as_bytes());
        entry.push(0); // NULL separator

        entry.extend_from_slice(&self.sha256[..]);
        entry
    }
}