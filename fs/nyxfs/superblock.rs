use crate::disk::SECTOR_SIZE;

pub const NYXFS_MAGIC: u32 = 0x4E595846; // "NYXF"
pub const NYXFS_VERSION: u16 = 1;
pub const SUPERBLOCK_LBA: u32 = 0;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Superblock {
    pub magic: u32,
    pub version: u16,
    pub block_size: u16,
    pub total_sectors: u32,
    pub bitmap_lba: u32,
    pub bitmap_sectors: u32,
    pub root_dir_lba: u32,
    pub root_dir_sectors: u32,
    pub data_start_lba: u32,
    pub max_entries: u16,
    pub reserved: [u8; 474],
}

impl Superblock {
    pub fn is_valid(&self) -> bool {
        self.magic == NYXFS_MAGIC && self.version == NYXFS_VERSION
    }

    pub fn from_bytes(buf: &[u8; SECTOR_SIZE]) -> Self {
        unsafe { core::ptr::read(buf.as_ptr() as *const Self) }
    }

    pub fn to_bytes(&self) -> [u8; SECTOR_SIZE] {
        let mut buf = [0u8; SECTOR_SIZE];
        unsafe {
            core::ptr::copy_nonoverlapping(
                self as *const Self as *const u8,
                buf.as_mut_ptr(),
                core::mem::size_of::<Self>(),
            );
        }
        buf
    }

    pub fn create_default(total_sectors: u32) -> Self {
        let bitmap_sectors = (total_sectors + 512 * 8 - 1) / (512 * 8);
        let root_dir_sectors: u32 = 4;
        let max_entries = (root_dir_sectors * SECTOR_SIZE as u32 / 64) as u16;

        Self {
            magic: NYXFS_MAGIC,
            version: NYXFS_VERSION,
            block_size: SECTOR_SIZE as u16,
            total_sectors,
            bitmap_lba: 1,
            bitmap_sectors,
            root_dir_lba: 1 + bitmap_sectors,
            root_dir_sectors,
            data_start_lba: 1 + bitmap_sectors + root_dir_sectors,
            max_entries,
            reserved: [0; 474],
        }
    }
}

pub fn read_superblock(disk_offset: u32) -> Option<Superblock> {
    let mut buf = [0u8; SECTOR_SIZE];
    if !crate::disk::read_sector(disk_offset + SUPERBLOCK_LBA, &mut buf) {
        return None;
    }
    let sb = Superblock::from_bytes(&buf);
    if sb.is_valid() { Some(sb) } else { None }
}

pub fn write_superblock(disk_offset: u32, sb: &Superblock) -> bool {
    let buf = sb.to_bytes();
    crate::disk::write_sector(disk_offset + SUPERBLOCK_LBA, &buf)
}

pub fn format_disk(disk_offset: u32, total_sectors: u32) -> Option<Superblock> {
    let sb = Superblock::create_default(total_sectors);

    if !write_superblock(disk_offset, &sb) {
        return None;
    }

    let zero = [0u8; SECTOR_SIZE];
    for i in 0..sb.bitmap_sectors {
        if !crate::disk::write_sector(disk_offset + sb.bitmap_lba + i, &zero) {
            return None;
        }
    }

    for i in 0..sb.root_dir_sectors {
        if !crate::disk::write_sector(disk_offset + sb.root_dir_lba + i, &zero) {
            return None;
        }
    }

    let mut bitmap_buf = [0u8; SECTOR_SIZE];
    let reserved = sb.data_start_lba as usize;
    for bit in 0..reserved {
        let byte_idx = bit / 8;
        let bit_idx = bit % 8;
        if byte_idx < SECTOR_SIZE {
            bitmap_buf[byte_idx] |= 1 << bit_idx;
        }
    }
    if !crate::disk::write_sector(disk_offset + sb.bitmap_lba, &bitmap_buf) {
        return None;
    }

    crate::logln!("[NyxFS] Formatted disk: {} sectors, data starts at LBA {}.", total_sectors, sb.data_start_lba);
    Some(sb)
}
