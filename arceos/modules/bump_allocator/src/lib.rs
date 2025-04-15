#![no_std]

use allocator::{BaseAllocator, ByteAllocator, PageAllocator, AllocResult};
use core::{alloc::Layout, mem, ptr::{null_mut, NonNull}};

/// Early memory allocator
/// Use it before formal bytes-allocator and pages-allocator can work!
/// This is a double-end memory range:
/// - Alloc bytes forward
/// - Alloc pages backward
///
/// [ bytes-used | avail-area | pages-used ]
/// |            | -->    <-- |            |
/// start       b_pos        p_pos       end
///
/// For bytes area, 'count' records number of allocations.
/// When it goes down to ZERO, free bytes-used area.
/// For pages area, it will never be freed!
///
pub struct EarlyAllocator<const PAGE_SIZE: usize> {
    total_size: usize,
    used_size: usize,
    left_index: usize,
    right_index: usize,
    free_list: *mut Block,
}

struct Block {
    size: usize,
    next: *mut Block,
}


unsafe impl<const PAGE_SIZE: usize> Sync for EarlyAllocator<PAGE_SIZE> {}
unsafe impl<const PAGE_SIZE: usize> Send for EarlyAllocator<PAGE_SIZE> {}

impl<const PAGE_SIZE: usize> EarlyAllocator<PAGE_SIZE> {
    pub const fn new() -> Self {
        Self { total_size: 0, used_size: 0, left_index: 0, right_index: 0, free_list: null_mut() }
    }

    unsafe fn init_free_list(&mut self, start: usize, size: usize) {
        self.left_index = start;
        self.right_index = start + size;
        self.used_size = 0;
        self.total_size = size;

        let block = start as *mut Block;
        (*block).size = size - mem::size_of::<Block>();
        (*block).next = null_mut();
        self.free_list = block;
    }

    unsafe fn split_block(block: *mut Block, required_size: usize) -> bool {
        let remaining_size = (*block).size - required_size;

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

    unsafe fn merge_blocks(&mut self) {
        let mut current = self.free_list;
        while !current.is_null() && !(*current).next.is_null() {
            let next = (*current).next;
            let current_end = (current as *mut u8).add(mem::size_of::<Block>() + (*current).size) as *mut Block;
            // 判断是否连续
            if current_end == next {
                (*current).size += mem::size_of::<Block>() + (*next).size;
                (*current).next = (*next).next;
            } else {
                // 切换下一个
                current = (*current).next;
            }
        }
    }

}

impl<const PAGE_SIZE: usize> BaseAllocator for EarlyAllocator<PAGE_SIZE> {
    fn init(&mut self, start: usize, size: usize) {
        unsafe {
            self.init_free_list(start, size);
        }
    }

    fn add_memory(&mut self, start: usize, size: usize) -> AllocResult {
        // 将新内存区域作为一个块添加到空闲链表
        unsafe {
            let new_block = start as *mut Block;
            (*new_block).size = size - mem::size_of::<Block>();
            (*new_block).next = self.free_list;
            self.free_list = new_block;
            
            self.total_size += size;
            self.merge_blocks();
            Ok(())
        }
    }
}

impl<const PAGE_SIZE: usize> ByteAllocator for EarlyAllocator<PAGE_SIZE> {
    fn alloc(&mut self, layout: Layout) -> AllocResult<NonNull<u8>> {
        unsafe {
            let required_size = layout.size().max(layout.align());
            if required_size + self.left_index >= self.right_index {
                return Err(allocator::AllocError::NoMemory)
            }
            let mut prev: *mut *mut Block = &mut self.free_list;
            let mut current = self.free_list;

            while !current.is_null() {
                if (*current).size >= required_size {
                    Self::split_block(current, required_size);
                    *prev = (*current).next;
                    let ptr = (current as *mut u8).add(mem::size_of::<Block>());
                    self.used_size += required_size;
                    self.left_index += required_size;
                    return Ok(NonNull::new(ptr).unwrap());
                }

                prev = &mut (*current).next;
                current = (*current).next;
            }
            Err(allocator::AllocError::NoMemory)
        }
    }

    fn dealloc(&mut self, pos: NonNull<u8>, layout: Layout) {
        unsafe {
            let size = layout.size().max(layout.align());
            self.used_size -= size;
            // 检查释放的内存是否在边界,回退字节区域
            if pos.as_ptr() as usize == self.left_index - size {
                self.left_index -= size;
            }
            let block = (pos.as_ptr() as *mut u8).sub(mem::size_of::<Block>()) as *mut Block;
            (*block).size = size;
            (*block).next = self.free_list;
            self.free_list = block;

            self.merge_blocks();
        }
    }

    fn available_bytes(&self) -> usize {
        self.total_size - self.used_size
    }

    fn total_bytes(&self) -> usize {
        self.total_size
    }

    fn used_bytes(&self) -> usize {
        self.used_size
    }
}

impl<const PAGE_SIZE: usize> PageAllocator for EarlyAllocator<PAGE_SIZE> {
    const PAGE_SIZE: usize = PAGE_SIZE;

    fn alloc_pages(&mut self, num_pages: usize, align_pow2: usize) -> AllocResult<usize> {
        if align_pow2 % Self::PAGE_SIZE != 0 {
            return Err(allocator::AllocError::InvalidParam);
        }
        let align_pow2 = align_pow2 / Self::PAGE_SIZE;
        if !align_pow2.is_power_of_two() {
            return Err(allocator::AllocError::InvalidParam);
        }

        let size = num_pages * Self::PAGE_SIZE;
        if self.right_index - size <= self.left_index {
            return Err(allocator::AllocError::NoMemory);
        }
        self.right_index -= size;
        self.used_size += size;
        Ok(self.right_index)
    }

    fn dealloc_pages(&mut self, pos: usize, num_pages: usize) {
        let size = num_pages * Self::PAGE_SIZE;
        if pos == self.right_index {
            self.right_index += size;
            self.used_size -= size;
        }

    }

    fn available_pages(&self) -> usize {
        self.total_size - self.used_size
    }

    fn total_pages(&self) -> usize {
        self.total_size
    }

    fn used_pages(&self) -> usize {
        self.used_size
    }
}

