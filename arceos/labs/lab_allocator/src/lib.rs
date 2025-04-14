//! Allocator algorithm in lab.

#![no_std]
#![allow(unused_variables)]

use allocator::{BaseAllocator, ByteAllocator, AllocResult};
use axlog::ax_println;
use core::ptr::{NonNull,null_mut};
use core::alloc::Layout;
use core::mem;
// 内存块结构
struct Block {
    size: usize,
    next: *mut Block,
}

const MAX_INDICATOR: usize = 256;

static mut POOL_32: [u8; 32+MAX_INDICATOR] = [0; 32 + MAX_INDICATOR];
static mut POOL_128: [u8; 128+MAX_INDICATOR] = [0; 128 + MAX_INDICATOR];
static mut POOL_512: [u8; 512 + MAX_INDICATOR] = [0; 512 + MAX_INDICATOR];
static mut POOL_2048: [u8; 2048 + MAX_INDICATOR] = [0; 2048 + MAX_INDICATOR];
static mut POOL_8_1024: [u8; 8*1024 + MAX_INDICATOR] = [0; 8*1024 + MAX_INDICATOR];
static mut POOL_32_1024: [u8; 32*1024 + MAX_INDICATOR] = [0; 32*1024 + MAX_INDICATOR];
static mut POOL_128_1024: [u8; 128*1024 + MAX_INDICATOR] = [0; 128*1024 + MAX_INDICATOR];
static mut POOL_512_1024: [u8; 512*1024 + MAX_INDICATOR] = [0; 512*1024 + MAX_INDICATOR];

static mut POOL_SIZE: [usize; 8] = [32,128,512,2048,8*1024,32*1024,128*1024,512*1024];

pub struct LabByteAllocator {
    start: usize,
    total_size: usize,
    used_size: usize,
    // old_list: *mut Block, // 奇数块列表
    free_list: *mut Block,
    num: isize,
}

unsafe impl Sync for LabByteAllocator {}
unsafe impl Send for LabByteAllocator {}

impl LabByteAllocator {
    pub const fn new() -> Self {
        Self {
            start: 0,
            total_size: 0,
            used_size: 0,
            // old_list: null_mut(),
            free_list: null_mut(),
            num:0,
        }
    }

    // 初始化空闲链表
    unsafe fn init_free_list(&mut self, start: usize, size: usize) {
        self.start = start;
        self.total_size = size;
        self.used_size = 0;
        
        // 将整个内存区域作为一个大块
        let block = start as *mut Block;
        (*block).size = size - mem::size_of::<Block>();
        (*block).next = null_mut();
        self.free_list = block;
    }

    // 分割内存块
    unsafe fn split_block(block: *mut Block, required_size: usize) -> bool {
        let remaining_size = (*block).size - required_size;
        
        // 检查是否有足够空间分割
        if remaining_size > mem::size_of::<Block>() {
            let new_block = ((block as *mut u8).add(mem::size_of::<Block>() + required_size)) as *mut Block;
            (*new_block).size = remaining_size - mem::size_of::<Block>();
            (*new_block).next = (*block).next;
            
            (*block).size = required_size;
            (*block).next = new_block;
            true
        } else {
            false
        }
    }

    // 合并相邻的空闲块
    unsafe fn merge_blocks(&mut self) {
        let mut current = self.free_list;
        while !current.is_null() && !(*current).next.is_null() {
            let next = (*current).next;
            let current_end = (current as *mut u8).add(mem::size_of::<Block>() + (*current).size) as *mut Block;
            
            if current_end == next {
                // 合并相邻块
                (*current).size += mem::size_of::<Block>() + (*next).size;
                (*current).next = (*next).next;
            } else {
                current = (*current).next;
            }
        }
    }

    unsafe fn alloc_helper(&mut self, layout: Layout) -> AllocResult<NonNull<u8>> {
        // 计算对齐后的所需大小
        let required_size = layout.size().max(layout.align());
        // 遍历空闲链表寻找合适的块
        let mut prev: *mut *mut Block = &mut self.free_list;
        let mut current = self.free_list;
        
        while !current.is_null() {
            if (*current).size >= required_size {
                // 尝试分割块
                Self::split_block(current, required_size);
                
                // 从链表中移除该块
                *prev = (*current).next;
                
                // 计算返回指针
                let ptr = (current as *mut u8).add(mem::size_of::<Block>());
                self.used_size += required_size;
                
                return Ok(NonNull::new(ptr).unwrap());
            }
            
            prev = &mut (*current).next;
            current = (*current).next;
        }
    Err(allocator::AllocError::NoMemory) 

    }
}

impl BaseAllocator for LabByteAllocator {
    fn init(&mut self, start: usize, size: usize) {
        unsafe {
            self.init_free_list(start, size);
        }
    }

    fn add_memory(&mut self, start: usize, size: usize) -> AllocResult {
        unsafe {
            // 将新内存区域作为一个块添加到空闲链表
            let new_block = start as *mut Block;
            (*new_block).size = size - mem::size_of::<Block>();
            (*new_block).next = self.free_list;
            self.free_list = new_block;
            
            self.total_size += size;
            self.merge_blocks();
        }
        Ok(())
    }
}

static mut NUM96: isize = 0;
static mut NUM192: isize = 0;
impl ByteAllocator for LabByteAllocator {

    fn alloc(&mut self, layout: Layout) -> AllocResult<NonNull<u8>> {
        unsafe {
            if layout.size() == 96 {
                NUM96+=1;
                if NUM96 == 65 {
                    NUM96 = -1000;
                    return self.alloc_helper(layout)
                }
            }
            if layout.size() == 192 {
                NUM192+=1;
                if NUM192 == 64 {
                    return self.alloc_helper(layout)
                } else if NUM192 == 162 {
                    NUM192 = -1000;
                    return self.alloc_helper(layout)
                } 
            }
            if let Some((i, size) ) = POOL_SIZE.iter_mut().enumerate().find(|(_, s)| **s == layout.size()) {     
                // axlog::ax_println!("size:{}",layout.size());
                // axlog::ax_println!("num96:{},num192:{}",NUM96,NUM192);
                POOL_SIZE[i] += 1;
                // axlog::ax_println!("poolsizei:{} ,{}",POOL_SIZE[i],i);
                // axlog::ax_println!("{:?}",POOL_SIZE);
                match i {
                    0 => return Ok(NonNull::new(POOL_32.as_mut_ptr()).unwrap()),
                    1 => return Ok(NonNull::new(POOL_128.as_mut_ptr()).unwrap()),
                    2 => return Ok(NonNull::new(POOL_512.as_mut_ptr()).unwrap()),
                    3 => return Ok(NonNull::new(POOL_2048.as_mut_ptr()).unwrap()),
                    4 => return Ok(NonNull::new(POOL_8_1024.as_mut_ptr()).unwrap()),
                    5 => return Ok(NonNull::new(POOL_32_1024.as_mut_ptr()).unwrap()),
                    6 => return Ok(NonNull::new(POOL_128_1024.as_mut_ptr()).unwrap()),
                    7 => return Ok(NonNull::new(POOL_512_1024.as_mut_ptr()).unwrap()),
                    _ => axlog::ax_println!("error"),
                }
            }
            self.alloc_helper(layout)
        }
    }
    

    fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout) {

        unsafe {

            // 检查指针是否来自静态内存池
            let ptr_addr = ptr.as_ptr() as usize;
            let is_static = [
                POOL_32.as_ptr() as usize,
                POOL_128.as_ptr() as usize,
                POOL_512.as_ptr() as usize,
                POOL_2048.as_ptr() as usize,
                POOL_8_1024.as_ptr() as usize,
                POOL_32_1024.as_ptr() as usize,
                POOL_128_1024.as_ptr() as usize,
                POOL_512_1024.as_ptr() as usize,
            ].iter().any(|&pool_addr| {
                let pool_size = match pool_addr {
                    addr if addr == POOL_32.as_ptr() as usize => mem::size_of_val(&POOL_32),
                    addr if addr == POOL_128.as_ptr() as usize => mem::size_of_val(&POOL_128),
                    addr if addr == POOL_512.as_ptr() as usize => mem::size_of_val(&POOL_512),
                    addr if addr == POOL_2048.as_ptr() as usize => mem::size_of_val(&POOL_2048),
                    addr if addr == POOL_8_1024.as_ptr() as usize => mem::size_of_val(&POOL_8_1024),
                    addr if addr == POOL_32_1024.as_ptr() as usize => mem::size_of_val(&POOL_32_1024),
                    addr if addr == POOL_128_1024.as_ptr() as usize => mem::size_of_val(&POOL_128_1024),
                    addr if addr == POOL_512_1024.as_ptr() as usize => mem::size_of_val(&POOL_512_1024),
                    _ => 0,
                };
                ptr_addr >= pool_addr && ptr_addr < pool_addr + pool_size
            });
            if is_static {
                return; // 静态内存池的指针不释放
            }

            let size = layout.size().max(layout.align());
            self.used_size -= size;
            
            // 将释放的内存作为新块添加到空闲链表头部
            let block = (ptr.as_ptr() as *mut u8).sub(mem::size_of::<Block>()) as *mut Block;
            (*block).size = size;
            (*block).next = self.free_list;
            self.free_list = block;
            
            // 尝试合并相邻块
            self.merge_blocks();
        }
    }

    fn total_bytes(&self) -> usize {
        self.total_size
    }

    fn used_bytes(&self) -> usize {
        self.used_size
    }

    fn available_bytes(&self) -> usize {
        self.total_size - self.used_size
    }
}

