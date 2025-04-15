extern crate alloc;

use alloc::sync::Arc;
use axfs_ramfs::{RamFileSystem,FileNode};
use axfs_vfs::{VfsNodeOps,VfsOps};
use std::os::arceos::api::fs::{AxDisk, MyFileSystemIf};
use std::fs;
struct MyFileSystemIfImpl;

#[crate_interface::impl_interface]
impl MyFileSystemIf for MyFileSystemIfImpl {
    fn new_myfs(_disk: AxDisk) -> Arc<dyn VfsOps> {
        Arc::new(RamFileSystem::new())
    }
}
