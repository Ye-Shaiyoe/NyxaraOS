use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::UnsafeCell;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InodeType {
    File,
    Directory,
    CharDevice,
}

pub trait InodeOperations {
    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<usize, i32>;
    fn write(&mut self, offset: usize, buf: &[u8]) -> Result<usize, i32>;
    fn get_size(&self) -> usize;
}

pub struct MemoryInode {
    pub name: String,
    pub inode_type: InodeType,
    pub data: Vec<u8>,
}

impl InodeOperations for MemoryInode {
    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<usize, i32> {
        if offset >= self.data.len() {
            return Ok(0);
        }
        let available = self.data.len() - offset;
        let to_read = available.min(buf.len());
        buf[..to_read].copy_from_slice(&self.data[offset..offset + to_read]);
        Ok(to_read)
    }

    fn write(&mut self, offset: usize, buf: &[u8]) -> Result<usize, i32> {
        let needed_len = offset + buf.len();
        if needed_len > self.data.len() {
            self.data.resize(needed_len, 0);
        }
        self.data[offset..needed_len].copy_from_slice(buf);
        Ok(buf.len())
    }

    fn get_size(&self) -> usize {
        self.data.len()
    }
}

pub struct RamFs {
    files: Vec<MemoryInode>,
    current_dir: String,
}

struct SafeRamFs(UnsafeCell<Option<RamFs>>);
unsafe impl Sync for SafeRamFs {}

static RAMFS: SafeRamFs = SafeRamFs(UnsafeCell::new(None));

fn normalize_path(cwd: &str, input: &str) -> Option<String> {
    let input = if input.is_empty() { "." } else { input };

    let base = if input.starts_with('/') { "/" } else { cwd };
    let mut parts: Vec<&str> = Vec::new();
    for part in base.split('/').chain(input.split('/')) {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            value => parts.push(value),
        }
    }

    let mut path = String::from("/");
    for (index, part) in parts.iter().enumerate() {
        if index > 0 {
            path.push('/');
        }
        path.push_str(part);
    }
    Some(path)
}

fn parent_path(path: &str) -> &str {
    match path.rfind('/') {
        Some(0) | None => "/",
        Some(index) => &path[..index],
    }
}

fn base_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn destination_path(fs: &RamFs, source: &str, destination: &str) -> Option<String> {
    let path = normalize_path(&fs.current_dir, destination)?;
    if has_directory(fs, &path) {
        Some(format_path(path.as_str(), base_name(source)))
    } else {
        Some(path)
    }
}

fn format_path(directory: &str, name: &str) -> String {
    if directory == "/" {
        let mut path = String::from("/");
        path.push_str(name);
        path
    } else {
        let mut path = String::from(directory);
        path.push('/');
        path.push_str(name);
        path
    }
}

fn find_inode<'a>(fs: &'a RamFs, path: &str) -> Option<&'a MemoryInode> {
    fs.files.iter().find(|inode| inode.name == path)
}

fn has_directory(fs: &RamFs, path: &str) -> bool {
    path == "/"
        || find_inode(fs, path)
            .map(|inode| inode.inode_type == InodeType::Directory)
            .unwrap_or(false)
}

pub fn init() {
    let mut fs = RamFs {
        files: Vec::new(),
        current_dir: String::from("/"),
    };

    for directory in ["/etc", "/dev", "/bin", "/home", "/tmp"] {
        fs.files.push(MemoryInode {
            name: String::from(directory),
            inode_type: InodeType::Directory,
            data: Vec::new(),
        });
    }

    for (name, data) in [
        (
            "/motd",
            b"Welcome to Nyxara Unix-like Operating System!\n".as_slice(),
        ),
        (
            "/readme.txt",
            b"Nyxara kernel v2 with POSIX syscalls and VFS.\n".as_slice(),
        ),
    ] {
        fs.files.push(MemoryInode {
            name: String::from(name),
            inode_type: InodeType::File,
            data: data.to_vec(),
        });
    }

    unsafe {
        *RAMFS.0.get() = Some(fs);
    }
    sync_from_disk();
}

pub fn current_dir() -> String {
    unsafe {
        (*RAMFS.0.get())
            .as_ref()
            .map(|fs| fs.current_dir.clone())
            .unwrap_or_else(|| String::from("/"))
    }
}

pub fn change_dir(path: &str) -> Result<(), i32> {
    unsafe {
        let fs = (&mut *RAMFS.0.get()).as_mut().ok_or(-5)?;
        let target = normalize_path(&fs.current_dir, path).ok_or(-2)?;
        if !has_directory(fs, &target) {
            return Err(-2);
        }
        fs.current_dir = target;
        Ok(())
    }
}

pub fn make_dir(path: &str) -> Result<(), i32> {
    unsafe {
        let fs = (&mut *RAMFS.0.get()).as_mut().ok_or(-5)?;
        let target = normalize_path(&fs.current_dir, path).ok_or(-22)?;
        if target == "/" || find_inode(fs, &target).is_some() {
            return Err(-17);
        }
        if !has_directory(fs, parent_path(&target)) {
            return Err(-2);
        }
        fs.files.push(MemoryInode {
            name: target,
            inode_type: InodeType::Directory,
            data: Vec::new(),
        });
        Ok(())
    }
}

pub fn make_dir_p(path: &str) -> Result<(), i32> {
    unsafe {
        let fs = (&mut *RAMFS.0.get()).as_mut().ok_or(-5)?;
        let target = normalize_path(&fs.current_dir, path).ok_or(-22)?;
        if target == "/" {
            return Ok(());
        }
        let parts: Vec<&str> = target.split('/').filter(|s| !s.is_empty()).collect();
        let mut cur = String::from("");
        for part in parts {
            cur.push('/');
            cur.push_str(part);
            if !has_directory(fs, &cur) {
                if find_inode(fs, &cur).is_some() {
                    return Err(-20); // ENOTDIR
                }
                fs.files.push(MemoryInode {
                    name: cur.clone(),
                    inode_type: InodeType::Directory,
                    data: Vec::new(),
                });
            }
        }
        Ok(())
    }
}

pub fn list_dir(path: &str) -> Result<Vec<(String, usize, InodeType)>, i32> {
    unsafe {
        let fs = (&*RAMFS.0.get()).as_ref().ok_or(-5)?;
        let target = normalize_path(&fs.current_dir, path).ok_or(-2)?;
        if !has_directory(fs, &target) {
            return Err(-2);
        }

        let prefix = if target == "/" {
            String::from("/")
        } else {
            let mut value = target.clone();
            value.push('/');
            value
        };
        let mut entries = Vec::new();
        for inode in &fs.files {
            let Some(remainder) = inode.name.strip_prefix(&prefix) else {
                continue;
            };
            if remainder.is_empty() || remainder.contains('/') {
                continue;
            }
            entries.push((String::from(remainder), inode.get_size(), inode.inode_type));
        }
        Ok(entries)
    }
}

pub fn list_files() -> Vec<(String, usize)> {
    list_dir("")
        .unwrap_or_default()
        .into_iter()
        .map(|(name, size, _)| (name, size))
        .collect()
}

fn is_ramfs_only(path: &str) -> bool {
    path.starts_with("/tmp") || path.starts_with("/dev")
}

pub fn read_file(name: &str) -> Option<Vec<u8>> {
    let cur = current_dir();
    let path = normalize_path(&cur, name)?;
    if crate::nyxfs::is_mounted() && !is_ramfs_only(&path) {
        if let Some(data) = crate::nyxfs::read_file(&path) {
            return Some(data);
        }
    }
    unsafe {
        let fs = (*RAMFS.0.get()).as_ref()?;
        let inode = find_inode(fs, &path)?;
        if inode.inode_type != InodeType::File {
            return None;
        }
        Some(inode.data.clone())
    }
}

pub fn remove_file(name: &str) -> Result<(), i32> {
    let cur = current_dir();
    let path = normalize_path(&cur, name).ok_or(-2)?;
    if crate::nyxfs::is_mounted() && !is_ramfs_only(&path) {
        let _ = crate::nyxfs::delete_file(&path);
    }
    unsafe {
        let fs = (&mut *RAMFS.0.get()).as_mut().ok_or(-5)?;
        let Some(index) = fs.files.iter().position(|inode| inode.name == path) else {
            return Err(-2);
        };
        if fs.files[index].inode_type != InodeType::File {
            return Err(-21);
        }
        fs.files.remove(index);
        Ok(())
    }
}

pub fn remove_dir(name: &str) -> Result<(), i32> {
    unsafe {
        let fs = (&mut *RAMFS.0.get()).as_mut().ok_or(-5)?;
        let path = normalize_path(&fs.current_dir, name).ok_or(-2)?;
        if path == "/" || !has_directory(fs, &path) {
            return Err(-2);
        }
        if fs.current_dir == path || fs.current_dir.starts_with(&format!("{}/", path)) {
            return Err(-16);
        }
        let prefix = format!("{}/", path);
        if fs.files.iter().any(|inode| inode.name.starts_with(&prefix)) {
            return Err(-39);
        }
        let Some(index) = fs.files.iter().position(|inode| inode.name == path) else {
            return Err(-2);
        };
        fs.files.remove(index);
        Ok(())
    }
}

pub fn copy_file(source: &str, destination: &str) -> Result<(), i32> {
    unsafe {
        let fs = (&mut *RAMFS.0.get()).as_mut().ok_or(-5)?;
        let source_path = normalize_path(&fs.current_dir, source).ok_or(-2)?;
        let destination_path = destination_path(fs, &source_path, destination).ok_or(-22)?;
        let Some(source_inode) = find_inode(fs, &source_path) else {
            return Err(-2);
        };
        if source_inode.inode_type != InodeType::File {
            return Err(-21);
        }
        if find_inode(fs, &destination_path).is_some() {
            return Err(-17);
        }
        if !has_directory(fs, parent_path(&destination_path)) {
            return Err(-2);
        }
        fs.files.push(MemoryInode {
            name: destination_path,
            inode_type: InodeType::File,
            data: source_inode.data.clone(),
        });
        Ok(())
    }
}

pub fn move_file(source: &str, destination: &str) -> Result<(), i32> {
    unsafe {
        let fs = (&mut *RAMFS.0.get()).as_mut().ok_or(-5)?;
        let source_path = normalize_path(&fs.current_dir, source).ok_or(-2)?;
        let destination_path = destination_path(fs, &source_path, destination).ok_or(-22)?;
        let Some(source_index) = fs.files.iter().position(|inode| inode.name == source_path) else {
            return Err(-2);
        };
        if fs.files[source_index].inode_type != InodeType::File {
            return Err(-21);
        }
        if find_inode(fs, &destination_path).is_some() {
            return Err(-17);
        }
        if !has_directory(fs, parent_path(&destination_path)) {
            return Err(-2);
        }
        fs.files[source_index].name = destination_path;
        Ok(())
    }
}

pub fn stat(path: &str) -> Option<(String, InodeType, usize)> {
    unsafe {
        let fs = (*RAMFS.0.get()).as_ref()?;
        let target = normalize_path(&fs.current_dir, path).unwrap_or_else(|| String::from("/"));
        if target == "/" {
            return Some((target, InodeType::Directory, 0));
        }
        let inode = find_inode(fs, &target)?;
        Some((inode.name.clone(), inode.inode_type, inode.get_size()))
    }
}

pub fn write_file(name: &str, data: &[u8]) -> Result<(), i32> {
    let cur = current_dir();
    let path = normalize_path(&cur, name).ok_or(-22)?;
    if path == "/" {
        return Err(-2);
    }
    if crate::nyxfs::is_mounted() && !is_ramfs_only(&path) {
        crate::nyxfs::write_file(&path, data)?;
    }
    unsafe {
        let fs = (&mut *RAMFS.0.get()).as_mut().ok_or(-5)?;
        if !has_directory(fs, parent_path(&path)) {
            let _ = make_dir_p(parent_path(&path));
        }
        if let Some(inode) = fs.files.iter_mut().find(|inode| inode.name == path) {
            if inode.inode_type != InodeType::File {
                return Err(-21);
            }
            inode.data.clear();
            inode.data.extend_from_slice(data);
            return Ok(());
        }
        fs.files.push(MemoryInode {
            name: path,
            inode_type: InodeType::File,
            data: data.to_vec(),
        });
        Ok(())
    }
}

pub fn read_file_offset(name: &str, offset: usize, buf: &mut [u8]) -> Result<usize, i32> {
    let cur = current_dir();
    let path = normalize_path(&cur, name).ok_or(-2)?;
    if crate::nyxfs::is_mounted() && !is_ramfs_only(&path) {
        if let Ok(bytes) = crate::nyxfs::read_file_offset(&path, offset, buf) {
            return Ok(bytes);
        }
    }
    unsafe {
        let fs = (*RAMFS.0.get()).as_ref().ok_or(-5)?;
        let inode = find_inode(fs, &path).ok_or(-2)?;
        if inode.inode_type != InodeType::File {
            return Err(-21);
        }
        inode.read(offset, buf)
    }
}

pub fn write_file_ramfs_only(path: &str, data: &[u8]) -> Result<(), i32> {
    unsafe {
        let fs = (&mut *RAMFS.0.get()).as_mut().ok_or(-5)?;
        if !has_directory(fs, parent_path(path)) {
            let _ = make_dir_p(parent_path(path));
        }
        if let Some(inode) = fs.files.iter_mut().find(|inode| inode.name == path) {
            inode.data.clear();
            inode.data.extend_from_slice(data);
            return Ok(());
        }
        fs.files.push(MemoryInode {
            name: String::from(path),
            inode_type: InodeType::File,
            data: data.to_vec(),
        });
        Ok(())
    }
}

pub fn sync_from_disk() {
    if crate::nyxfs::is_mounted() {
        let entries = crate::nyxfs::list_root();
        for (name, entry_type, _) in entries {
            if entry_type == crate::nyxfs::dir::TYPE_FILE {
                if let Some(data) = crate::nyxfs::read_file(&name) {
                    let _ = write_file_ramfs_only(&name, &data);
                }
            } else if entry_type == crate::nyxfs::dir::TYPE_DIR {
                let _ = make_dir_p(&name);
            }
        }
    }
}
