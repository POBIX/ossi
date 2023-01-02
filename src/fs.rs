use core::mem::size_of;

use spin::{Lazy, Mutex};

/// The number of sectors the Header struct takes up.
/// Maximum number of files = HEADER_SECTORS * 512 / size_of::<FileMetadata>() - 1
const HEADER_SECTORS: usize = 2;
const MAX_FILES: usize = HEADER_SECTORS * 512 / size_of::<FileMetadata>();

const MAX_PATH_LENGTH: usize = 32;

/// The first sectors of a hard drive using our file system are a list of FileMetadatas.
/// We use them to find out where each file is. (to map each path to its contents)
#[repr(packed)]
#[derive(Clone, Copy)]
struct FileMetadata {
    /// a string that contains the path of each file. padded with nulls to the right.
    path: [u8; MAX_PATH_LENGTH],
    /// the index of the of the file content's first sector
    sector: usize,
    /// how many sectors does this file take up?
    size: usize,
    /// is this file currently open
    opened: bool,
}

/// The struct that sits at the top of the hard drive, containing the FileMetadata maps.
#[repr(packed)]
struct Header {
    first_null: usize, // the index (in entries, the next field) of the first null FileMetadata.
    entries: [FileMetadata; MAX_FILES],
    _padding: [u8; 1024 - 988], // align to 512 bytes
}

pub struct File {
    index: usize,
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
    pub fn open(path: &str) -> Result<File, FileError> {
        let mut header = HEADER.lock();

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
                if file.opened {
                    return Err(FileError::FileAlreadyOpen);
                }
                file.opened = true;
                return Ok(File { index: i });
            }
        }
        Err(FileError::FileNotFound)
    }

    pub fn close(&mut self) {
        let mut header = HEADER.lock();
        header.entries[self.index].opened = false;
        self.index = MAX_FILES+1; // mark this reference as invalid
    }

    pub fn create(path: &'static str) -> Result<File, FileError> {
        let mut header = HEADER.lock();

        if path.len() >= MAX_PATH_LENGTH {
            return Err(FileError::PathTooLong);
        }
        if header.first_null >= header.entries.len() {
            return Err(FileError::TooManyFiles);
        }

        // header.first_null is the index of the file we're going to create.
        // in order to figure out which sector we should write to,
        // we simply add one sector to the previous file's sector.
        let addr: usize = if header.first_null > 0 {
            let prev = header.entries[header.first_null];
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
            opened: false,
        };
        header.first_null += 1;

        // update it on disk
        update_header(&header);

        Ok(File { index: first_null })
    }

    pub fn delete(&mut self) {}
}

fn read_header() -> Mutex<Header> {
    let buffer: [u8; HEADER_SECTORS * 512] = [0; HEADER_SECTORS * 512];
    let header: Header;
    unsafe {
        crate::ata::read_sectors(0, core::mem::transmute(&buffer), HEADER_SECTORS);
        header = core::mem::transmute(buffer);
    }

    Mutex::new(header)
}

/// writes header to disk.
#[inline]
fn update_header(header: &Header) {
    unsafe {
        // we transmute header to be a byte array, and we use *& in order to get the data behind the mutex.
        crate::ata::write_sectors(0, core::mem::transmute(header), HEADER_SECTORS);
    }
}

static HEADER: Lazy<Mutex<Header>> = Lazy::new(read_header);
