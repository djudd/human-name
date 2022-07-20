#[macro_use]
extern crate criterion;

mod bench {
    use human_name::Name;
    use std::collections::HashSet;
    use std::fs::File;
    use std::io::prelude::*;
    use std::io::BufReader;

    use criterion::{black_box, criterion_group, Criterion};

    fn parsing_first_last(c: &mut Criterion) {
        c.bench_function("first last", |b| {
            b.iter(|| {
                let parsed = Name::parse("Juan Garcia");
                black_box(parsed.is_none())
            })
        });
    }

    fn parsing_sort_order(c: &mut Criterion) {
        c.bench_function("last, first", |b| {
            b.iter(|| {
                let parsed = Name::parse("Garcia, J.Q.");
                black_box(parsed.is_none())
            })
        });
    }

    fn parsing_needs_namecase(c: &mut Criterion) {
        c.bench_function("all-caps", |b| {
            b.iter(|| {
                let parsed = Name::parse("JAIME GARCIA");
                black_box(parsed.is_none())
            })
        });
    }

    fn parsing_unparseable(c: &mut Criterion) {
        c.bench_function("unparseable", |b| {
            b.iter(|| {
                let parsed = Name::parse("foo@bar.com");
                black_box(parsed.is_none())
            })
        });
    }

    fn parsing_complex(c: &mut Criterion) {
        let name = "鈴木 Velasquez y Garcia, Dr. Juan Q. 'Don Juan' Xavier III";
        c.bench_function("complex", |b| {
            b.iter(|| {
                let parsed = Name::parse(name);
                black_box(parsed.is_none())
            })
        });
    }

    criterion_group!(
        e2e_parsing,
        parsing_first_last,
        parsing_sort_order,
        parsing_needs_namecase,
        parsing_unparseable,
        parsing_complex
    );

    fn equality_equal(c: &mut Criterion) {
        let x = Name::parse("Jane Doe");
        let y = Name::parse("Jane H. Doe");

        c.bench_function("equal", |b| b.iter(|| black_box(x == y)));
    }

    fn equality_not_equal(c: &mut Criterion) {
        let x = Name::parse("Jane Doe");
        let y = Name::parse("Foo Bar");

        c.bench_function("unequal", |b| b.iter(|| black_box(x == y)));
    }

    fn equality_almost_equal(c: &mut Criterion) {
        let x = Name::parse("Jane Doe");
        let y = Name::parse("John Doe");

        c.bench_function("almost equal", |b| b.iter(|| black_box(x == y)));
    }

    criterion_group!(
        e2e_equality,
        equality_equal,
        equality_not_equal,
        equality_almost_equal
    );

    fn parsing_many(c: &mut Criterion) {
        let f = File::open("tests/benchmark-names.txt").ok().unwrap();
        let reader = BufReader::new(f);
        let names: Vec<String> = reader.lines().map(|l| l.ok().unwrap()).collect();

        c.bench_function(&format!("parse {} names", names.len()), |b| {
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
        });
    }

    fn equality_many(c: &mut Criterion) {
        let f = File::open("tests/benchmark-names.txt").ok().unwrap();
        let reader = BufReader::new(f);
        let names: Vec<Name> = reader
            .lines()
            .filter_map(|l| Name::parse(&l.ok().unwrap()))
            .collect();

        let mut deduped = HashSet::with_capacity(names.len() / 4);
        c.bench_function(&format!("dedup {} names", names.len()), |b| {
            b.iter(|| {
                deduped.extend(names.iter());
                black_box(deduped.len())
            })
        });
    }

    criterion_group!(realistic, parsing_many, equality_many);
}

criterion_main!(
    realistic,
    e2e_equality,
    e2e_parsing,
    case,
    decomposition,
    parse,
    nick,
    name_part,
    Name
);
