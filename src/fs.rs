use core::mem::size_of;

use spin::Mutex;

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
    opened: bool
}

impl FileMetadata {
    const fn null() -> FileMetadata {
        FileMetadata { path: [0; MAX_PATH_LENGTH], sector: 0, size: 0, opened: false }
    }
}

/// The struct that sits at the top of the hard drive, containing the FileMetadata maps.
#[repr(packed)]
struct Header {
    first_null: usize, // the index (in entries, the next field) of the first null FileMetadata.
    entries: [FileMetadata; MAX_FILES],
    _padding: [u8; 1024 - 988] // align to 512 bytes
}

pub struct File {
    address: usize,
    path: &'static str
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
    // pub fn open(path: &str) -> Result<File, FileError> {

    // }

    pub fn close(&mut self) {

    }

    // pub fn read(&self) -> &[u8] {

    // }

    pub fn write(&mut self, data: &[u8]) {

    }

    pub fn create(path: &'static str) -> Result<File, FileError> {
        let mut header = HEADER.lock();

        if path.len() >= MAX_PATH_LENGTH {
            return Err(FileError::PathTooLong);
        }
        if header.first_null >= header.entries.len() {
            return Err(FileError::TooManyFiles);
        }

        let mut buffer: [u8; HEADER_SECTORS * 512] = [0; HEADER_SECTORS * 512];
        unsafe {
            crate::ata::read_sectors(0, core::mem::transmute(&buffer), HEADER_SECTORS);
            let header_read: &Header = core::mem::transmute(&buffer);
            let line_for_debug = 5;
        }

        // header.first_null is the index of the file we're going to create.
        // in order to figure out which sector we should write to,
        // we simply add one sector to the previous file's sector.
        let prev_entry = header.entries[header.first_null - 1];
        let addr: usize = prev_entry.sector + prev_entry.size;

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

        unsafe {
            // update the header on disk.
            // we transmute header to be a byte array, and we use *& in order to get the data behind the mutex.
            crate::ata::write_sectors(0, core::mem::transmute(&*header), HEADER_SECTORS);
        }

        Ok(File {
            address: addr,
            path
        })
    }

    pub fn delete(&mut self) {

    }
}

const fn read_header() -> Header {
    Header {
        first_null: 1, // first entry is always set to null
        entries: [FileMetadata::null(); MAX_FILES],
        _padding: [0;1024 - 988]
    }
}

static HEADER: Mutex<Header> = Mutex::new(read_header());
