extern crate human_name;
extern crate serde_json;

use std::env;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::process;

const USAGE: &str = "
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

fn equality_mode(args: &[String]) {
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
                        let result = writeln!(&mut io::stdout(), "{}", input.trim());
                        if result.is_err() {
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
            writeln!(&mut std::io::stdout(), "n").ok();
            process::exit(1);
        } else {
            writeln!(&mut std::io::stdout(), "y").ok();
            process::exit(0);
        }
    }
}

fn parse_mode(args: &[String]) {
    if args[2] == "-" {
        let reader = BufReader::new(io::stdin());
        for line in reader.lines() {
            match line.ok() {
                Some(input) => {
                    let parsed = human_name::Name::parse(&input);
                    let output = match parsed {
                        Some(name) => serde_json::to_string(&name).unwrap(),
                        None => "".to_string(),
                    };

                    if writeln!(&mut io::stdout(), "{}", output).is_err() {
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
            println!("{}", serde_json::to_string(&parsed.unwrap()).unwrap());
        }
    }
}
