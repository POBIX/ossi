use core::alloc::Layout;
use alloc::{alloc::alloc, vec::Vec, string::{String, ToString}};
use spin::{Lazy, Mutex};
use crate::kernel;
use crate::fs::*;

/// The struct that sits at the top of the hard drive, containing the FileMetadata maps.
#[repr(packed)]
pub struct Header {
    first_null: usize, // the index (in entries, the next field) of the first null FileMetadata.
    pub(crate) entries: [FileMetadata; MAX_FILES],
    _padding: [u8; 1024 - 988], // align to 512 bytes
}

pub(crate) static HEADER: Lazy<Mutex<&mut Header>> = Lazy::new(read_header);

fn read_header() -> Mutex<&'static mut Header> {
    unsafe {
        let ptr = alloc(Layout::from_size_align_unchecked(HEADER_SECTORS * 512, 4));
        let header: &mut Header;
        kernel::ata::read_sectors(0, ptr, HEADER_SECTORS);
        header = core::mem::transmute(ptr);
        Mutex::new(header)
    }
}

/// writes header to disk.
#[inline]
fn update_header(header: &Header) {
    unsafe {
        kernel::ata::write_sectors(0, core::mem::transmute(header), HEADER_SECTORS);
    }
}

impl File {
    fn new(fd: usize, md: &FileMetadata) -> File { File { fd, ptr: 0, md: *md } }
}

pub(crate) fn open_file(path_raw: *const u8, len: usize) -> Result<File, FileError> {
    let mut header = HEADER.lock();
    let path: &str = unsafe {
        core::str::from_utf8_unchecked(core::slice::from_raw_parts(path_raw, len))
    };

    // search for a file with the same path as the parameter
    for i in 0..header.entries.len() {
        let file = &mut header.entries[i];
        if file.flags.contains(FileFlags::DELETED) { continue; }
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
            return Ok(File::new(i, file));
        }
    }
    Err(FileError::FileNotFound)
}

pub(crate) fn create_file(path_raw: *const u8, len: usize) -> Result<File, FileError> {
    let mut header = HEADER.lock();

    if len >= MAX_PATH_LENGTH {
        return Err(FileError::PathTooLong);
    }
    if header.first_null >= header.entries.len() {
        return Err(FileError::TooManyFiles);
    }

    let path: &str = unsafe {
        core::str::from_utf8_unchecked(core::slice::from_raw_parts(path_raw, len))
    };

    // Make sure that path doesn't already exist
    for i in 0..header.entries.len() {
        let file = &mut header.entries[i];
        if file.flags.contains(FileFlags::DELETED) { continue; }
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

    // update it on disk
    update_header(&header);

    Ok(File::new(first_null, &header.entries[first_null]))
}

pub(crate) fn delete_file(fd: usize) -> Result<(), FileError> {
    let mut header = HEADER.lock();
    let metadata = &mut header.entries[fd];

    if !metadata.flags.contains(FileFlags::OPENED) {
        return Err(FileError::FileClosed);
    }
    metadata.flags.set(FileFlags::DELETED, true);
    update_header(&header);
    Ok(())
}

pub(crate) fn set_file_size(fd: usize, val: usize) {
    let mut header = HEADER.lock();
    header.entries[fd].size = val;
    update_header(&header);
}

pub(crate) fn close_file(fd: usize) {
    // close the file
    let mut header = HEADER.lock();
    if fd >= header.entries.len() {
        // File already closed
        return;
    }
    header.entries[fd].flags.set(FileFlags::OPENED, false);
    update_header(&header);
}

pub(crate) fn ls(root: &str, folders: &mut Vec<String>, files: &mut Vec<FileMetadata>) {
    let header = HEADER.lock();

    for file in &header.entries {
        if !file.path.starts_with(root.as_bytes()) || file.flags.contains(FileFlags::DELETED) {
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

