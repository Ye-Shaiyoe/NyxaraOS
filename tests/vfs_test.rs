#![allow(dead_code)]

extern crate alloc;

#[macro_export]
macro_rules! logln {
    ($($arg:tt)*) => {};
}

pub mod disk {
    pub const SECTOR_SIZE: usize = 512;
    pub fn is_available() -> bool { false }
    pub fn read_sector(_lba: u32, _buf: &mut [u8; SECTOR_SIZE]) -> bool { false }
    pub fn write_sector(_lba: u32, _buf: &[u8; SECTOR_SIZE]) -> bool { false }
}

#[path = "../fs/nyxfs/mod.rs"]
pub mod nyxfs;

#[path = "../rust/src/vfs.rs"]
mod vfs;

use vfs::InodeType;

#[test]
fn supports_safe_file_and_directory_operations() {
    vfs::init();
    assert_eq!(vfs::current_dir(), "/");

    vfs::make_dir("projects").unwrap();
    vfs::make_dir("projects/src").unwrap();
    assert!(vfs::change_dir("projects/src").is_ok());
    assert_eq!(vfs::current_dir(), "/projects/src");

    vfs::write_file("main.rs", b"fn main() {}\n").unwrap();
    assert_eq!(
        vfs::read_file("./main.rs").as_deref(),
        Some(b"fn main() {}\n".as_slice())
    );
    assert_eq!(vfs::stat("main.rs").unwrap().1, InodeType::File);

    vfs::copy_file("main.rs", "copy.rs").unwrap();
    assert_eq!(vfs::read_file("copy.rs"), vfs::read_file("main.rs"));
    assert!(vfs::copy_file("main.rs", "copy.rs").is_err());

    vfs::move_file("copy.rs", "renamed.rs").unwrap();
    assert!(vfs::read_file("copy.rs").is_none());
    assert!(vfs::read_file("renamed.rs").is_some());

    assert!(vfs::remove_dir("/projects").is_err());
    vfs::remove_file("renamed.rs").unwrap();
    vfs::remove_file("main.rs").unwrap();
    vfs::change_dir("/").unwrap();
    vfs::remove_dir("/projects/src").unwrap();
    vfs::remove_dir("/projects").unwrap();
    assert!(vfs::stat("/projects").is_none());

    // Test list_dir on empty string and dot
    let root_entries = vfs::list_dir("").unwrap();
    assert!(root_entries.iter().any(|(name, _, kind)| name == "etc" && *kind == InodeType::Directory));
    assert!(root_entries.iter().any(|(name, _, kind)| name == "motd" && *kind == InodeType::File));

    let dot_entries = vfs::list_dir(".").unwrap();
    assert_eq!(root_entries.len(), dot_entries.len());

    let list_files = vfs::list_files();
    assert!(!list_files.is_empty());

    // Test stat on dot and slash
    let (stat_name, stat_kind, _) = vfs::stat(".").unwrap();
    assert_eq!(stat_name, "/");
    assert_eq!(stat_kind, InodeType::Directory);

    // Test copy to root directory
    vfs::make_dir("subdir").unwrap();
    vfs::write_file("subdir/sample.txt", b"sample data\n").unwrap();
    vfs::copy_file("subdir/sample.txt", "/").unwrap();
    assert_eq!(vfs::read_file("/sample.txt").as_deref(), Some(b"sample data\n".as_slice()));
    assert_eq!(vfs::read_file("sample.txt").as_deref(), Some(b"sample data\n".as_slice()));
    vfs::remove_file("/sample.txt").unwrap();
    vfs::remove_file("subdir/sample.txt").unwrap();
    vfs::remove_dir("subdir").unwrap();

    // Test make_dir_p
    vfs::make_dir_p("a/b/c").unwrap();
    assert!(vfs::change_dir("a/b/c").is_ok());
    assert_eq!(vfs::current_dir(), "/a/b/c");
    vfs::change_dir("/").unwrap();
    vfs::remove_dir("/a/b/c").unwrap();
    vfs::remove_dir("/a/b").unwrap();
    vfs::remove_dir("/a").unwrap();
}
