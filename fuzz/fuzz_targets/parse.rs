#![no_main]
use libfuzzer_sys::fuzz_target;
use human_name::Name;

fuzz_target!(|data: &str| {
    Name::parse(data);
});
