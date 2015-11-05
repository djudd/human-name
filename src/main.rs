#![feature(test)]

extern crate human_name;
extern crate test;
extern crate rustc_serialize;

use std::env;
use std::process;
use std::io;
use std::io::BufReader;
use std::io::prelude::*;
use rustc_serialize::json;

const USAGE: &'static str = "
Usage:
    human_name -
    human_name <name>

If `-` is the argument, human_name will expect input on stdin.
Otherwise, it will try to parse the arguments as a name.
";

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        writeln!(&mut std::io::stderr(), "{}", USAGE).ok().unwrap();
        process::exit(64);
    }

    if args.len() == 2 && args[1] == "-" {
        let reader = BufReader::new(io::stdin());
        for line in reader.lines() {
            match line.ok() {
                Some(input) => {
                    let parsed = human_name::Name::parse(&input);
                    let output = match parsed {
                        Some(name) => { json::encode(&name).unwrap() },
                        None => { "".to_string() }
                    };

                    if !writeln!(&mut io::stdout(), "{}", output).is_ok() {
                        break
                    }
                },
                None => { break }
            }
        }
    } else {
        let parsed = human_name::Name::parse(&args[1..].join(" "));
        if parsed.is_none() {
            process::exit(1);
        } else {
            println!("{}", json::encode(&parsed.unwrap()).unwrap());
        }
    }
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
