extern crate libc;

use self::libc::c_char;
use std::ffi::{CString,CStr};
use std::mem;
use std::ptr;
use std::hash::{Hash,Hasher,SipHasher};
use super::Name;

#[no_mangle]
pub extern "C" fn human_name_parse(input: *const libc::c_char) -> Option<Box<Name>> {
    let s = unsafe {
        CStr::from_ptr(input).to_string_lossy()
    };
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
    let s = CString::new(name.surname().into_owned()).unwrap();
    s.into_raw()
}

#[no_mangle]
pub extern "C" fn human_name_given_name(name: &Name) -> *const c_char {
    match name.given_name() {
        Some(given_name) => {
            let s = CString::new(given_name).unwrap();
            s.into_raw()
        }
        None => ptr::null()
    }
}

#[no_mangle]
pub extern "C" fn human_name_initials(name: &Name) -> *const c_char {
    let s = CString::new(name.initials()).unwrap();
    s.into_raw()
}

#[no_mangle]
pub extern "C" fn human_name_first_initial(name: &Name) -> *const c_char {
    let s = CString::new(name.first_initial().to_string()).unwrap();
    s.into_raw()
}

#[no_mangle]
pub extern "C" fn human_name_middle_initials(name: &Name) -> *const c_char {
    match name.middle_initials() {
        Some(initials) => {
            let s = CString::new(initials).unwrap();
            s.into_raw()
        }
        None => ptr::null()
    }
}

#[no_mangle]
pub extern "C" fn human_name_goes_by_middle_name(name: &Name) -> bool {
    name.goes_by_middle_name()
}

#[no_mangle]
pub extern "C" fn human_name_middle_names(name: &Name) -> *const c_char {
    match name.middle_name() {
        Some(middle_names) => {
            let s = CString::new(middle_names.into_owned()).unwrap();
            s.into_raw()
        }
        None => ptr::null()
    }
}

#[no_mangle]
pub extern "C" fn human_name_suffix(name: &Name) -> *const c_char {
    match name.suffix() {
        Some(suffix) => {
            let s = CString::new(suffix).unwrap();
            s.into_raw()
        }
        None => ptr::null()
    }
}

#[no_mangle]
pub extern "C" fn human_name_display_short(name: &Name) -> *const c_char {
    let s = CString::new(name.display_short()).unwrap();
    s.into_raw()
}

#[no_mangle]
pub extern "C" fn human_name_display_full(name: &Name) -> *const c_char {
    let s = CString::new(name.display_full()).unwrap();
    s.into_raw()
}
