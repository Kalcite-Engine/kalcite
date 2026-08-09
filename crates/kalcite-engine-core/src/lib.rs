#![no_std]
use core::mem::MaybeUninit;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Vec2i {
    pub x: i16,
    pub y: i16,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: i16,
    pub y: i16,
    pub w: i16,
    pub h: i16,
}
impl Rect {
    pub fn intersects(self, o: Self) -> bool {
        self.x < o.x + o.w && self.x + self.w > o.x && self.y < o.y + o.h && self.y + self.h > o.y
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Color565(pub u16);
impl Color565 {
    pub const BLACK: Self = Self(0);
    pub const WHITE: Self = Self(0xffff);
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self(
            (((r as u16 >> 3) & 31) << 11) | (((g as u16 >> 2) & 63) << 5) | ((b as u16 >> 3) & 31),
        )
    }
}

pub struct Canvas<'a> {
    pixels: &'a mut [u16],
    width: i16,
    height: i16,
}
impl<'a> Canvas<'a> {
    pub fn new(p: &'a mut [u16], w: u16, h: u16) -> Option<Self> {
        if p.len() < w as usize * h as usize {
            return None;
        }
        Some(Self {
            pixels: p,
            width: w as i16,
            height: h as i16,
        })
    }
    pub fn clear(&mut self, c: Color565) {
        self.pixels[..self.width as usize * self.height as usize].fill(c.0)
    }
    pub fn pixel(&mut self, x: i16, y: i16, c: Color565) {
        if x >= 0 && y >= 0 && x < self.width && y < self.height {
            self.pixels[y as usize * self.width as usize + x as usize] = c.0
        }
    }
    pub fn rect(&mut self, r: Rect, c: Color565) {
        let x0 = r.x.max(0);
        let y0 = r.y.max(0);
        let x1 = (r.x + r.w).min(self.width);
        let y1 = (r.y + r.h).min(self.height);
        for y in y0..y1 {
            let row = y as usize * self.width as usize;
            for x in x0..x1 {
                self.pixels[row + x as usize] = c.0
            }
        }
    }
    pub fn pixels(&self) -> &[u16] {
        self.pixels
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handle {
    index: u16,
    generation: u16,
}
struct Slot<T> {
    generation: u16,
    occupied: bool,
    value: MaybeUninit<T>,
}
pub struct Pool<T, const N: usize> {
    slots: [Slot<T>; N],
    len: usize,
}
impl<T, const N: usize> Pool<T, N> {
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
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn spawn(&mut self, value: T) -> Option<Handle> {
        for (i, s) in self.slots.iter_mut().enumerate() {
            if !s.occupied {
                s.value.write(value);
                s.occupied = true;
                self.len += 1;
                return Some(Handle {
                    index: i as u16,
                    generation: s.generation,
                });
            }
        }
        None
    }
    pub fn get(&self, h: Handle) -> Option<&T> {
        let s = self.slots.get(h.index as usize)?;
        if s.occupied && s.generation == h.generation {
            Some(unsafe { s.value.assume_init_ref() })
        } else {
            None
        }
    }
    pub fn get_mut(&mut self, h: Handle) -> Option<&mut T> {
        let s = self.slots.get_mut(h.index as usize)?;
        if s.occupied && s.generation == h.generation {
            Some(unsafe { s.value.assume_init_mut() })
        } else {
            None
        }
    }
    pub fn despawn(&mut self, h: Handle) -> bool {
        let Some(s) = self.slots.get_mut(h.index as usize) else {
            return false;
        };
        if !s.occupied || s.generation != h.generation {
            return false;
        }
        unsafe { s.value.assume_init_drop() };
        s.occupied = false;
        s.generation = s.generation.wrapping_add(1).max(1);
        self.len -= 1;
        true
    }
    pub fn for_each_mut(&mut self, mut f: impl FnMut(&mut T)) {
        for s in &mut self.slots {
            if s.occupied {
                f(unsafe { s.value.assume_init_mut() })
            }
        }
    }
}
impl<T, const N: usize> Default for Pool<T, N> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T, const N: usize> Drop for Pool<T, N> {
    fn drop(&mut self) {
        for s in &mut self.slots {
            if s.occupied {
                unsafe { s.value.assume_init_drop() }
            }
        }
    }
}

pub struct FixedStep {
    step_ms: u32,
    last: u32,
    acc: u32,
    max_steps: u8,
}
impl FixedStep {
    pub const fn new(step_ms: u32) -> Self {
        Self {
            step_ms,
            last: 0,
            acc: 0,
            max_steps: 4,
        }
    }
    pub fn advance(&mut self, now: u32, mut update: impl FnMut()) {
        let delta = if self.last == 0 {
            0
        } else {
            now.wrapping_sub(self.last)
                .min(self.step_ms * self.max_steps as u32)
        };
        self.last = now;
        self.acc += delta;
        let mut n = 0;
        while self.acc >= self.step_ms && n < self.max_steps {
            update();
            self.acc -= self.step_ms;
            n += 1
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    #[test]
    fn stale_handle_rejected() {
        let mut p: Pool<u8, 1> = Pool::new();
        let h = p.spawn(3).unwrap();
        assert!(p.despawn(h));
        let _ = p.spawn(4).unwrap();
        assert!(p.get(h).is_none())
    }
    #[test]
    fn clipping() {
        let mut b = [0u16; 16];
        let mut c = Canvas::new(&mut b, 4, 4).unwrap();
        c.rect(
            Rect {
                x: -1,
                y: -1,
                w: 2,
                h: 2,
            },
            Color565::WHITE,
        );
        assert_eq!(b[0], 0xffff)
    }
}
