use core::alloc::Layout;
use alloc::{alloc::alloc, vec::Vec, string::{String, ToString}};
use spin::{Lazy, Mutex};
use crate::syscall;
use crate::fs::*;

fn read_header() -> Mutex<&'static mut Header> {
    unsafe {
        let ptr = alloc(Layout::from_size_align_unchecked(HEADER_SECTORS * 512, 4));
        let header: &mut Header;
        syscall::ReadSectors::call(0, ptr, HEADER_SECTORS);
        header = core::mem::transmute(ptr);
        Mutex::new(header)
    }
}

pub(crate) fn dir(root: &String, folders: &mut Vec<String>, files: &mut Vec<FileMetadata>) {
    let header = syscall::get_fs_header().lock();

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

pub(crate) static HEADER: Lazy<Mutex<&mut Header>> = Lazy::new(read_header);
