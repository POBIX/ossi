use spin::Mutex;

/// The number of sectors the Header struct takes up.
/// Maximum number of files = HEADER_SECTORS * 512 / size_of::<HeaderEntry>()
const HEADER_SECTORS: usize = 256;

const MAX_PATH_LENGTH: usize = 64;

/// The first sectors of a hard drive using our file system is a list of HeaderEntries.
/// We use them to find out where each file is.
#[repr(packed)]
#[derive(Clone, Copy)]
struct HeaderEntry {
    path: [u8; MAX_PATH_LENGTH], // a string that contains the path of each file. padded with nulls to the right.
    address: usize, // the index of the of the file content's first sector
    size: usize, // how many sectors does this file take up?
    opened: bool // is this file currently open
}

impl HeaderEntry {
    const fn null() -> HeaderEntry {
        HeaderEntry { path: [0; MAX_PATH_LENGTH], address: 0, size: 0, opened: false }
    }
}

#[repr(packed)]
struct Header {
    first_null: usize, // the index (in entries) of the first null Entry.
    entries: [HeaderEntry; HEADER_SECTORS * 512]
}

pub struct File {
    address: usize,
    path: &'static str
}

pub enum FileError {
    TooManyFiles,
    FileAlreadyExists,
    FileAlreadyOpen,
    FileNotOpen,
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

        // we want to put the file's header one sector after the last one we wrote to.
        let prev_entry = header.entries[header.first_null - 1];
        let addr: usize = prev_entry.address + prev_entry.size;

        let mut padded_path: [u8; MAX_PATH_LENGTH] = [0; MAX_PATH_LENGTH];
        let len = path.len();
        padded_path[..len].copy_from_slice(path.as_bytes());

        let first_null = header.first_null;
        header.entries[first_null] = HeaderEntry {
            path: padded_path,
            address: addr,
            size: 1,
            opened: true,
        };

        unsafe {
            // update the header on disk
            crate::ata::write_sectors(0, core::mem::transmute(header));
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
        first_null: HEADER_SECTORS,
        entries: [HeaderEntry::null(); HEADER_SECTORS * 512],
    }
}

static HEADER: Mutex<Header> = Mutex::new(read_header());