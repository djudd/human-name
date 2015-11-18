#![feature(test)]

extern crate human_name;
extern crate test;
extern crate rustc_serialize;

use std::env;
use std::process;
use std::io;
use std::io::BufReader;
use std::io::prelude::*;
use rustc_serialize::json::ToJson;

#[cfg_attr(rustfmt, rustfmt_skip)]
const USAGE: &'static str = "
Usage:
    human_name parse <name>
    human_name parse -
    human_name eq '<name1>' '<name2>'
    human_name eq '<name>' -

With the `eq` command, human_name will check names for equality, If '-' is the
first argument, it will expect newline-separated names from stdin to compare to
the second argument, and will print each which matches. Otherwise, it will compare
the two arguments, exiting with status 0 if the names are equal, and status 1 if
not.

With the `parse` command, it will run in parsing mode. If `-` is the argument,
it will expect newline-separated names to parse from stdin. Otherwise, it will
try to parse the arguments as a name, exiting with status 0 if it succeeds, and
status 1 otherwise. In either case it will print parsed output as JSON.
";

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 4 && args[1] == "eq" {
        equality_mode(&args);
    } else if args.len() > 2 && args[1] == "parse" {
        parse_mode(&args);
    } else {
        writeln!(&mut std::io::stderr(), "{}", USAGE).ok().unwrap();
        process::exit(64);
    }
}

fn equality_mode(args: &Vec<String>) {
    if args[2] == "-" {
        let parsed_a = human_name::Name::parse(&args[3]);
        if parsed_a.is_none() {
            writeln!(&mut std::io::stderr(), "parse failed!").ok();
            process::exit(1);
        }

        let reader = BufReader::new(io::stdin());
        for line in reader.lines() {
            match line.ok() {
                Some(input) => {
                    let parsed_b = human_name::Name::parse(&input);
                    if parsed_a == parsed_b {
                        if !writeln!(&mut io::stdout(), "{}", input.trim()).is_ok() {
                            break;
                        }
                    };
                }
                None => {
                    break;
                }
            }
        }
    } else {
        let parsed_a = human_name::Name::parse(&args[2]);
        let parsed_b = human_name::Name::parse(&args[3]);
        if parsed_a.is_none() || parsed_b.is_none() {
            writeln!(&mut std::io::stdout(), "parse failed!").ok();
            process::exit(1);
        } else if parsed_a.unwrap() != parsed_b.unwrap() {
            writeln!(&mut std::io::stdout(), "not equal!").ok();
            process::exit(1);
        }
        else {
            writeln!(&mut std::io::stdout(), "equal").ok();
            process::exit(0);
        }
    }
}

fn parse_mode(args: &Vec<String>) {
    if args[2] == "-" {
        let reader = BufReader::new(io::stdin());
        for line in reader.lines() {
            match line.ok() {
                Some(input) => {
                    let parsed = human_name::Name::parse(&input);
                    let output = match parsed {
                        Some(name) => {
                            name.to_json().to_string()
                        }
                        None => {
                            "".to_string()
                        }
                    };

                    if !writeln!(&mut io::stdout(), "{}", output).is_ok() {
                        break;
                    }
                }
                None => {
                    break;
                }
            }
        }
    } else {
        let parsed = human_name::Name::parse(&args[2..].join(" "));
        if parsed.is_none() {
            process::exit(1);
        } else {
            println!("{}", parsed.unwrap().to_json());
        }
    }
}

#[cfg(test)]
mod bench {
    use std::collections::HashSet;
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
    fn bench_parsing_needs_namecase(b: &mut Bencher) {
        b.iter(|| {
            let parsed = Name::parse("JAIME GARCIA");
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
    fn bench_equality_equal(b: &mut Bencher) {
        let x = Name::parse("Jane Doe");
        let y = Name::parse("Jane H. Doe");

        b.iter(|| {
            black_box(x == y)
        })
    }

    #[bench]
    fn bench_equality_not_equal(b: &mut Bencher) {
        let x = Name::parse("Jane Doe");
        let y = Name::parse("Foo Bar");

        b.iter(|| {
            black_box(x == y)
        })
    }

    #[bench]
    fn bench_equality_close_to_equal(b: &mut Bencher) {
        let x = Name::parse("Jane Doe");
        let y = Name::parse("John Doe");

        b.iter(|| {
            black_box(x == y)
        })
    }

    #[bench]
    fn bench_parsing_many(b: &mut Bencher) {
        let f = File::open("tests/benchmark-names.txt").ok().unwrap();
        let reader = BufReader::new(f);
        let names: Vec<String> = reader.lines().map(|l| l.ok().unwrap()).collect();

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

    #[bench]
    fn bench_equality_many(b: &mut Bencher) {
        let f = File::open("tests/benchmark-names.txt").ok().unwrap();
        let reader = BufReader::new(f);
        let names: Vec<Name> = reader
            .lines()
            .filter_map(|l| Name::parse(&l.ok().unwrap()))
            .collect();

        let mut deduped = HashSet::with_capacity(names.len() / 4);
        b.iter(|| {
            deduped.extend(names.iter());
            black_box(deduped.len())
        })
    }
}
