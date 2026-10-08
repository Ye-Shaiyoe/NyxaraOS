pub mod superblock;
pub mod dir;
pub mod file;

use alloc::string::String;
use alloc::vec::Vec;
use core::cell::UnsafeCell;

use self::superblock::Superblock;
use self::dir::{DirOps, TYPE_FILE, TYPE_DIR};
use self::file::{BitmapOps, read_file_data, write_file_data};
use crate::disk::SECTOR_SIZE;

const NYXFS_DISK_SECTORS: u32 = 2880;

struct NyxFsState {
    mounted: bool,
    disk_offset: u32,
    sb: Superblock,
}

struct SafeNyxFs(UnsafeCell<Option<NyxFsState>>);
unsafe impl Sync for SafeNyxFs {}

static NYXFS: SafeNyxFs = SafeNyxFs(UnsafeCell::new(None));

fn state() -> Option<&'static NyxFsState> {
    unsafe { (*NYXFS.0.get()).as_ref() }
}

fn dir_ops(s: &NyxFsState) -> DirOps {
    DirOps {
        disk_offset: s.disk_offset,
        dir_lba: s.sb.root_dir_lba,
        dir_sectors: s.sb.root_dir_sectors,
    }
}

fn bitmap_ops(s: &NyxFsState) -> BitmapOps {
    BitmapOps {
        disk_offset: s.disk_offset,
        bitmap_lba: s.sb.bitmap_lba,
        bitmap_sectors: s.sb.bitmap_sectors,
        total_sectors: s.sb.total_sectors,
    }
}

pub fn init(disk_offset: u32) -> bool {
    if !crate::disk::is_available() {
        crate::logln!("[NyxFS] No disk available, skipping mount.");
        return false;
    }

    let sb = match superblock::read_superblock(disk_offset) {
        Some(sb) => {
            crate::logln!("[NyxFS] Valid superblock found on disk.");
            sb
        }
        None => {
            crate::logln!("[NyxFS] No valid superblock, formatting disk...");
            match superblock::format_disk(disk_offset, NYXFS_DISK_SECTORS) {
                Some(sb) => sb,
                None => {
                    crate::logln!("[NyxFS] Format failed!");
                    return false;
                }
            }
        }
    };

    unsafe {
        *NYXFS.0.get() = Some(NyxFsState {
            mounted: true,
            disk_offset,
            sb,
        });
    }

    crate::logln!(
        "[NyxFS] Mounted at disk offset {}, data at LBA {}, {} max entries.",
        disk_offset, sb.data_start_lba, sb.max_entries
    );
    true
}

pub fn is_mounted() -> bool {
    state().map(|s| s.mounted).unwrap_or(false)
}

pub fn list_root() -> Vec<(String, u8, u32)> {
    match state() {
        Some(s) => dir_ops(s).list_entries(),
        None => Vec::new(),
    }
}

pub fn read_file(name: &str) -> Option<Vec<u8>> {
    let s = state()?;
    let (_, entry) = dir_ops(s).find_entry(name)?;
    if entry.entry_type != TYPE_FILE {
        return None;
    }
    read_file_data(s.disk_offset, &s.sb, entry.start_block, entry.size)
}

pub fn write_file(name: &str, data: &[u8]) -> Result<(), i32> {
    let s = state().ok_or(-5)?;
    let ops = dir_ops(s);
    let bmap = bitmap_ops(s);

    let sectors_needed = if data.is_empty() { 1 } else {
        (data.len() + SECTOR_SIZE - 1) / SECTOR_SIZE
    };

    if let Some((idx, existing)) = ops.find_entry(name) {
        if existing.entry_type != TYPE_FILE {
            return Err(-21);
        }
        let old_sectors = if existing.size == 0 { 1 } else {
            (existing.size as usize + SECTOR_SIZE - 1) / SECTOR_SIZE
        };
        bmap.free_blocks(existing.start_block as usize, old_sectors);

        let block = bmap.alloc_blocks(sectors_needed).ok_or(-28)?;
        if !write_file_data(s.disk_offset, &s.sb, block, data) {
            bmap.free_blocks(block as usize, sectors_needed);
            return Err(-5);
        }
        let mut updated = existing;
        updated.start_block = block;
        updated.size = data.len() as u32;
        if !ops.write_entry(idx, &updated) {
            return Err(-5);
        }
        return Ok(());
    }

    let block = bmap.alloc_blocks(sectors_needed).ok_or(-28)?;
    if !write_file_data(s.disk_offset, &s.sb, block, data) {
        bmap.free_blocks(block as usize, sectors_needed);
        return Err(-5);
    }
    ops.create_entry(name, TYPE_FILE, block, data.len() as u32).ok_or(-28)?;
    Ok(())
}

pub fn delete_file(name: &str) -> Result<(), i32> {
    let s = state().ok_or(-5)?;
    let ops = dir_ops(s);
    let bmap = bitmap_ops(s);

    let (idx, entry) = ops.find_entry(name).ok_or(-2)?;
    if entry.entry_type != TYPE_FILE {
        return Err(-21);
    }

    let sectors = if entry.size == 0 { 1 } else {
        (entry.size as usize + SECTOR_SIZE - 1) / SECTOR_SIZE
    };
    bmap.free_blocks(entry.start_block as usize, sectors);
    if !ops.delete_entry(idx) {
        return Err(-5);
    }
    Ok(())
}

pub fn create_dir(name: &str) -> Result<(), i32> {
    let s = state().ok_or(-5)?;
    let ops = dir_ops(s);
    if ops.find_entry(name).is_some() {
        return Err(-17);
    }
    ops.create_entry(name, TYPE_DIR, 0, 0).ok_or(-28)?;
    Ok(())
}

pub fn delete_dir(name: &str) -> Result<(), i32> {
    let s = state().ok_or(-5)?;
    let ops = dir_ops(s);
    let (idx, entry) = ops.find_entry(name).ok_or(-2)?;
    if entry.entry_type != TYPE_DIR {
        return Err(-20);
    }
    if !ops.delete_entry(idx) {
        return Err(-5);
    }
    Ok(())
}

pub fn stat_entry(name: &str) -> Option<(u8, u32)> {
    let s = state()?;
    let (_, entry) = dir_ops(s).find_entry(name)?;
    Some((entry.entry_type, entry.size))
}

pub fn file_exists(name: &str) -> bool {
    match state() {
        Some(s) => dir_ops(s).find_entry(name).is_some(),
        None => false,
    }
}

pub fn read_file_offset(name: &str, offset: usize, buf: &mut [u8]) -> Result<usize, i32> {
    let s = state().ok_or(-5)?;
    let (_, entry) = dir_ops(s).find_entry(name).ok_or(-2)?;
    if entry.entry_type != TYPE_FILE {
        return Err(-21);
    }
    let file_size = entry.size as usize;
    if offset >= file_size {
        return Ok(0);
    }
    let available = file_size - offset;
    let to_read = available.min(buf.len());

    let start_sector = offset / SECTOR_SIZE;
    let offset_in_sector = offset % SECTOR_SIZE;
    let mut sector_buf = [0u8; SECTOR_SIZE];
    let mut bytes_read = 0;

    let mut sec = start_sector;
    let mut sec_offset = offset_in_sector;

    while bytes_read < to_read {
        let lba = s.disk_offset + s.sb.data_start_lba + entry.start_block as u32 + sec as u32;
        if !crate::disk::read_sector(lba, &mut sector_buf) {
            return Err(-5);
        }
        let chunk = (SECTOR_SIZE - sec_offset).min(to_read - bytes_read);
        buf[bytes_read..bytes_read + chunk].copy_from_slice(&sector_buf[sec_offset..sec_offset + chunk]);
        bytes_read += chunk;
        sec += 1;
        sec_offset = 0;
    }

    Ok(bytes_read)
}
