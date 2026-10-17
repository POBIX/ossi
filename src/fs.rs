use core::mem::size_of;

use crate::{io, syscall};

/// The number of sectors the Header struct should take up.
pub const HEADER_SECTORS: usize = 2;
/// The maximum possible number of files
pub const MAX_FILES: usize = HEADER_SECTORS * 512 / size_of::<FileMetadata>();

pub const MAX_PATH_LENGTH: usize = 32;

bitflags::bitflags! {
    #[derive(Clone, Copy)]
    pub struct FileFlags: u8 {
        const OPENED = 1;
        const DELETED = 2;
    }
}

/// The first sectors of a hard drive using our file system are a list of FileMetadatas.
/// We use them to find out where each file is. (to map each path to its contents)
#[repr(packed)]
#[derive(Clone, Copy)]
pub struct FileMetadata {
    /// a string that contains the path of each file. padded with nulls to the right.
    pub path: [u8; MAX_PATH_LENGTH],
    /// the index of the of the file content's first sector
    pub sector: usize,
    /// how many sectors does this file take up?
    pub size: usize,
    /// flags for this file
    pub flags: FileFlags,
}

pub struct File {
    pub(crate) fd: usize,
    pub(crate) ptr: usize
}

#[derive(Debug)]
pub enum FileError {
    TooManyFiles,
    FileAlreadyExists,
    FileAlreadyOpen,
    FileClosed,
    OutOfSpace,
    FileNotFound,
    PathTooLong,
}

impl File {
    pub fn get_fd(&self) -> usize { self.fd }
}

impl Drop for File {
    fn drop(&mut self) {
        syscall::CloseFile::call(self.fd);
    }
}

impl io::Seek for File {
    fn seek(&mut self, pos: usize) {
        self.ptr = pos;
    }

    fn get_cursor_position(&self) -> usize {
        self.ptr
    }
}

impl io::Read for File {
    fn read_byte(&self) -> u8 {
        let sector_offset = self.ptr / 512; // the sector our byte is in
        let mut buffer = [0u8; 512]; // we have to read the whole sector, even for just one byte
        let md: FileMetadata = syscall::GetFileMetadata::call(self.fd);
        syscall::ReadSectors::call((md.sector + sector_offset) as u32, buffer.as_mut_ptr(), 1);

        buffer[self.ptr % 512]
    }

    fn read_bytes(&self, buffer: &mut [u8]) -> usize {
        let md: FileMetadata = syscall::GetFileMetadata::call(self.fd);
        let count = usize::min(md.size * 512, buffer.len());

        let sector_a = self.ptr / 512; // the sector offset of the first byte
        let sector_b = (self.ptr + count) / 512; // the sector offset of the last byte
        let mut sector_count = sector_b - sector_a; // the number of sectors we'll read
        // If the read amount doesn't exactly fit within N sectors, we need to read one extra sector
        if self.ptr % 512 != 0 || count % 512 != 0 {
            sector_count += 1;
        }
        syscall::ReadSectors::call((md.sector+sector_a) as u32, buffer.as_mut_ptr(), sector_count);
        count
    }
}

impl io::Write for File {
    fn write_byte(&mut self, byte: u8) {
        let md: FileMetadata = syscall::GetFileMetadata::call(self.fd);
        let sector_offset = self.ptr / 512; // the sector our byte is in
        let mut buffer = [0u8; 512]; // we have to write over the whole sector, even for just 1 byte
        // in order to not overwrite anything else, read this sector first
        syscall::ReadSectors::call((md.sector + sector_offset) as u32, buffer.as_mut_ptr(), 1);

        // set the byte
        buffer[self.ptr % 512] = byte;

        // update the sector on disk
        syscall::WriteSectors::call((md.sector + sector_offset) as u32, buffer.as_ptr(), 1);
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        //todo: resizing :(
        let sector_a = self.ptr / 512; // the first byte's sector
        let sector_b = (self.ptr + bytes.len()) / 512; // the last byte's sector
        let sector_count = sector_b - sector_a + 1;

        // If ptr currently points inside a sector, we want to not override everything before it,
        // so we read the final sector
        if self.ptr % 512 != 0 {
            //TODO
        }

        // update the sectors on disk
        let md: FileMetadata = syscall::GetFileMetadata::call(self.fd);
        syscall::WriteSectors::call((md.sector + sector_a) as u32, bytes.as_ptr(), sector_count);
    }
}

