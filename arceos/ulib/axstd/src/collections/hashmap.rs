// use axalloc::global_allocator;
use core::hash::{Hash, Hasher};
extern crate alloc;
use alloc::vec::Vec;
use axhal::misc::random;
use core::slice;

pub struct HashMap<K, V> {
    buckets: Vec<Option<(K, V)>>,
    len: usize,                    // 实际元素数量
    capacity: usize,               // 桶的总数
}
impl<K, V> HashMap<K, V>
where 
    K: Hash + Eq
{
    const INITIAL_CAPACITY: usize = 16;

    /// 分配桶空间
    fn allocate_buckets(capacity: usize) -> Vec<Option<(K, V)>> {
        let mut buckets = Vec::with_capacity(capacity);
        buckets.resize_with(capacity, || None);
        buckets
    }

    pub fn new() -> Self {
        Self {
            buckets: Self::allocate_buckets(Self::INITIAL_CAPACITY),
            len: 0,
            capacity: Self::INITIAL_CAPACITY,
        }
    }

    /// 插入键值对
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        // 负载因子超过 0.75 时扩容
        if self.len >= self.capacity * 3 / 4 {
            self.resize();
        }

        let mut index = self.hash(&key);
        loop {
            index &= self.capacity - 1;  // 等价于 index % self.capacity
            match &mut self.buckets[index] {
                Some((k, v)) if *k == key => {
                    // 替换现有值
                    return Some(core::mem::replace(v, value));
                }
                slot @ None => {
                    // 插入新值
                    *slot = Some((key, value));
                    self.len += 1;
                    return None;
                }
                _ => index += 1,  // 线性探测
            }
        }
    }

      /// 查找键
      pub fn get(&self, key: &K) -> Option<&V> {
        let mut index = self.hash(key);
        loop {
            index &= self.capacity - 1;
            match &self.buckets[index] {
                Some((k, v)) if k == key => return Some(v),
                None => return None,
                _ => index += 1,
            }
        }
    }

    /// 哈希函数
    fn hash(&self, key: &K) -> usize {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish() as usize
    }

    /// 扩容（重建哈希表）
    fn resize(&mut self) {
        let new_capacity = self.capacity * 2;
        let mut new_map = Self {
            buckets: Self::allocate_buckets(new_capacity),
            len: 0,
            capacity: new_capacity,
        };

        // 迁移旧数据
        for slot in self.buckets.drain(..) {
            if let Some((k, v)) = slot {
                new_map.insert(k, v);
            }
        }
        *self = new_map;
    }


    /// 不可变迭代器
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            inner: self.buckets.iter()
        }
    }
}
pub struct Iter<'a, K, V> {
    inner: slice::Iter<'a, Option<(K, V)>>,
}
impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.inner.next()? {
                Some((k, v)) => return Some((k, v)),
                None => continue, // 跳过空桶
            }
        }
    }
}
pub struct DefaultHasher(u64);

impl DefaultHasher {
    pub fn new() -> Self {
        Self(random() as u64)
    }
}

impl Hasher for DefaultHasher {
    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 = self.0.wrapping_mul(0x1000193).wrapping_add(byte as u64);
        }
    }

    fn finish(&self) -> u64 {
        self.0
    }
}