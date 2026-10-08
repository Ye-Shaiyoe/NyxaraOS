use crate::disk::SECTOR_SIZE;
use alloc::vec::Vec;

use super::superblock::Superblock;

pub fn read_file_data(disk_offset: u32, sb: &Superblock, start_block: u16, size: u32) -> Option<Vec<u8>> {
    if size == 0 {
        return Some(Vec::new());
    }

    let sectors_needed = (size as usize + SECTOR_SIZE - 1) / SECTOR_SIZE;
    let mut data = Vec::with_capacity(size as usize);
    let mut buf = [0u8; SECTOR_SIZE];

    for i in 0..sectors_needed {
        let lba = disk_offset + sb.data_start_lba + start_block as u32 + i as u32;
        if !crate::disk::read_sector(lba, &mut buf) {
            return None;
        }
        let remaining = size as usize - data.len();
        let to_copy = remaining.min(SECTOR_SIZE);
        data.extend_from_slice(&buf[..to_copy]);
    }

    Some(data)
}

pub fn write_file_data(disk_offset: u32, sb: &Superblock, start_block: u16, data: &[u8]) -> bool {
    if data.is_empty() {
        return true;
    }

    let sectors_needed = (data.len() + SECTOR_SIZE - 1) / SECTOR_SIZE;

    for i in 0..sectors_needed {
        let mut buf = [0u8; SECTOR_SIZE];
        let offset = i * SECTOR_SIZE;
        let remaining = data.len() - offset;
        let to_copy = remaining.min(SECTOR_SIZE);
        buf[..to_copy].copy_from_slice(&data[offset..offset + to_copy]);

        let lba = disk_offset + sb.data_start_lba + start_block as u32 + i as u32;
        if !crate::disk::write_sector(lba, &buf) {
            return false;
        }
    }

    true
}

pub struct BitmapOps {
    pub disk_offset: u32,
    pub bitmap_lba: u32,
    pub bitmap_sectors: u32,
    pub total_sectors: u32,
}

impl BitmapOps {
    pub fn alloc_blocks(&self, count: usize) -> Option<u16> {
        let mut buf = [0u8; SECTOR_SIZE];
        let bits_per_sector = SECTOR_SIZE * 8;

        for sec in 0..self.bitmap_sectors {
            if !crate::disk::read_sector(self.disk_offset + self.bitmap_lba + sec, &mut buf) {
                return None;
            }

            let base_bit = sec as usize * bits_per_sector;
            let mut run_start: Option<usize> = None;
            let mut run_len: usize = 0;

            for bit in 0..bits_per_sector {
                let abs_bit = base_bit + bit;
                if abs_bit >= self.total_sectors as usize {
                    break;
                }

                let byte_idx = bit / 8;
                let bit_idx = bit % 8;
                let used = (buf[byte_idx] >> bit_idx) & 1 != 0;

                if !used {
                    if run_start.is_none() {
                        run_start = Some(abs_bit);
                        run_len = 0;
                    }
                    run_len += 1;
                    if run_len >= count {
                        let start = run_start.unwrap();
                        self.mark_blocks(start, count, true);
                        return Some(start as u16);
                    }
                } else {
                    run_start = None;
                    run_len = 0;
                }
            }
        }
        None
    }

    pub fn free_blocks(&self, start: usize, count: usize) {
        self.mark_blocks(start, count, false);
    }

    fn mark_blocks(&self, start: usize, count: usize, used: bool) {
        let bits_per_sector = SECTOR_SIZE * 8;

        for i in 0..count {
            let bit = start + i;
            let sec = bit / bits_per_sector;
            let bit_in_sec = bit % bits_per_sector;
            let byte_idx = bit_in_sec / 8;
            let bit_idx = bit_in_sec % 8;

            let mut buf = [0u8; SECTOR_SIZE];
            let lba = self.disk_offset + self.bitmap_lba + sec as u32;
            if !crate::disk::read_sector(lba, &mut buf) {
                continue;
            }

            if used {
                buf[byte_idx] |= 1 << bit_idx;
            } else {
                buf[byte_idx] &= !(1 << bit_idx);
            }

            let _ = crate::disk::write_sector(lba, &buf);
        }
    }
}
