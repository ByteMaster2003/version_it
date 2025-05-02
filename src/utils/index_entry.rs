use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use sha2::{Digest, Sha256};
use std::{
    fs, io,
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FileStatus {
    New = 0,
    Modified = 1,
    Unchanged = 2,
    Deleted = 3,
}

#[derive(Debug)]
pub struct IndexEntry {
    pub ctime_secs: u32,
    pub ctime_nsecs: u32,
    pub mtime_secs: u32,
    pub mtime_nsecs: u32,
    pub mode: u32,
    pub file_size: u32,
    pub sha256: [u8; 32],
    pub status: FileStatus,
    pub flags: u16,   // includes length of path
    pub path: String, // variable length
}

impl IndexEntry {
    pub fn create(file_path: &str) -> Self {
        assert!(
            fs::exists(file_path).unwrap_or(false),
            "File {file_path} does not exists"
        );

        let metadata = fs::metadata(file_path).expect("Unable to get metadata");
        let content = fs::read(file_path).expect("Unable to read file");
        let mut hasher = Sha256::new();

        // Create header
        let header = format!("blob {}\0", content.len());
        hasher.update(header.as_bytes());
        hasher.update(&content);

        let sha256_result = hasher.finalize();
        let mut sha256 = [0u8; 32];
        sha256.copy_from_slice(&sha256_result[..]);

        let mtime = metadata
            .modified()
            .unwrap_or(SystemTime::now())
            .duration_since(UNIX_EPOCH)
            .unwrap();
        let ctime = metadata
            .created()
            .unwrap_or(SystemTime::now())
            .duration_since(UNIX_EPOCH)
            .unwrap();

        let path = Path::new(file_path)
            .to_str()
            .expect("Non-UTF8 path not supported")
            .to_string();

        let flags = (path.len() as u16) & 0xFFF; // 12 bits for path length in git

        return IndexEntry {
            ctime_secs: ctime.as_secs() as u32,
            ctime_nsecs: ctime.subsec_nanos(),
            mtime_secs: mtime.as_secs() as u32,
            mtime_nsecs: mtime.subsec_nanos(),
            mode: 0o100644, // regular non-executable file
            file_size: metadata.len() as u32,
            sha256,
            status: FileStatus::New,
            flags,
            path,
        };
    }

    pub fn write<W: Write>(&self, file: &mut W) -> io::Result<()> {
        file.write_u32::<BigEndian>(self.ctime_secs)?;
        file.write_u32::<BigEndian>(self.ctime_nsecs)?;
        file.write_u32::<BigEndian>(self.mtime_secs)?;
        file.write_u32::<BigEndian>(self.mtime_nsecs)?;
        file.write_u32::<BigEndian>(self.mode)?;
        file.write_u32::<BigEndian>(self.file_size)?;
        file.write_all(&self.sha256)?;
        file.write_all(&[self.status as u8])?;
        file.write_u16::<BigEndian>(self.flags)?;

        // Variable field (path)
        let path_bytes = self.path.as_bytes();
        file.write_all(path_bytes)?;

        // Calculate padding
        let total_size = 4 * 6 + 32 + 2 + path_bytes.len(); // fixed fields + path
        let padding = (8 - (total_size % 8)) % 8;
        file.write_all(&vec![0u8; padding])?;

        return Ok(());
    }

    pub fn read<R: Read + Seek>(reader: &mut R) -> io::Result<Option<Self>> {
        let ctime_secs = match reader.read_u32::<BigEndian>() {
            Ok(v) => v,
            Err(_) => return Ok(None), // EOF reached
        };
        let ctime_nsecs = reader.read_u32::<BigEndian>()?;
        let mtime_secs = reader.read_u32::<BigEndian>()?;
        let mtime_nsecs = reader.read_u32::<BigEndian>()?;
        let mode = reader.read_u32::<BigEndian>()?;
        let file_size = reader.read_u32::<BigEndian>()?;

        let mut sha256 = [0u8; 32];
        reader.read_exact(&mut sha256)?;
        let mut status_buf = [0u8; 1];
        reader.read_exact(&mut status_buf)?;
        let status = match status_buf[0] {
            0 => FileStatus::New,
            1 => FileStatus::Modified,
            2 => FileStatus::Unchanged,
            3 => FileStatus::Deleted,
            _ => return Ok(None), // Invalid status, corrupted index
        };

        let flags = reader.read_u16::<BigEndian>()?;
        let path_len = (flags & 0x0FFF) as usize;

        let mut path_buf = vec![0u8; path_len];
        reader.read_exact(&mut path_buf)?;
        let path = String::from_utf8(path_buf).expect("Invalid UTF-8 in path");

        // Skip padding
        let total_size = 4 * 6 + 32 + 2 + path_len;
        let padding = (8 - (total_size % 8)) % 8;
        if padding > 0 {
            reader.seek(SeekFrom::Current(padding as i64))?;
        }

        Ok(Some(Self {
            ctime_secs,
            ctime_nsecs,
            mtime_secs,
            mtime_nsecs,
            mode,
            file_size,
            sha256,
            status,
            flags,
            path,
        }))
    }
}
