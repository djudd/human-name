//! A C API for interacting with `Name` objects.

extern crate libc;

use self::libc::c_char;
use super::Name;
use std::ffi::{CStr, CString};
use std::mem;
use std::ptr;

macro_rules! str_to_char_star {
    ($str:expr) => {{
        let s = CString::new($str).unwrap();
        s.into_raw()
    }};
}

macro_rules! option_str_to_char_star {
    ($opt:expr) => {
        match $opt {
            Some(string) => {
                let s = CString::new(string).unwrap();
                s.into_raw()
            }
            None => ptr::null(),
        }
    };
}

#[no_mangle]
pub unsafe extern "C" fn human_name_parse(input: *const libc::c_char) -> Option<Box<Name>> {
    let s = CStr::from_ptr(input).to_string_lossy();
    Name::parse(&*s).map(Box::new)
}

#[no_mangle]
pub unsafe extern "C" fn human_name_free_name(name_ptr: *mut Name) {
    mem::drop(Box::from_raw(name_ptr));
}

#[no_mangle]
pub unsafe extern "C" fn human_name_free_string(str_ptr: *mut c_char) {
    mem::drop(CString::from_raw(str_ptr));
}

#[no_mangle]
pub unsafe extern "C" fn human_name_consistent_with(a: &Name, b: &Name) -> bool {
    a.consistent_with(b)
}

#[no_mangle]
pub unsafe extern "C" fn human_name_hash(name: &Name) -> u64 {
    name.hash
}

#[no_mangle]
pub unsafe extern "C" fn human_name_surname(name: &Name) -> *const c_char {
    str_to_char_star!(name.surname().into_owned())
}

#[no_mangle]
pub unsafe extern "C" fn human_name_given_name(name: &Name) -> *const c_char {
    option_str_to_char_star!(name.given_name())
}

#[no_mangle]
pub unsafe extern "C" fn human_name_initials(name: &Name) -> *const c_char {
    str_to_char_star!(name.initials())
}

#[no_mangle]
pub unsafe extern "C" fn human_name_first_initial(name: &Name) -> *const c_char {
    str_to_char_star!(name.first_initial().to_string())
}

#[no_mangle]
pub unsafe extern "C" fn human_name_middle_initials(name: &Name) -> *const c_char {
    option_str_to_char_star!(name.middle_initials())
}

#[no_mangle]
pub unsafe extern "C" fn human_name_goes_by_middle_name(name: &Name) -> bool {
    name.goes_by_middle_name()
}

#[no_mangle]
pub unsafe extern "C" fn human_name_matches_slug_or_localpart(
    name: &Name,
    input: *const libc::c_char,
) -> bool {
    let s = CStr::from_ptr(input).to_string_lossy();
    name.matches_slug_or_localpart(&*s)
}

#[no_mangle]
pub unsafe extern "C" fn human_name_middle_names(name: &Name) -> *const c_char {
    option_str_to_char_star!(name.middle_name().map(|n| n.into_owned()))
}

#[no_mangle]
pub unsafe extern "C" fn human_name_suffix(name: &Name) -> *const c_char {
    option_str_to_char_star!(name.suffix())
}

#[no_mangle]
pub unsafe extern "C" fn human_name_display_first_last(name: &Name) -> *const c_char {
    str_to_char_star!(name.display_first_last().into_owned())
}

#[no_mangle]
pub unsafe extern "C" fn human_name_display_initial_surname(name: &Name) -> *const c_char {
    str_to_char_star!(name.display_initial_surname().into_owned())
}

#[no_mangle]
pub unsafe extern "C" fn human_name_display_full(name: &Name) -> *const c_char {
    str_to_char_star!(name.display_full())
}

#[no_mangle]
pub unsafe extern "C" fn human_name_byte_len(name: &Name) -> u32 {
    name.byte_len() as u32
}
