extern crate human_name;
extern crate unicode_normalization;

use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;
use unicode_normalization::UnicodeNormalization;

fn none_if_empty(s: &str) -> Option<&str> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

#[test]
fn parsing() {
    let f = File::open("tests/parseable-names.txt").ok().unwrap();
    let reader = BufReader::new(f);

    for line in reader.lines() {
        let line: String = line.ok().unwrap().nfkd().collect();

        if line.starts_with("#") || !line.contains("|") {
            continue;
        }

        let parts: Vec<&str> = line.split("|").collect();
        let input = parts[0];
        let surname = parts[1];
        let given_name = parts[2];
        let middle_names = parts[3];
        let first_initial = parts[4].chars().nth(0).unwrap();
        let middle_initials = parts[5];
        let suffix = parts[6];

        let name = human_name::Name::parse(input);
        assert!(!name.is_none(), "[{}] Could not parse!", input);

        let given_name = none_if_empty(&given_name);
        let middle_names = none_if_empty(&middle_names);
        let middle_initials = none_if_empty(&middle_initials);
        let suffix = none_if_empty(&suffix);

        let name = name.unwrap();
        assert!(name.surname() == surname,
                "[{}] Expected surname {}, got {}",
                input,
                surname,
                name.surname());
        assert!(name.first_initial() == first_initial,
                "[{}] Expected first initial {}, got {}",
                input,
                first_initial,
                name.first_initial());
        assert!(name.given_name() == given_name,
                "[{}] Expected given_name {:?}, got {:?}",
                input,
                given_name,
                name.given_name());
        assert!(name.middle_name().map(|w| w.to_string()) == middle_names.map(|w| w.to_string()),
                "[{}] Expected middle names {:?}, got {:?}",
                input,
                middle_names,
                name.middle_name());
        assert!(name.middle_initials() == middle_initials,
                "[{}] Expected middle initials {:?}, got {:?}",
                input,
                middle_initials,
                name.middle_initials());
        assert!(name.suffix() == suffix,
                "[{}] Expected suffix {:?}, got {:?}",
                input,
                suffix,
                name.suffix());
    }
}

#[test]
fn unparseable() {
    let f = File::open("tests/unparseable-names.txt").ok().unwrap();
    let reader = BufReader::new(f);

    for line in reader.lines() {
        let line = line.ok().unwrap();

        if line.starts_with("#") {
            continue;
        }

        let result = human_name::Name::parse(&line);
        assert!(result.is_none(),
                "'Parsed' junk name: '{}' as '{}'",
                line,
                result.unwrap().display_first_last());
    }
}

#[test]
fn equality() {
    let f = File::open("tests/equal-names.txt").ok().unwrap();
    let reader = BufReader::new(f);

    for line in reader.lines() {
        let line = line.ok().unwrap();

        if line.starts_with("#") {
            continue;
        }

        let parts: Vec<&str> = line.split('|').collect();
        let a = parts[0];
        let b = parts[1];
        let expect = parts[2];

        let parsed_a = human_name::Name::parse(&a);
        let parsed_b = human_name::Name::parse(&b);

        assert!(parsed_a.is_some(), "{} was not parsed", a);
        assert!(parsed_b.is_some(), "{} was not parsed", b);

        if expect == "==" {
            assert!(parsed_a == parsed_b,
                    "{} should be equal to {} but was not!",
                    a,
                    b);
            assert!(parsed_b == parsed_a,
                    "{} should be equal to {} but was not!",
                    b,
                    a);
            assert!(parsed_a.unwrap().memoized_surname_hash() == parsed_b.unwrap().memoized_surname_hash(),
                    "{} should have the same hash as {} but did not!",
                    a,
                    b);
        } else {
            assert!(parsed_a != parsed_b,
                    "{} should not be equal to {} but was!",
                    a,
                    b);
            assert!(parsed_b != parsed_a,
                    "{} should not be equal to {} but was!",
                    b,
                    a);
        }
    }
}

#[test]
fn web_match() {
    let f = File::open("tests/web-matches.txt").ok().unwrap();
    let reader = BufReader::new(f);

    for line in reader.lines() {
        let line = line.ok().unwrap();

        if line.starts_with("#") {
            continue;
        }

        let parts: Vec<&str> = line.split('|').collect();

        let name = human_name::Name::parse(parts[1]).unwrap();
        let compare = parts[0];
        assert!(name.matches_slug_or_localpart(compare), "{} should match {} but did not!", name.display_full(), compare);
    }
}

#[test]
fn web_nonmatch() {
    let f = File::open("tests/web-nonmatches.txt").ok().unwrap();
    let reader = BufReader::new(f);

    for line in reader.lines() {
        let line = line.ok().unwrap();

        if line.starts_with("#") {
            continue;
        }

        let parts: Vec<&str> = line.split('|').collect();

        let name = human_name::Name::parse(parts[1]).unwrap();
        let compare = parts[0];
        assert!(!name.matches_slug_or_localpart(compare), "{} should not match {} but did!", name.display_full(), compare);
    }
}
