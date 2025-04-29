//! `Arc<Inode>` -> `OSInodeInner`: In order to open files concurrently
//! we need to wrap `Inode` into `Arc`,but `Mutex` in `Inode` prevents
//! file systems from being accessed simultaneously
//!
//! `UPSafeCell<OSInodeInner>` -> `OSInode`: for static `ROOT_INODE`,we
//! need to wrap `OSInodeInner` into `UPSafeCell`
use super::{File, StatMode};
use crate::drivers::BLOCK_DEVICE;
use crate::mm::UserBuffer;
use crate::sync::UPSafeCell;
use alloc::{collections::btree_map::BTreeMap, sync::Arc};
use alloc::vec::Vec;
use bitflags::*;
use easy_fs::{EasyFileSystem, Inode};
use lazy_static::*;

/// inode in memory
/// A wrapper around a filesystem inode
/// to implement File trait atop
pub struct OSInode {
    readable: bool,
    writable: bool,
    inner: UPSafeCell<OSInodeInner>,
}
/// The OS inode inner in 'UPSafeCell'
pub struct OSInodeInner {
    offset: usize,
    mode: StatMode,
    inode: Arc<Inode>,
}

pub struct LinkManager {
    inode_to_link_count_map: BTreeMap<u32, u32>,
}


impl LinkManager {
    pub fn get_link_count(&mut self, inode_id: u32) -> u32 {
        if let Some(ret) = self.inode_to_link_count_map.get(&inode_id) {
            *ret
        } else {
            // 一开始加载的应用程序不会有链接计数
            self.increase_link_count(inode_id);
            1
        }
    }
    pub fn descrease_link_count(&mut self, inode_id: u32) -> Option<()> {
        if let Some(count) = self.inode_to_link_count_map.get_mut(&inode_id) {
            if *count > 1 {
                *count -= 1;
                Some(())
            } else {
                self.inode_to_link_count_map.remove(&inode_id);
                None
            }
        } else {
            None
        }
    }
    pub fn increase_link_count(&mut self, inode_id: u32) {
        if let Some(count) = self.inode_to_link_count_map.get_mut(&inode_id) {
            *count += 1;
        } else {
            self.inode_to_link_count_map.insert(inode_id, 1);
        }
    }
}

impl OSInode {
    /// create a new inode in memory
    pub fn new(readable: bool, writable: bool, inode: Arc<Inode>) -> Self {
        let mode = if inode.is_dir() {
            StatMode::DIR
        } else {
            StatMode::FILE
        };
        Self {
            readable,
            writable,
            inner: unsafe { UPSafeCell::new(OSInodeInner { offset: 0, mode, inode }) },
        }
    }
    /// read all data from the inode
    pub fn read_all(&self) -> Vec<u8> {
        let mut inner = self.inner.exclusive_access();
        let mut buffer: Vec<u8> = Vec::with_capacity(512);
        buffer.resize(512, 0);
        let mut v: Vec<u8> = Vec::new();
        loop {
            let len = inner.inode.read_at(inner.offset, &mut buffer);
            if len == 0 {
                break;
            }
            inner.offset += len;
            v.extend_from_slice(&buffer[..len]);
        }
        v
    }
}

lazy_static! {
    pub static ref ROOT_INODE: Arc<Inode> = {
        let efs = EasyFileSystem::open(BLOCK_DEVICE.clone());
        Arc::new(EasyFileSystem::root_inode(&efs))
    };
    pub static ref LINK_MANAGER: UPSafeCell<LinkManager> = {
        let mut inode_to_link_count_map = BTreeMap::new();
        inode_to_link_count_map.insert(ROOT_INODE.get_inode_id(), 1);
        unsafe {
            UPSafeCell::new(LinkManager {
                inode_to_link_count_map,
            })
        }
    };
}

/// List all apps in the root directory
pub fn list_apps() {
    println!("/**** APPS ****");
    for app in ROOT_INODE.ls() {
        println!("{}", app);
    }
    println!("**************/");
}

bitflags! {
    ///  The flags argument to the open() system call is constructed by ORing together zero or more of the following values:
    pub struct OpenFlags: u32 {
        /// readyonly
        const RDONLY = 0;
        /// writeonly
        const WRONLY = 1 << 0;
        /// read and write
        const RDWR = 1 << 1;
        /// create new file
        const CREATE = 1 << 9;
        /// truncate file size to 0
        const TRUNC = 1 << 10;
    }
}

impl OpenFlags {
    /// Do not check validity for simplicity
    /// Return (readable, writable)
    pub fn read_write(&self) -> (bool, bool) {
        if self.is_empty() {
            (true, false)
        } else if self.contains(Self::WRONLY) {
            (false, true)
        } else {
            (true, true)
        }
    }
}

/// Open a file
pub fn open_file(name: &str, flags: OpenFlags) -> Option<Arc<OSInode>> {
    let (readable, writable) = flags.read_write();
    if flags.contains(OpenFlags::CREATE) {
        if let Some(inode) = ROOT_INODE.find(name) {
            // clear size
            inode.clear();
            Some(Arc::new(OSInode::new(readable, writable, inode)))
        } else {
            // create file
            let ret = ROOT_INODE
                .create(name)
                .map(|inode| Arc::new(OSInode::new(readable, writable, inode)));
            LINK_MANAGER.exclusive_access().increase_link_count(ret.as_ref().unwrap().inner.exclusive_access().inode.get_inode_id());
            ret
        }
    } else {
        ROOT_INODE.find(name).map(|inode| {
            if flags.contains(OpenFlags::TRUNC) {
                inode.clear();
            }
            Arc::new(OSInode::new(readable, writable, inode))
        })
    }
}

/// linkat syscall
pub fn linkat(old_name: &str, new_name: &str) -> isize {
    // 判断是否同名
    if old_name == new_name {
        return -1;
    }
    // 判断是否存在
    if let Some(inode) = ROOT_INODE.find(old_name) {
        // 判断是否已经存在
        if let Some(_) = ROOT_INODE.find(new_name) {
            return -1;
        }
        // 创建新文件
        ROOT_INODE.linkat(inode.get_inode_id(), new_name);
        // list_apps();
        // println!("kernel: sys_linkat old_name: {}, new_name: {}", old_name, new_name);
        LINK_MANAGER.exclusive_access().increase_link_count(inode.get_inode_id());
        0
    } else {
        return -1;
    }
}

/// unlink syscall
pub fn unlink(name: &str) -> isize{
    // 判断是否存在
    if let Some(inode) = ROOT_INODE.find(name) {
        // 减少链接计数
        if LINK_MANAGER.exclusive_access().descrease_link_count(inode.get_inode_id()).is_none() {
            // 删除文件
            ROOT_INODE.unlink(name, true);
        }
        else {
            // 删除文件
            ROOT_INODE.unlink(name, false);
        }
        // list_apps();
        0
    } else {
        -1
    }
}

impl File for OSInode {
    fn readable(&self) -> bool {
        self.readable
    }
    fn writable(&self) -> bool {
        self.writable
    }
    fn read(&self, mut buf: UserBuffer) -> usize {
        let mut inner = self.inner.exclusive_access();
        let mut total_read_size = 0usize;
        for slice in buf.buffers.iter_mut() {
            let read_size = inner.inode.read_at(inner.offset, *slice);
            if read_size == 0 {
                break;
            }
            inner.offset += read_size;
            total_read_size += read_size;
        }
        total_read_size
    }
    fn write(&self, buf: UserBuffer) -> usize {
        let mut inner = self.inner.exclusive_access();
        let mut total_write_size = 0usize;
        for slice in buf.buffers.iter() {
            let write_size = inner.inode.write_at(inner.offset, *slice);
            assert_eq!(write_size, slice.len());
            inner.offset += write_size;
            total_write_size += write_size;
        }
        total_write_size
    }
    fn stat(&self) -> super::Stat {
        let inner = self.inner.exclusive_access();
        super::Stat {
            dev: 0,
            ino: inner.inode.get_inode_id() as u64,
            mode: inner.mode,
            nlink: LINK_MANAGER
                .exclusive_access()
                .get_link_count(inner.inode.get_inode_id()),
            pad: [0; 7],
        }
    }
}
