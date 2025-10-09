use super::encoding::*;
use super::lock::Lock;

use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Seek, Write},
    os::unix::fs::OpenOptionsExt,
    path::Path,
    vec,
};

pub struct Authority(Vec<Entry>);

impl Authority {
    pub fn new(entries: Option<Vec<Entry>>) -> Self {
        Self(entries.unwrap_or_default())
    }

    pub fn add_entry(&mut self, entry: Entry) {
        self.0.push(entry);
    }

    fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut buf = Vec::new();

        while let Some(entry) = Entry::read_from(reader)? {
            buf.push(entry);
        }

        Ok(Self(buf))
    }

    fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        for entry in &self.0 {
            entry.write_to(writer)?
        }

        Ok(())
    }
}

impl IntoIterator for Authority {
    type Item = Entry;
    type IntoIter = vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

pub struct AuthorityFile {
    file: File,
    _lock: Option<Lock>,
}

impl AuthorityFile {
    pub fn from_existing(file: File, lock: Lock) -> io::Result<Self> {
        Ok(Self {
            file,
            _lock: Some(lock),
        })
    }

    /// # Safety
    /// the caller should ensure no other process will open the same file
    /// Note that for files created by other programs, this is generraly impossible to guarantee
    /// Thus, this api is not recommended, unless you are absolutely sure what you're doing
    pub unsafe fn from_existing_unlocked(file: File) -> Self {
        Self { file, _lock: None }
    }

    fn create_inner(path: &Path) -> io::Result<File> {
        OpenOptions::new()
            .read(true)
            .write(true)
            .mode(0o600)
            .create_new(true)
            .open(path)
    }

    pub fn create(path: &Path) -> io::Result<Self> {
        let file = Self::create_inner(path)?;
        let lock = Lock::aqquire(path)?;

        Ok(Self {
            file,
            _lock: Some(lock),
        })
    }

    /// # Safety
    /// the caller should ensure no other process will open the same path
    pub unsafe fn create_unlocked(path: &Path) -> io::Result<Self> {
        let file = Self::create_inner(path)?;
        Ok(Self { file, _lock: None })
    }

    pub fn get(&mut self) -> io::Result<Authority> {
        self.file.rewind()?;
        Authority::read_from(&mut self.file)
    }

    pub fn set(&mut self, authority: Authority) -> io::Result<()> {
        self.file.rewind()?;
        authority.write_to(&mut self.file)
    }
}
