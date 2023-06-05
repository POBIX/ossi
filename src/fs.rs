use core::alloc::Layout;
use core::mem::size_of;

use alloc::{alloc::alloc, vec::Vec, string::{String, ToString}};
use spin::{Lazy, Mutex};

use crate::io;

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

/// The struct that sits at the top of the hard drive, containing the FileMetadata maps.
#[repr(packed)]
pub struct Header {
    first_null: usize, // the index (in entries, the next field) of the first null FileMetadata.
    entries: [FileMetadata; MAX_FILES],
    _padding: [u8; 1024 - 988], // align to 512 bytes
}

pub struct File {
    index: usize,
    ptr: usize
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
    fn from_index(index: usize) -> File { File { index, ptr: 0 } }

    pub fn open(path: &str) -> Result<File, FileError> {
        let mut header = crate::syscall::get_fs_header().lock();

        // search for a file with the same path as the parameter
        for i in 0..header.entries.len() {
            let file = &mut header.entries[i];
            // convert the path (a null terminated [u8]) to a rust string
            let file_path = unsafe {
                core::str::from_utf8_unchecked(
                    core::ffi::CStr::from_ptr(file.path.as_ptr() as *const _).to_bytes(),
                )
            };
            if file_path == path {
                // found it!
                if file.flags.contains(FileFlags::OPENED) {
                    return Err(FileError::FileAlreadyOpen);
                }
                file.flags.set(FileFlags::OPENED, true);
                return Ok(File::from_index(i));
            }
        }
        Err(FileError::FileNotFound)
    }

    pub fn create(path: &str) -> Result<File, FileError> {
        let mut header = crate::syscall::get_fs_header().lock();

        if path.len() >= MAX_PATH_LENGTH {
            return Err(FileError::PathTooLong);
        }
        if header.first_null >= header.entries.len() {
            return Err(FileError::TooManyFiles);
        }

        // Make sure that path doesn't already exist
        for i in 0..header.entries.len() {
            let file = &mut header.entries[i];
            // convert the path (a null terminated [u8]) to a rust string
            let file_path = unsafe {
                core::str::from_utf8_unchecked(
                    core::ffi::CStr::from_ptr(file.path.as_ptr() as *const _).to_bytes(),
                )
            };
            if file_path == path {
                return Err(FileError::FileAlreadyExists);
            }
        }

        // header.first_null is the index of the file we're going to create.
        // in order to figure out which sector we should write to,
        // we simply add one sector to the previous file's sector.
        // TODO: this assumes files are never deleted.
        let addr: usize = if header.first_null > 0 {
            let prev = header.entries[header.first_null-1];
            prev.sector + prev.size
        } else {
            // or, if this is the first file, use the first available sector.
            HEADER_SECTORS + 1
        };

        // pad the path with nulls to the right
        let mut padded_path: [u8; MAX_PATH_LENGTH] = [0; MAX_PATH_LENGTH];
        let len = path.len();
        padded_path[..len].copy_from_slice(path.as_bytes());

        // update the metadata in memory
        let first_null = header.first_null;
        header.entries[first_null] = FileMetadata {
            path: padded_path,
            sector: addr,
            size: 1,
            flags: FileFlags::empty()
        };
        header.first_null += 1;

        crate::println!("{} {} {}", unsafe { core::str::from_utf8_unchecked(&padded_path) }, addr, 1);

        // update it on disk
        update_header(&header);

        Ok(File::from_index(first_null))
    }

    pub fn delete(&mut self) -> Result<(), FileError>{
        let mut header = crate::syscall::get_fs_header().lock();
        let metadata = &mut header.entries[self.index];

        if !metadata.flags.contains(FileFlags::OPENED) {
            return Err(FileError::FileClosed);
        }
        metadata.flags.set(FileFlags::DELETED, true);
        Ok(())
    }

    #[inline]
    pub fn get_metadata(&self) -> FileMetadata {
        crate::syscall::get_fs_header().lock().entries[self.index]
    }

    pub fn set_size(&mut self, val: usize) {
        let mut header = crate::syscall::get_fs_header().lock();
        header.entries[self.index].size = val;
        update_header(&header);
    }

    pub fn close(&mut self) {
        // close the file
        let mut header = crate::syscall::get_fs_header().lock();
        header.entries[self.index].flags.set(FileFlags::OPENED, false);
        self.index = MAX_FILES+1; // mark this reference as invalid
        update_header(&header);
    }
}

impl Drop for File {
    fn drop(&mut self) {
        self.close();
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
        let md = self.get_metadata();
        crate::syscall::ReadSectors::call((md.sector + sector_offset) as u32, buffer.as_mut_ptr(), 1);

        buffer[self.ptr % 512]
    }

    fn read_bytes(&self, buffer: &mut [u8]) -> usize {
        let md = self.get_metadata();
        let count = usize::min(md.size * 512, buffer.len());

        let sector_a = self.ptr / 512; // the sector offset of the first byte
        let sector_b = (self.ptr + count) / 512; // the sector offset of the last byte
        let mut sector_count = sector_b - sector_a; // the number of sectors we'll read
        // If the read amount doesn't exactly fit within N sectors, we need to read one extra sector
        if self.ptr % 512 != 0 || count % 512 != 0 {
            sector_count += 1;
        }
        crate::syscall::ReadSectors::call((md.sector + sector_a) as u32, buffer.as_mut_ptr(), sector_count);
        count
    }
}

impl io::Write for File {
    fn write_byte(&mut self, byte: u8) {
        let sector_offset = self.ptr / 512; // the sector our byte is in
        let mut buffer = [0u8; 512]; // we have to write over the whole sector, even for just one byte
        // in order to not overwrite anything else, read this sector first
        let md = self.get_metadata();
        crate::syscall::ReadSectors::call((md.sector + sector_offset) as u32, buffer.as_mut_ptr(), 1);

        // set the byte
        buffer[self.ptr % 512] = byte;

        // update the sector on disk
        crate::syscall::WriteSectors::call((md.sector + sector_offset) as u32, buffer.as_ptr(), 1);
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        //todo: resizing :(
        let sector_a = self.ptr / 512; // the first byte's sector
        let sector_b = (self.ptr + bytes.len()) / 512; // the last byte's sector
        let sector_count = sector_b - sector_a + 1;

        // In case ptr currently points at the middle of a sector, we want to not override everything before it,
        // so we read the final sector
        if self.ptr % 512 != 0 {
            //TODO
        }
        let md = self.get_metadata();

        // update the sectors on disk
        crate::syscall::WriteSectors::call((md.sector + sector_a) as u32, bytes.as_ptr(), sector_count);
    }
}

fn read_header() -> Mutex<&'static mut Header> {
    unsafe {
        let ptr = alloc(Layout::from_size_align_unchecked(HEADER_SECTORS * 512, 4));
        let header: &mut Header;
        crate::syscall::ReadSectors::call(0, ptr, HEADER_SECTORS);
        header = core::mem::transmute(ptr);
        Mutex::new(header)
    }
}

/// writes header to disk.
#[inline]
fn update_header(header: &Header) {
    unsafe {
        // we transmute header to be a byte array, and we use *& in order to get the data behind the mutex.
        crate::syscall::WriteSectors::call(0, core::mem::transmute(header), HEADER_SECTORS);
    }
}

pub(crate) fn dir(root: &String, folders: &mut Vec<String>, files: &mut Vec<FileMetadata>) {
    let header = crate::syscall::get_fs_header().lock();

    for file in &header.entries {
        if !file.path.starts_with(root.as_bytes()) {
            continue;
        }
        let split: Vec<_> = file.path[root.len()..].split(|&b| b == b'/').collect();

        // If there aren't any slashes in the path (after removing the root), it's a top level file
        if split.len() == 1 {
            files.push(file.clone());
        }
        else {
            // Otherwise, get the folder
            let name = unsafe { core::str::from_utf8_unchecked(split[0]) };
            if folders.iter().all(|item| *item != name) {
                folders.push(name.to_string())
            }
        }
    }
}

pub(crate) static HEADER: Lazy<Mutex<&mut Header>> = Lazy::new(read_header);
