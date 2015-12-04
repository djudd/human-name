extern crate libc;

use self::libc::c_char;
use std::ffi::{CString, CStr};
use std::mem;
use std::ptr;
use std::hash::{Hash, Hasher, SipHasher};
use super::Name;

macro_rules! str_to_char_star {
    ($str:expr) => { {
        let s = CString::new($str).unwrap();
        s.into_raw()
    } }
}

macro_rules! option_str_to_char_star {
    ($opt:expr) => {
        match $opt {
            Some(string) => {
                let s = CString::new(string).unwrap();
                s.into_raw()
            }
            None => ptr::null()
        }
    }
}

#[no_mangle]
pub extern "C" fn human_name_parse(input: *const libc::c_char) -> Option<Box<Name>> {
    let s = unsafe { CStr::from_ptr(input).to_string_lossy() };
    Name::parse(&*s).map(|n| Box::new(n))
}

#[no_mangle]
pub extern "C" fn human_name_free_name(name_ptr: *mut Name) {
    unsafe {
        mem::drop(Box::from_raw(name_ptr));
    }
}

#[no_mangle]
pub extern "C" fn human_name_free_string(str_ptr: *mut c_char) {
    unsafe {
        mem::drop(CString::from_raw(str_ptr));
    }
}

#[no_mangle]
pub extern "C" fn human_name_consistent_with(a: &Name, b: &Name) -> bool {
    a.consistent_with(b)
}

#[no_mangle]
pub extern "C" fn human_name_hash(name: &Name) -> u64 {
    let mut s = SipHasher::new();
    name.hash(&mut s);
    s.finish()
}

#[no_mangle]
pub extern "C" fn human_name_surname(name: &Name) -> *const c_char {
    str_to_char_star!(name.surname().into_owned())
}

#[no_mangle]
pub extern "C" fn human_name_given_name(name: &Name) -> *const c_char {
    option_str_to_char_star!(name.given_name())
}

#[no_mangle]
pub extern "C" fn human_name_initials(name: &Name) -> *const c_char {
    str_to_char_star!(name.initials())
}

#[no_mangle]
pub extern "C" fn human_name_first_initial(name: &Name) -> *const c_char {
    str_to_char_star!(name.first_initial().to_string())
}

#[no_mangle]
pub extern "C" fn human_name_middle_initials(name: &Name) -> *const c_char {
    option_str_to_char_star!(name.middle_initials())
}

#[no_mangle]
pub extern "C" fn human_name_goes_by_middle_name(name: &Name) -> bool {
    name.goes_by_middle_name()
}

#[no_mangle]
pub extern "C" fn human_name_middle_names(name: &Name) -> *const c_char {
    option_str_to_char_star!(name.middle_name().map(|n| n.into_owned()))
}

#[no_mangle]
pub extern "C" fn human_name_suffix(name: &Name) -> *const c_char {
    option_str_to_char_star!(name.suffix())
}

#[no_mangle]
pub extern "C" fn human_name_display_short(name: &Name) -> *const c_char {
    str_to_char_star!(name.display_short())
}

#[no_mangle]
pub extern "C" fn human_name_display_full(name: &Name) -> *const c_char {
    str_to_char_star!(name.display_full())
}
