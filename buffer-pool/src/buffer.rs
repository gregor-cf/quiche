// Copyright (C) 2025, Cloudflare, Inc.
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are
// met:
//
//     * Redistributions of source code must retain the above copyright notice,
//       this list of conditions and the following disclaimer.
//
//     * Redistributions in binary form must reproduce the above copyright
//       notice, this list of conditions and the following disclaimer in the
//       documentation and/or other materials provided with the distribution.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS
// IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO,
// THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
// PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR
// CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
// EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
// PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
// PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
// LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
// NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
// SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use std::mem::MaybeUninit;
use std::ops::Deref;
use std::ops::DerefMut;

use crate::Reuse;

/// A convinience wrapper around Vec that allows to "consume" data from the
/// front *without* shifting.
///
/// This is not unlike `VecDeque` but more ergonomic
/// for the operations we require. Conceptually `VecDeque` is two slices, and
/// this is one slice. Also there is no `set_len` for `VecDeque`, so it has to
/// be converted to `Vec` and then back again.
#[derive(Default, Debug)]
pub struct ConsumeBuffer {
    inner: Vec<u8>,
    head: usize,
}

impl Deref for ConsumeBuffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.inner[self.head..]
    }
}

impl DerefMut for ConsumeBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner[self.head..]
    }
}

impl Reuse for ConsumeBuffer {
    fn reuse(&mut self, val: usize) -> bool {
        self.inner.clear();
        self.inner.shrink_to(val);
        self.head = 0;
        self.inner.capacity() > 0
    }
}

impl ConsumeBuffer {
    pub fn from_vec(inner: Vec<u8>) -> Self {
        ConsumeBuffer { inner, head: 0 }
    }

    pub fn into_vec(self) -> Vec<u8> {
        let mut inner = self.inner;
        inner.drain(0..self.head);
        inner
    }

    pub fn pop_front(&mut self, count: usize) {
        assert!(self.head + count <= self.inner.len());
        self.head += count;
    }

    pub fn expand(&mut self, count: usize) {
        self.inner.reserve_exact(count);
        // SAFETY: u8 is always initialized and we reserved the capacity.
        unsafe { self.inner.set_len(count) };
    }

    pub fn truncate(&mut self, count: usize) {
        self.inner.truncate(self.head + count);
    }

    pub fn add_prefix(&mut self, prefix: &[u8]) -> bool {
        if self.head < prefix.len() {
            return false;
        }

        self.head -= prefix.len();
        self.inner[self.head..self.head + prefix.len()].copy_from_slice(prefix);

        true
    }
}

impl<'a> Extend<&'a u8> for ConsumeBuffer {
    fn extend<T: IntoIterator<Item = &'a u8>>(&mut self, iter: T) {
        self.inner.extend(iter)
    }
}

#[derive(Default, Debug)]
pub struct BufWithPrefix {
    inner: Vec<u8>,
    head: usize,
}

impl Deref for BufWithPrefix {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.inner[self.head..]
    }
}

impl DerefMut for BufWithPrefix {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner[self.head..]
    }
}

impl Reuse for BufWithPrefix {
    fn reuse(&mut self, val: usize) -> bool {
        self.inner.clear();
        self.inner.shrink_to(val);
        self.head = 0;
        self.inner.capacity() > 0
    }
}

impl BufWithPrefix {
    // pub fn from_vec(inner: Vec<u8>) -> Self {
    //    BufWithPrefix { inner, head: 0 }
    //}
    // pub fn gm_empty() -> Self {
    //     Self::default()
    // }

    // pub fn gm_dgram() -> Self {
    //     Self::with_capacity_and_headroom(1500, 9)
    // }

    // pub fn gm_max_buf() -> Self {
    //     Self::with_capacity_and_headroom(65536, 20)
    // }

    pub fn from_slice(data: &[u8]) -> Self {
        BufWithPrefix {
            inner: data.into(),
            head: 0,
        }
    }

    // pub fn dgram_from_slice(data: &[u8]) -> Self {
    //     let mut res = Self::gm_dgram();
    //     res.extend(data);
    //     res
    // }

    pub fn with_capacity(capacity: usize) -> Self {
        BufWithPrefix {
            inner: Vec::with_capacity(capacity),
            head: 0,
        }
    }

    pub fn with_capacity_and_headroom(capacity: usize, headroom: usize) -> Self {
        assert!(capacity >= headroom);
        let mut v = Vec::with_capacity(capacity);
        v.resize(headroom, 0);
        BufWithPrefix {
            inner: v,
            head: headroom,
        }
    }

    pub fn from_vec_with_headroom(v: Vec<u8>, headroom: usize) -> Self {
        assert!(headroom <= v.len());
        BufWithPrefix {
            inner: v,
            head: headroom,
        }
    }

    pub fn into_vec(self) -> Vec<u8> {
        let mut inner = self.inner;
        inner.drain(0..self.head);
        inner
    }

    pub fn truncate(&mut self, count: usize) {
        self.inner.truncate(self.head + count);
    }

    pub fn pop_front(&mut self, count: usize) {
        assert!(self.head + count <= self.inner.len());
        self.head += count;
    }

    pub fn add_prefix(&mut self, prefix: &[u8]) -> bool {
        if self.head < prefix.len() {
            return false;
        }

        self.head -= prefix.len();
        self.inner[self.head..self.head + prefix.len()].copy_from_slice(prefix);

        true
    }

    pub fn clear(&mut self) {
        self.head = 0;
        self.inner.clear();
    }

    pub fn remaining(&self) -> usize {
        self.inner.capacity() - self.inner.len()
    }

    pub fn spare_capacity_mut(&mut self) -> &mut [MaybeUninit<u8>] {
        self.inner.spare_capacity_mut()
    }

    pub unsafe fn assume_init_and_filled(&mut self, additional: usize) {
        unsafe { self.inner.set_len(self.inner.len() + additional) };
    }
}

impl<'a> Extend<&'a u8> for BufWithPrefix {
    fn extend<T: IntoIterator<Item = &'a u8>>(&mut self, iter: T) {
        self.inner.extend(iter)
    }
}
