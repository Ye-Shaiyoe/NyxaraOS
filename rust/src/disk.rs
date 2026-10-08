pub const SECTOR_SIZE: usize = 512;

extern "C" {
    fn ata_identify() -> i32;
    fn ata_read_sectors(lba: u32, count: u8, buf: *mut u8) -> i32;
    fn ata_write_sectors(lba: u32, count: u8, buf: *const u8) -> i32;
}

static mut DISK_AVAILABLE: bool = false;

pub fn init() -> bool {
    let sectors = unsafe { ata_identify() };
    if sectors > 0 {
        unsafe { DISK_AVAILABLE = true };
        crate::logln!("[Disk] ATA disk detected: {} sectors ({} KB).", sectors, sectors as u32 * 512 / 1024);
        true
    } else {
        crate::logln!("[Disk] No ATA disk detected.");
        false
    }
}

pub fn is_available() -> bool {
    unsafe { DISK_AVAILABLE }
}

pub fn read_sector(lba: u32, buf: &mut [u8; SECTOR_SIZE]) -> bool {
    if !is_available() {
        return false;
    }
    let ret = unsafe { ata_read_sectors(lba, 1, buf.as_mut_ptr()) };
    ret == 1
}

pub fn write_sector(lba: u32, buf: &[u8; SECTOR_SIZE]) -> bool {
    if !is_available() {
        return false;
    }
    let ret = unsafe { ata_write_sectors(lba, 1, buf.as_ptr()) };
    ret == 1
}

pub fn read_sectors(lba: u32, count: u8, buf: &mut [u8]) -> bool {
    if !is_available() || buf.len() < count as usize * SECTOR_SIZE {
        return false;
    }
    let ret = unsafe { ata_read_sectors(lba, count, buf.as_mut_ptr()) };
    ret == count as i32
}

pub fn write_sectors(lba: u32, count: u8, buf: &[u8]) -> bool {
    if !is_available() || buf.len() < count as usize * SECTOR_SIZE {
        return false;
    }
    let ret = unsafe { ata_write_sectors(lba, count, buf.as_ptr()) };
    ret == count as i32
}
