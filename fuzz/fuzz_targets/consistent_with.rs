#![no_main]
use libfuzzer_sys::fuzz_target;
use human_name::Name;

fuzz_target!(|data: [String; 2]| {
    let [a, b] = data;
    if let Some(a) = Name::parse(&a) {
        if let Some(b) = Name::parse(&b) {
            a.consistent_with(&b);
        }
    }
});
