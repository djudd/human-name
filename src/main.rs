#![feature(test)]

extern crate human_name;
extern crate test;

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

#[cfg(test)]
mod bench {
    use std::io::prelude::*;
    use std::io::BufReader;
    use std::fs::File;
    use test::Bencher;
    use test::black_box;
    use human_name::Name;

    #[bench]
    fn bench_parsing_first_last(b: &mut Bencher) {
        b.iter(|| {
            let parsed = Name::parse("Juan Garcia");
            black_box(parsed.is_none())
        })
    }

    #[bench]
    fn bench_parsing_sort_order(b: &mut Bencher) {
        b.iter(|| {
            let parsed = Name::parse("Garcia, J.Q.");
            black_box(parsed.is_none())
        })
    }

    #[bench]
    fn bench_parsing_unparseable(b: &mut Bencher) {
        b.iter(|| {
            let parsed = Name::parse("foo@bar.com");
            black_box(parsed.is_none())
        })
    }

    #[bench]
    fn bench_parsing_complex(b: &mut Bencher) {
        b.iter(|| {
            let parsed = Name::parse("鈴木 Velasquez y Garcia, Dr. Juan Q. 'Don Juan' Xavier III");
            black_box(parsed.is_none())
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
                let parsed = Name::parse(&name);
                if parsed.is_none() {
                    invalid += 1;
                } else {
                    valid += 1;
                }
            }

            black_box(valid);
            black_box(invalid);
        })
    }
}
