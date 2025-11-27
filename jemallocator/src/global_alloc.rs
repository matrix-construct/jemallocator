// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use core::{
    alloc::{GlobalAlloc, Layout},
    hint::assert_unchecked,
};

use libc::{c_void, uintptr_t};

use crate::{
    adjust_layout, ffi,
    ffi::{MALLOCX_ALIGN, MALLOCX_ZERO},
    Jemalloc,
};

unsafe impl GlobalAlloc for Jemalloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        #[cfg(feature = "global_hooks")]
        if let Some(hook) = super::HOOK_GLOBAL_ALLOC {
            hook(layout);
        }

        let layout = adjust_layout(layout);
        let flags = MALLOCX_ALIGN(layout.align());
        debug_assert!(
            ffi::nallocx(layout.size(), flags) >= layout.size(),
            "alloc: nallocx() reported failure"
        );

        let ptr = ffi::mallocx(layout.size(), flags);
        debug_assert!(
            (ptr as uintptr_t).is_multiple_of(layout.align()),
            "alloc: alignment mismatch"
        );

        debug_assert!(
            ffi::sallocx(ptr, flags) >= layout.size(),
            "alloc: sallocx() size mismatch"
        );

        ptr as *mut u8
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        #[cfg(feature = "global_hooks")]
        if let Some(hook) = super::HOOK_GLOBAL_ALLOC_ZEROED {
            hook(layout);
        }

        let layout = adjust_layout(layout);
        let flags = MALLOCX_ALIGN(layout.align()) | MALLOCX_ZERO;
        debug_assert!(
            ffi::nallocx(layout.size(), flags) >= layout.size(),
            "alloc_zeroed: nallocx() reported failure"
        );

        let ptr = ffi::mallocx(layout.size(), flags);
        debug_assert!(
            (ptr as uintptr_t).is_multiple_of(layout.align()),
            "alloc: alignment mismatch"
        );

        debug_assert!(
            ffi::sallocx(ptr, flags) >= layout.size(),
            "alloc_zeroed: sallocx() size mismatch"
        );

        ptr as *mut u8
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        #[cfg(feature = "global_hooks")]
        if let Some(hook) = super::HOOK_GLOBAL_REALLOC {
            hook(layout, ptr, new_size);
        }

        let layout = Layout::from_size_align_unchecked(new_size, layout.align());
        let layout = adjust_layout(layout);
        let flags = MALLOCX_ALIGN(layout.align());
        debug_assert!(
            ffi::nallocx(layout.size(), flags) >= layout.size(),
            "realloc: nallocx() reported failure"
        );

        let ptr = ffi::rallocx(ptr as *mut c_void, layout.size(), flags);
        debug_assert!(
            (ptr as uintptr_t).is_multiple_of(layout.align()),
            "alloc: alignment mismatch"
        );

        debug_assert!(
            ffi::sallocx(ptr, flags) >= layout.size(),
            "reelloc: sallocx() size mismatch"
        );

        ptr as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        #[cfg(feature = "global_hooks")]
        if let Some(hook) = super::HOOK_GLOBAL_DEALLOC {
            hook(layout, ptr);
        }

        assert_unchecked(!ptr.is_null());
        let ptr = ptr as *mut c_void;
        let layout = adjust_layout(layout);
        debug_assert!(
            (ptr as uintptr_t).is_multiple_of(layout.align()),
            "dealloc: alignment mismatch"
        );

        let flags = MALLOCX_ALIGN(layout.align());
        debug_assert!(
            ffi::sallocx(ptr, flags) >= layout.size(),
            "dealloc: sallocx() size mismatch"
        );

        ffi::sdallocx(ptr, layout.size(), flags)
    }
}
