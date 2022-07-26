#[macro_use]
extern crate criterion;

use benchable::Location;
use human_name::{benchable, Name};
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
        b.iter(|| {
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

fn initialize_struct_initial_surname(c: &mut Criterion) {
    let name = "J. Doe";
    let parsed = benchable::parse(&*name).unwrap();

    c.bench_function("initial surname", |b| {
        b.iter(|| {
            black_box(
                benchable::initialize_name_struct(&parsed, name.len())
                    .unwrap()
                    .byte_len(),
            )
        })
    });
}

fn initialize_struct_first_last(c: &mut Criterion) {
    let name = "John Doe";
    let parsed = benchable::parse(&*name).unwrap();
    c.bench_function("first last", |b| {
        b.iter(|| {
            black_box(
                benchable::initialize_name_struct(&parsed, name.len())
                    .unwrap()
                    .byte_len(),
            )
        })
    });
}

fn initialize_struct_complex(c: &mut Criterion) {
    let name = "John Allen Q.R. de la MacDonald Jr.";
    let parsed = benchable::parse(&*name).unwrap();
    c.bench_function("complex", |b| {
        b.iter(|| {
            black_box(
                benchable::initialize_name_struct(&parsed, name.len())
                    .unwrap()
                    .byte_len(),
            )
        })
    });
}

criterion_group!(
    name_struct,
    initialize_struct_initial_surname,
    initialize_struct_first_last,
    initialize_struct_complex
);

fn all_from_text(c: &mut Criterion) {
    c.bench_function("first last", |b| {
        b.iter(|| {
            black_box(benchable::name_part_all_from_text("John Doe", true, Location::Start).count())
        })
    });
    c.bench_function("initial surname", |b| {
        b.iter(|| {
            black_box(benchable::name_part_all_from_text("J. Doe", true, Location::Start).count())
        })
    });
    c.bench_function("nonascii", |b| {
        b.iter(|| {
            black_box(benchable::name_part_all_from_text("이용희", false, Location::Start).count())
        })
    });
    c.bench_function("all caps", |b| {
        b.iter(|| {
            black_box(
                benchable::name_part_all_from_text("JOHN DOE", false, Location::Start).count(),
            )
        })
    });
}

criterion_group!(name_part, all_from_text);

fn mixed_case(c: &mut Criterion) {
    c.bench_function("not mixed", |b| {
        b.iter(|| black_box(benchable::is_mixed_case("JOHN MACDONALD")))
    });

    c.bench_function("mixed", |b| {
        b.iter(|| black_box(benchable::is_mixed_case("J. MacDonald")))
    });
}

fn capitalize(c: &mut Criterion) {
    c.bench_function("uppercase ascii", |b| {
        b.iter(|| black_box(benchable::capitalize_word("JONATHAN", true)))
    });

    c.bench_function("complex", |b| {
        b.iter(|| black_box(benchable::capitalize_word("föö-bar", false)))
    });
}

criterion_group!(case, capitalize, mixed_case);

fn strip_nicknames(c: &mut Criterion) {
    c.bench_function("no nickname", |b| {
        b.iter(|| {
            black_box(benchable::strip_nickname("James T. Kirk").len());
        })
    });

    c.bench_function("nickname", |b| {
        b.iter(|| {
            black_box(benchable::strip_nickname("James T. 'Jimmy' Kirk").len());
        })
    });
}

fn have_matching_variants(c: &mut Criterion) {
    c.bench_function("no match", |b| {
        b.iter(|| {
            black_box(benchable::have_matching_variants("David", "Daniel"));
        })
    });

    c.bench_function("match", |b| {
        b.iter(|| {
            black_box(benchable::have_matching_variants("David", "Dave"));
        })
    });
}

criterion_group!(nick, strip_nicknames, have_matching_variants);

fn normalization(c: &mut Criterion) {
    c.bench_function("ascii", |b| {
        b.iter(|| black_box(benchable::normalize_nfkd_whitespace("James 'J' S. Brown MD").len()))
    });

    c.bench_function("nfkd non-ascii", |b| {
        b.iter(|| black_box(benchable::normalize_nfkd_whitespace("James «J» S. Brown MD").len()))
    });

    c.bench_function("non-nfkd non-ascii", |b| {
        b.iter(|| black_box(benchable::normalize_nfkd_whitespace("James 'J' S. Bröwn MD").len()))
    });
}

criterion_group!(decomposition, normalization);

criterion_main!(
    realistic,
    e2e_equality,
    e2e_parsing,
    case,
    decomposition,
    nick,
    name_part,
    name_struct
);
