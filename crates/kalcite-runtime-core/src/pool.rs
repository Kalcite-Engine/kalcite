use core::{marker::PhantomData, mem::MaybeUninit};

pub struct SignalQueue<T, const N: usize> {
    slots: [Option<T>; N],
    head: usize,
    len: usize,
}

impl<T, const N: usize> SignalQueue<T, N> {
    pub const fn new() -> Self {
        Self {
            slots: [const { None }; N],
            head: 0,
            len: 0,
        }
    }

    pub fn push(&mut self, value: T) -> bool {
        if self.len == N || N == 0 {
            return false;
        }
        let tail = (self.head + self.len) % N;
        self.slots[tail] = Some(value);
        self.len += 1;
        true
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        let value = self.slots[self.head].take();
        self.head = (self.head + 1) % N;
        self.len -= 1;
        value
    }
}

impl<T, const N: usize> Default for SignalQueue<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Compact, typed, generational reference to an object in a StaticPool.
/// 0xffff is reserved as the invalid slot index.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Handle<T> {
    index: u16,
    generation: u16,
    marker: PhantomData<fn() -> T>,
}
impl<T> Copy for Handle<T> {}
impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Handle<T> {
    pub const INVALID: Self = Self {
        index: u16::MAX,
        generation: 0,
        marker: PhantomData,
    };
    pub const fn invalid() -> Self {
        Self::INVALID
    }
    pub const fn is_valid(self) -> bool {
        self.index != u16::MAX
    }
    pub const fn index(self) -> Option<usize> {
        if self.is_valid() {
            Some(self.index as usize)
        } else {
            None
        }
    }
}
impl<T> Default for Handle<T> {
    fn default() -> Self {
        Self::INVALID
    }
}

struct Slot<T> {
    generation: u16,
    occupied: bool,
    value: MaybeUninit<T>,
}

/// Fixed-capacity object storage. It never allocates and rejects stale handles.
pub struct StaticPool<T, const N: usize> {
    slots: [Slot<T>; N],
    len: usize,
}
impl<T, const N: usize> StaticPool<T, N> {
    pub fn new() -> Self {
        Self {
            slots: core::array::from_fn(|_| Slot {
                generation: 1,
                occupied: false,
                value: MaybeUninit::uninit(),
            }),
            len: 0,
        }
    }
    pub const fn capacity(&self) -> usize {
        N
    }
    pub const fn len(&self) -> usize {
        self.len
    }
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub const fn is_full(&self) -> bool {
        self.len == N
    }

    /// Returns Handle::INVALID instead of allocating/failing catastrophically.
    pub fn spawn(&mut self, value: T) -> Handle<T> {
        for (index, slot) in self.slots.iter_mut().enumerate() {
            if !slot.occupied {
                slot.value.write(value);
                slot.occupied = true;
                self.len += 1;
                return Handle {
                    index: index as u16,
                    generation: slot.generation,
                    marker: PhantomData,
                };
            }
        }
        Handle::INVALID
    }
    pub fn contains(&self, handle: Handle<T>) -> bool {
        self.get(handle).is_some()
    }
    pub fn get(&self, handle: Handle<T>) -> Option<&T> {
        let slot = self.slots.get(handle.index as usize)?;
        if slot.occupied && slot.generation == handle.generation {
            Some(unsafe { slot.value.assume_init_ref() })
        } else {
            None
        }
    }
    pub fn get_mut(&mut self, handle: Handle<T>) -> Option<&mut T> {
        let slot = self.slots.get_mut(handle.index as usize)?;
        if slot.occupied && slot.generation == handle.generation {
            Some(unsafe { slot.value.assume_init_mut() })
        } else {
            None
        }
    }
    pub fn despawn(&mut self, handle: Handle<T>) -> bool {
        let Some(slot) = self.slots.get_mut(handle.index as usize) else {
            return false;
        };
        if !slot.occupied || slot.generation != handle.generation {
            return false;
        }
        unsafe {
            slot.value.assume_init_drop();
        }
        slot.occupied = false;
        slot.generation = slot.generation.wrapping_add(1).max(1);
        self.len -= 1;
        true
    }
    pub fn for_each_mut(&mut self, mut f: impl FnMut(&mut T)) {
        for slot in &mut self.slots {
            if slot.occupied {
                f(unsafe { slot.value.assume_init_mut() });
            }
        }
    }
}
impl<T, const N: usize> Default for StaticPool<T, N> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T, const N: usize> Drop for StaticPool<T, N> {
    fn drop(&mut self) {
        for slot in &mut self.slots {
            if slot.occupied {
                unsafe {
                    slot.value.assume_init_drop();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    #[test]
    fn stale_handle_is_rejected() {
        let mut pool: StaticPool<u8, 1> = StaticPool::new();
        let old = pool.spawn(7);
        assert!(pool.despawn(old));
        let new = pool.spawn(9);
        assert!(new.is_valid());
        assert!(pool.get(old).is_none());
        assert_eq!(pool.get(new), Some(&9));
    }
    #[test]
    fn full_pool_returns_invalid_handle() {
        let mut pool: StaticPool<u8, 1> = StaticPool::new();
        assert!(pool.spawn(1).is_valid());
        assert!(!pool.spawn(2).is_valid());
    }

    #[test]
    fn signal_queue_is_bounded_and_fifo() {
        let mut queue: SignalQueue<u8, 2> = SignalQueue::new();
        assert!(queue.push(1));
        assert!(queue.push(2));
        assert!(!queue.push(3));
        assert_eq!(queue.pop(), Some(1));
        assert!(queue.push(3));
        assert_eq!(queue.pop(), Some(2));
        assert_eq!(queue.pop(), Some(3));
        assert_eq!(queue.pop(), None);
    }
}
