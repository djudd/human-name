#![feature(test)]

extern crate human_name;
extern crate test;

use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;
use test::Bencher;

fn debug(names: Vec<&str>) {
    for raw in names.iter() {
        let maybe_name = human_name::Name::parse(raw);
        if maybe_name.is_none() {
            println!("!! {}", raw);
            continue;
        }

        let name = maybe_name.unwrap();
        let first: &str = match name.given_name {
            Some(ref given) => { given }
            None => { "[?]" }
        };
        let middle: &str = match name.middle_initials {
            Some(ref initials) => { initials }
            None => { "[?]" }
        };

        println!("{} => {} [{}, {}, {}]", raw, name.display(), name.surname, first, middle);
    }
}

fn main() {

}

#[bench]
fn bench_parsing_first_last(b: &mut Bencher) {
    b.iter(|| {
        let parsed = human_name::Name::parse("Juan Garcia");
        test::black_box(parsed.is_none())
    })
}

#[bench]
fn bench_parsing_sort_order(b: &mut Bencher) {
    b.iter(|| {
        let parsed = human_name::Name::parse("Garcia, J.Q.");
        test::black_box(parsed.is_none())
    })
}

#[bench]
fn bench_parsing_unparseable(b: &mut Bencher) {
    b.iter(|| {
        let parsed = human_name::Name::parse("foo@bar.com");
        test::black_box(parsed.is_none())
    })
}

#[bench]
fn bench_parsing_complex(b: &mut Bencher) {
    b.iter(|| {
        let parsed = human_name::Name::parse("鈴木 Velasquez y Garcia, Dr. Juan Q. 'Don Juan' Xavier III");
        test::black_box(parsed.is_none())
    })
}

#[bench]
fn bench_parsing_many(b: &mut Bencher) {
    let f = File::open("tests/benchmark-names.txt").ok().unwrap();
    let reader = BufReader::new(f);
    let names: Vec<String> = reader.lines().map( |l| l.ok().unwrap() ).collect();

    b.iter(move || {
        let mut valid = 0;
        let mut invalid = 0;

        for name in names.iter() {
            let parsed = human_name::Name::parse(&name);
            if parsed.is_none() {
                invalid += 1;
            } else {
                valid += 1;
            }
        }

        test::black_box(valid);
        test::black_box(invalid);
    })
}
