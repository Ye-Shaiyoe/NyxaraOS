use crate::disk::SECTOR_SIZE;
use alloc::string::String;
use alloc::vec::Vec;

pub const ENTRY_SIZE: usize = 64;
pub const MAX_NAME_LEN: usize = 31;
pub const ENTRIES_PER_SECTOR: usize = SECTOR_SIZE / ENTRY_SIZE;

pub const TYPE_FREE: u8 = 0;
pub const TYPE_FILE: u8 = 1;
pub const TYPE_DIR: u8 = 2;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DirEntry {
    pub name: [u8; 32],
    pub entry_type: u8,
    pub start_block: u16,
    _pad: u8,
    pub size: u32,
    pub parent_block: u16,
    pub reserved: [u8; 22],
}

impl DirEntry {
    pub const fn empty() -> Self {
        Self {
            name: [0; 32],
            entry_type: TYPE_FREE,
            start_block: 0,
            _pad: 0,
            size: 0,
            parent_block: 0,
            reserved: [0; 22],
        }
    }

    pub fn is_free(&self) -> bool {
        self.entry_type == TYPE_FREE
    }

    pub fn get_name(&self) -> &str {
        let len = self.name.iter().position(|&b| b == 0).unwrap_or(32);
        core::str::from_utf8(&self.name[..len]).unwrap_or("")
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = [0; 32];
        let bytes = name.as_bytes();
        let len = bytes.len().min(MAX_NAME_LEN);
        self.name[..len].copy_from_slice(&bytes[..len]);
    }
}

pub struct DirOps {
    pub disk_offset: u32,
    pub dir_lba: u32,
    pub dir_sectors: u32,
}

impl DirOps {
    pub fn read_entries(&self) -> Vec<DirEntry> {
        let mut entries = Vec::new();
        let mut buf = [0u8; SECTOR_SIZE];

        for sec in 0..self.dir_sectors {
            if !crate::disk::read_sector(self.disk_offset + self.dir_lba + sec, &mut buf) {
                break;
            }
            for i in 0..ENTRIES_PER_SECTOR {
                let offset = i * ENTRY_SIZE;
                let entry: DirEntry = unsafe {
                    core::ptr::read(buf[offset..].as_ptr() as *const DirEntry)
                };
                entries.push(entry);
            }
        }
        entries
    }

    pub fn write_entry(&self, index: usize, entry: &DirEntry) -> bool {
        let sector_idx = index / ENTRIES_PER_SECTOR;
        let entry_in_sector = index % ENTRIES_PER_SECTOR;

        if sector_idx >= self.dir_sectors as usize {
            return false;
        }

        let mut buf = [0u8; SECTOR_SIZE];
        let lba = self.disk_offset + self.dir_lba + sector_idx as u32;
        if !crate::disk::read_sector(lba, &mut buf) {
            return false;
        }

        let offset = entry_in_sector * ENTRY_SIZE;
        unsafe {
            core::ptr::copy_nonoverlapping(
                entry as *const DirEntry as *const u8,
                buf[offset..].as_mut_ptr(),
                ENTRY_SIZE,
            );
        }

        crate::disk::write_sector(lba, &buf)
    }

    pub fn find_entry(&self, name: &str) -> Option<(usize, DirEntry)> {
        let entries = self.read_entries();
        for (i, entry) in entries.iter().enumerate() {
            if !entry.is_free() && entry.get_name() == name {
                return Some((i, *entry));
            }
        }
        None
    }

    pub fn find_free_slot(&self) -> Option<usize> {
        let entries = self.read_entries();
        for (i, entry) in entries.iter().enumerate() {
            if entry.is_free() {
                return Some(i);
            }
        }
        None
    }

    pub fn create_entry(&self, name: &str, entry_type: u8, start_block: u16, size: u32) -> Option<usize> {
        let slot = self.find_free_slot()?;
        let mut entry = DirEntry::empty();
        entry.set_name(name);
        entry.entry_type = entry_type;
        entry.start_block = start_block;
        entry.size = size;

        if self.write_entry(slot, &entry) {
            Some(slot)
        } else {
            None
        }
    }

    pub fn delete_entry(&self, index: usize) -> bool {
        let empty = DirEntry::empty();
        self.write_entry(index, &empty)
    }

    pub fn list_entries(&self) -> Vec<(String, u8, u32)> {
        let entries = self.read_entries();
        let mut result = Vec::new();
        for entry in &entries {
            if entry.is_free() {
                continue;
            }
            result.push((
                String::from(entry.get_name()),
                entry.entry_type,
                entry.size,
            ));
        }
        result
    }
}
