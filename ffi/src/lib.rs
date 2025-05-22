//! C-compatible foreign function interface for guillotiere, that can be easily fed to cbindgen.

#![no_std]
extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use guillotiere::*;
use core::mem::transmute;

use guillotiere::AtlasAllocator as guillotiere_atlas_allocator_t;
use guillotiere::ChangeList as guillotiere_change_list_t;
use guillotiere::SimpleAtlasAllocator as guillotiere_simple_atlas_allocator_t;

#[repr(C)]
#[no_mangle]
pub struct guillotiere_size_t {
    pub width: i32,
    pub height: i32,
}

#[repr(C)]
#[no_mangle]
pub struct guillotiere_rectangle_t {
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
}

#[repr(C)]
#[no_mangle]
pub struct guillotiere_change_t {
    pub old_alloc: guillotiere_allocation_t,
    pub new_alloc: guillotiere_allocation_t,
}

#[repr(C)]
#[no_mangle]
pub struct guillotiere_changes_t {
    pub changes: *const guillotiere_change_t,
    pub count: usize,
}

#[repr(C)]
#[no_mangle]
pub struct guillotiere_failures_t {
    pub failures: *const guillotiere_allocation_t,
    pub count: usize,
}

#[repr(C)]
#[no_mangle]
pub struct guillotiere_alloc_id_t {
    id: u32,
}

#[repr(C)]
#[no_mangle]
pub struct guillotiere_allocation_t {
    pub id: guillotiere_alloc_id_t,
    pub rectangle: guillotiere_rectangle_t,
}

#[repr(C)]
#[no_mangle]
pub struct guillotiere_allocator_options_t {
    pub width_alignment: i32,
    pub height_alignment: i32,
    pub small_size_threshold: i32,
    pub large_size_threshold: i32,
}

fn from_ffi_options(options: &guillotiere_allocator_options_t) -> AllocatorOptions {
    AllocatorOptions {
        alignment: size2(options.width_alignment, options.height_alignment),
        small_size_threshold: options.small_size_threshold,
        large_size_threshold: options.large_size_threshold,
    }
}

#[no_mangle]
pub unsafe extern "C" fn guillotiere_atlas_allocator_new(
    size: guillotiere_size_t,
) -> *mut guillotiere_atlas_allocator_t {
    Box::into_raw(Box::new(AtlasAllocator::new(transmute(size))))
}

#[no_mangle]
pub unsafe extern "C" fn guillotiere_atlas_allocator_with_options(
    size: guillotiere_size_t,
    options: &guillotiere_allocator_options_t,
) -> *mut guillotiere_atlas_allocator_t {
    let options = from_ffi_options(options);
    Box::into_raw(Box::new(AtlasAllocator::with_options(
        transmute(size),
        &options,
    )))
}

#[no_mangle]
pub unsafe extern "C" fn guillotiere_atlas_allocator_delete(
    atlas: *mut guillotiere_atlas_allocator_t,
) {
    let _ = Box::from_raw(atlas);
}

#[no_mangle]
pub unsafe extern "C" fn guillotiere_atlas_allocator_clear(
    atlas: &mut guillotiere_atlas_allocator_t,
) {
    atlas.clear();
}

#[no_mangle]
pub unsafe extern "C" fn guillotiere_atlas_allocator_reset(
    atlas: &mut guillotiere_atlas_allocator_t,
    size: guillotiere_size_t,
    options: &guillotiere_allocator_options_t,
) {
    let options = from_ffi_options(options);
    atlas.reset(transmute(size), &options);
}

#[no_mangle]
pub unsafe extern "C" fn guillotiere_atlas_allocator_size(
    atlas: &guillotiere_atlas_allocator_t,
) -> guillotiere_size_t {
    transmute(atlas.size())
}

#[no_mangle]
pub unsafe extern "C" fn guillotiere_atlas_allocator_is_empty(
    atlas: &mut guillotiere_atlas_allocator_t,
) -> bool {
    atlas.is_empty()
}

#[no_mangle]
pub unsafe extern "C" fn guillotiere_atlas_allocator_allocate(
    atlas: &mut guillotiere_atlas_allocator_t,
    size: guillotiere_size_t,
    result: &mut guillotiere_allocation_t,
) -> bool {
    if let Some(alloc) = atlas.allocate(transmute(size)) {
        *result = transmute(alloc);
        return true;
    }

    false
}

#[no_mangle]
pub unsafe extern "C" fn guillotiere_atlas_allocator_deallocate(
    atlas: &mut guillotiere_atlas_allocator_t,
    id: guillotiere_alloc_id_t,
) {
    atlas.deallocate(transmute(id));
}

#[no_mangle]
pub unsafe extern "C" fn guillotiere_atlas_allocator_grow(
    atlas: &mut guillotiere_atlas_allocator_t,
    new_size: guillotiere_size_t,
) {
    atlas.grow(transmute(new_size));
}

#[no_mangle]
pub unsafe extern "C" fn guillotiere_atlas_allocator_rearrange(
    atlas: &mut guillotiere_atlas_allocator_t,
    change_list: &mut guillotiere_change_list_t,
) {
    core::mem::swap(change_list, &mut atlas.rearrange());
}

#[no_mangle]
pub unsafe extern "C" fn guillotiere_atlas_allocator_resize_and_rearrange(
    atlas: &mut guillotiere_atlas_allocator_t,
    new_size: guillotiere_size_t,
    change_list: &mut guillotiere_change_list_t,
) {
    core::mem::swap(
        change_list,
        &mut atlas.resize_and_rearrange(transmute(new_size)),
    );
}

#[no_mangle]
pub unsafe extern "C" fn guillotiere_change_list_new() -> *mut guillotiere_change_list_t {
    Box::into_raw(Box::new(ChangeList {
        changes: Vec::new(),
        failures: Vec::new(),
    }))
}

#[no_mangle]
pub unsafe extern "C" fn guillotiere_change_list_delete(
    change_list: *mut guillotiere_change_list_t,
) {
    let _ = Box::from_raw(change_list);
}

#[no_mangle]
pub unsafe extern "C" fn guillotiere_change_list_changes(
    change_list: &guillotiere_change_list_t,
) -> guillotiere_changes_t {
    guillotiere_changes_t {
        changes: transmute(change_list.changes.as_ptr()),
        count: change_list.changes.len(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn guillotiere_change_list_failures(
    change_list: &guillotiere_change_list_t,
) -> guillotiere_failures_t {
    guillotiere_failures_t {
        failures: transmute(change_list.failures.as_ptr()),
        count: change_list.failures.len(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn guillotiere_simple_atlas_allocator_new(
    size: guillotiere_size_t,
) -> *mut guillotiere_simple_atlas_allocator_t {
    Box::into_raw(Box::new(SimpleAtlasAllocator::new(transmute(size))))
}

#[no_mangle]
pub unsafe extern "C" fn guillotiere_simple_atlas_allocator_with_options(
    size: guillotiere_size_t,
    options: &guillotiere_allocator_options_t,
) -> *mut guillotiere_simple_atlas_allocator_t {
    let options = from_ffi_options(options);
    Box::into_raw(Box::new(SimpleAtlasAllocator::with_options(
        transmute(size),
        &options,
    )))
}

#[no_mangle]
pub unsafe extern "C" fn guillotiere_simple_atlas_allocator_delete(
    atlas: *mut guillotiere_simple_atlas_allocator_t,
) {
    let _ = Box::from_raw(atlas);
}

#[no_mangle]
pub unsafe extern "C" fn guillotiere_simple_atlas_allocator_clear(
    atlas: &mut guillotiere_simple_atlas_allocator_t,
) {
    atlas.clear();
}

#[no_mangle]
pub unsafe extern "C" fn guillotiere_simple_atlas_allocator_reset(
    atlas: &mut guillotiere_simple_atlas_allocator_t,
    size: guillotiere_size_t,
    options: &guillotiere_allocator_options_t,
) {
    let options = from_ffi_options(options);
    atlas.reset(transmute(size), &options);
}

#[no_mangle]
pub unsafe extern "C" fn guillotiere_simple_atlas_allocator_is_empty(
    atlas: &mut guillotiere_simple_atlas_allocator_t,
) -> bool {
    atlas.is_empty()
}

#[no_mangle]
pub unsafe extern "C" fn guillotiere_simple_atlas_allocator_size(
    atlas: &guillotiere_simple_atlas_allocator_t,
) -> guillotiere_size_t {
    transmute(atlas.size())
}

#[no_mangle]
pub unsafe extern "C" fn guillotiere_simple_atlas_allocator_allocate(
    atlas: &mut guillotiere_simple_atlas_allocator_t,
    size: guillotiere_size_t,
    result: &mut guillotiere_rectangle_t,
) -> bool {
    if let Some(alloc) = atlas.allocate(transmute(size)) {
        *result = transmute(alloc);
        return true;
    }

    false
}

#[no_mangle]
pub unsafe extern "C" fn guillotiere_simple_atlas_allocator_grow(
    atlas: &mut guillotiere_simple_atlas_allocator_t,
    new_size: guillotiere_size_t,
) {
    atlas.grow(transmute(new_size));
}

#[no_mangle]
pub unsafe extern "C" fn guillotiere_simple_atlas_allocator_init_from_allocator(
    atlas: &mut guillotiere_simple_atlas_allocator_t,
    src: &guillotiere_atlas_allocator_t,
) {
    atlas.init_from_allocator(src);
}

#[no_mangle]
pub unsafe extern "C" fn guillotiere_allocator_options_default(
    options: &mut guillotiere_allocator_options_t,
) {
    *options = guillotiere_allocator_options_t {
        width_alignment: DEFAULT_OPTIONS.alignment.width,
        height_alignment: DEFAULT_OPTIONS.alignment.height,
        small_size_threshold: DEFAULT_OPTIONS.small_size_threshold,
        large_size_threshold: DEFAULT_OPTIONS.large_size_threshold,
    };
}

// TODO:
// for_each_free_rectangle
// for_each_allocated_rectangle
// svg dump
