// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![cfg(feature = "api")]
#![cfg(disabled)]

use crate::ffi;
use crate::Jemalloc;
use core::alloc::{Alloc, AllocErr, CannotReallocInPlace, Excess};
use core::ptr::NonNull;
use core::{
    alloc::{GlobalAlloc, Layout},
    cmp,
    hint::assert_unchecked,
};

use crate::ffi::{MALLOCX_ALIGN, MALLOCX_ZERO};
use libc::{c_void, uintptr_t};

unsafe impl Alloc for Jemalloc {
    #[inline]
    unsafe fn alloc(&mut self, layout: Layout) -> Result<NonNull<u8>, AllocErr> {
        NonNull::new(GlobalAlloc::alloc(self, layout)).ok_or(AllocErr)
    }

    #[inline]
    unsafe fn alloc_zeroed(&mut self, layout: Layout) -> Result<NonNull<u8>, AllocErr> {
        NonNull::new(GlobalAlloc::alloc_zeroed(self, layout)).ok_or(AllocErr)
    }

    #[inline]
    unsafe fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout) {
        GlobalAlloc::dealloc(self, ptr.as_ptr(), layout)
    }

    #[inline]
    unsafe fn realloc(
        &mut self,
        ptr: NonNull<u8>,
        layout: Layout,
        new_size: usize,
    ) -> Result<NonNull<u8>, AllocErr> {
        NonNull::new(GlobalAlloc::realloc(self, ptr.as_ptr(), layout, new_size)).ok_or(AllocErr)
    }

    #[inline]
    unsafe fn alloc_excess(&mut self, layout: Layout) -> Result<Excess, AllocErr> {
        let flags = layout_to_flags(layout.align(), layout.size());
        let ptr = ffi::mallocx(layout.size(), flags);
        if let Some(nonnull) = NonNull::new(ptr as *mut u8) {
            let excess = ffi::nallocx(layout.size(), flags);
            Ok(Excess(nonnull, excess))
        } else {
            Err(AllocErr)
        }
    }

    #[inline]
    unsafe fn realloc_excess(
        &mut self,
        ptr: NonNull<u8>,
        layout: Layout,
        new_size: usize,
    ) -> Result<Excess, AllocErr> {
        let flags = layout_to_flags(layout.align(), new_size);
        let ptr = ffi::rallocx(ptr.cast().as_ptr(), new_size, flags);
        if let Some(nonnull) = NonNull::new(ptr as *mut u8) {
            let excess = ffi::nallocx(new_size, flags);
            Ok(Excess(nonnull, excess))
        } else {
            Err(AllocErr)
        }
    }

    #[inline]
    fn usable_size(&self, layout: &Layout) -> (usize, usize) {
        let flags = layout_to_flags(layout.align(), layout.size());
        unsafe {
            let max = ffi::nallocx(layout.size(), flags);
            (layout.size(), max)
        }
    }

    #[inline]
    unsafe fn grow_in_place(
        &mut self,
        ptr: NonNull<u8>,
        layout: Layout,
        new_size: usize,
    ) -> Result<(), CannotReallocInPlace> {
        let flags = layout_to_flags(layout.align(), new_size);
        let usable_size = ffi::xallocx(ptr.cast().as_ptr(), new_size, 0, flags);
        if usable_size >= new_size {
            Ok(())
        } else {
            // `xallocx` returns a size smaller than the requested one to
            // indicate that the allocation could not be grown in place
            //
            // the old allocation remains unaltered
            Err(CannotReallocInPlace)
        }
    }

    #[inline]
    unsafe fn shrink_in_place(
        &mut self,
        ptr: NonNull<u8>,
        layout: Layout,
        new_size: usize,
    ) -> Result<(), CannotReallocInPlace> {
        if new_size == layout.size() {
            return Ok(());
        }
        let flags = layout_to_flags(layout.align(), new_size);
        let usable_size = ffi::xallocx(ptr.cast().as_ptr(), new_size, 0, flags);

        if usable_size < layout.size() {
            // If `usable_size` is smaller than the original size, the
            // size-class of the allocation was shrunk to the size-class of
            // `new_size`, and it is safe to deallocate the allocation with
            // `new_size`:
            Ok(())
        } else if usable_size == ffi::nallocx(new_size, flags) {
            // If the allocation was not shrunk and the size class of `new_size`
            // is the same as the size-class of `layout.size()`, then the
            // allocation can be properly deallocated using `new_size` (and also
            // using `layout.size()` because the allocation did not change)

            // note: when the allocation is not shrunk, `xallocx` returns the
            // usable size of the original allocation, which in this case matches
            // that of the requested allocation:
            debug_assert_eq!(
                ffi::nallocx(new_size, flags),
                ffi::nallocx(layout.size(), flags)
            );
            Ok(())
        } else {
            // If the allocation was not shrunk, but the size-class of
            // `new_size` is not the same as that of the original allocation,
            // then shrinking the allocation failed:
            Err(CannotReallocInPlace)
        }
    }
}
