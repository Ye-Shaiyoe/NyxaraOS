# NyxFS Persistent Filesystem Architecture

NyxFS is a lightweight, persistent disk filesystem designed for NyxaraOS. It bridges the Virtual File System (VFS) with real hardware block storage (ATA PIO / virtual IDE disks).

## 1. Disk Layout

A standard NyxFS volume (e.g. 1.44 MB = 2880 sectors of 512 bytes each) is structured as follows:

| Sector / LBA | Region | Description |
|---|---|---|
| **LBA 0** | Superblock | Magic `0x4E595846` ("NYXF"), version, total sectors, layout offsets |
| **LBA 1** | Block Bitmap | 1 bit per 512-byte sector tracking allocation status |
| **LBA 2..5** | Root Directory | Fixed-size directory entries (64 bytes each, 32 entries) |
| **LBA 6..2879** | Data Blocks | Raw file contents and allocated block clusters |

### Directory Entry Structure (64 Bytes)
```rust
#[repr(C)]
pub struct DirEntry {
    pub name: [u8; 32],        // Null-terminated filename (max 31 chars)
    pub entry_type: u8,        // 0 = Free, 1 = File, 2 = Directory
    pub start_block: u16,      // First data sector index
    _pad: u8,
    pub size: u32,             // File size in bytes
    pub parent_block: u16,     // Parent directory block pointer
    pub reserved: [u8; 22],
}
```

## 2. Low-Level Storage Layer (`hal/ata.c` & `rust/src/disk.rs`)

NyxaraOS uses an ATA PIO (Port I/O) driver on the Primary ATA Bus (`0x1F0`..`0x1F7`):
- **Drive Selection**: Prioritizes secondary data disk (IDE Slave `0xB0`) to prevent modifying the kernel image (`nyxara.img` on Master `0xA0`).
- **Read Operations**: `ata_read_sectors(lba, count, buf)` reads 512 bytes per sector via `inw(0x1F0)` after polling for DRQ.
- **Write Operations**: `ata_write_sectors(lba, count, buf)` writes 512 bytes per sector via `outw(0x1F0)` followed by `ATA_CMD_FLUSH` (`0xE7`).

## 3. File Descriptor Table (`rust/src/fd.rs`)

Each process holds an isolated `FdTable` tracking open files:
- `fd 0`: Standard Input (Keyboard buffer)
- `fd 1`: Standard Output (VGA Console & Serial)
- `fd 2`: Standard Error (VGA Console & Serial)
- `fd 3..31`: Open disk/VFS files with independent seek offsets and access flags (`O_RDONLY`, `O_WRONLY`, `O_RDWR`, `O_CREAT`).

## 4. System Calls (`rust/src/syscall.rs`)

Userland processes interact with the filesystem via POSIX-compliant `int 0x80` syscalls:
- `SYS_OPEN` (5): `sys_open(path, flags) -> fd`
- `SYS_CLOSE` (6): `sys_close(fd) -> 0`
- `SYS_READ` (3): `sys_read(fd, buf, count) -> bytes_read`
- `SYS_WRITE` (4): `sys_write(fd, buf, count) -> bytes_written`
