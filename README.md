# human-name
A library and command-line client for parsing and comparing human names.

[![Build Status](https://github.com/djudd/human-name/actions/workflows/rust.yml/badge.svg)](https://github.com/djudd/human-name/actions/workflows/rust.yml)

[`Documentation`](https://docs.rs/human_name)

# Uses and limitations

If you are trying to programmatically put human names into a canonical format,
you are of course in [a state of sin](http://www.kalzumeus.com/2010/06/17/falsehoods-programmers-believe-about-names/).
But sometimes, something is better than nothing.

If you want to let your users enter arbitrary names but try to use a given name
in a salutation, or you're looking for a person's index in PubMed's list of authors
for a co-authored paper, or you want to extract surnames from unstructured name
entries for a frequency analysis, you might find this useful.

Most names are not unique. `human_name` will tell you that "J. Doe" might be "Jane Doe",
but of course it might not be, and even "Jane Doe" might not be the other "Jane Doe."
The comparison logic here is only useful if you have external reason to believe
two names may represent the same person.

`human_name` will work best on Latin names - i.e., data from North or South America
and/or Europe. For example, it doesn't understand surname-first formats without
commas, common in East Asia: "Park Geun-hye" will be parsed as having the given
name "Park", and the last name "Guen-hye". And it doesn't handle single-word names.
It won't blow up on Unicode, and it handles non-ASCII punctuation and accents
with some intelligence, but don't feed in Arabic or Han characters and expect
better results than a naive whitespace or word-boundary split.

`human_name` tries to fail nicely, such that if parsing fails, either it will do
so explicitly, returning nothing, or at least, calling `display_full` on the result
will return the input, modulo whitespace. But there are no guarantees.

`human_name` tries aggressively to treat strings as names, which makes it
 definitely _not_ suitable for extracting names from a larger piece of text
(although it will strip titles, nicknames, etc, from a name field.)

Because the goals of this library include both name comparison and memory efficiency,
parsed names are Unicode NFKD-normalized and capitalized in a conventional way
(handling "Mc" and a few other edge cases), and the raw input is not preserved.

# From Rust code

```rust
use human_name::Name;

let jane_doe = Name::parse("Jane Doe").unwrap();
let john_doe = Name::parse("John Doe").unwrap();
let j_doe = Name::parse("Doe, J.").unwrap();

assert!(jane_doe.consistent_with(&j_doe));
assert!(!jane_doe.consistent_with(&john_doe));

let oscar = Name::parse("MR OSCAR DE LA HOYA JR").unwrap();
assert_eq!(Some("Oscar"), oscar.given_name());
assert_eq!("de la Hoya", oscar.surname());
assert_eq!(Some("Jr."), oscar.generational_suffix());
assert_eq!(Some("Mr."), oscar.honorific_prefix());
assert_eq!("Oscar de la Hoya, Jr.", oscar.display_full());

assert!(Name::parse("foo@bar.com").is_none());
```

See the [docs](https://docs.rs/human_name) for details.

# From the command line

There are two modes, "parse" and "eq". The mode is passed as the first argument.
You can pass input as subsequent arguments:

```bash
$ human_name parse "Jane Doe"
{"first_initial":"J","given_name":"Jane","surname":"Doe"}

$ human_name parse "MR OSCAR DE LA HOYA JR"
{"first_initial":"O","given_name":"Oscar","generational_suffix":"Jr.","surname":"de la Hoya"}

$ human_name eq "Jane Doe" "Jane M. Doe"
y
$ echo $?
0

$ human_name eq "Jane M. Doe" "Jane H. Doe"
n
$ echo $?
1
```

Or, with the second argument "-", you can pass input on stdin. For example,
to find the most common surnames in a file of newline-delimited names:

```bash
$ human_name parse - < names.txt | jq .surname | sort | uniq -c | sort -nr | head -n3
111 "Zhang"
109 "Li"
106 "Wang"
```

To find all the possible "J. Smith"s in a file of newline-delimited names:

```bash
$ human_name eq - "J Smith" < names.text
Smith, Jason A.
Jay Smith
```

# Optional Features

The following features are enabled by default, but can be turned off when `human_name`
is being used as a library.

## name_eq_hash

Implements `Eq` for `Name` using `consistent_with`, and `Hash` using `surname_hash`.

Optional because both of these implementations are questionable for general-purpose
use: this `Eq` is not transitive, and `Hash` is collision-prone. See docs for details.

## serialization

Implements serialization for `Name` using `serde`. This serialization format is
intended to allow programs not using `human_name` to see the parse results;
deserialization isn't implemented because when round-tripping is desired, just
using `display_full` to serialize as a string and then `parse` to deserialize
should produce a more compact and reasonably performant result.

# Bindings in other languages

Ruby bindings using the `ffi` gem are available at [github.com/djudd/human-name-rb](https://github.com/djudd/human-name-rb)

Python bindings using the `ctypes` module are available at [github.com/djudd/human-name-py](https://github.com/djudd/human-name-py)

# Versioning

This library follows [semver](https://semver.org/) with respect to its programmatic API.
Changes to the parsing and consistency-checking heuristics are not considered breaking
and will typically come with a minor version bump.

# Performance

As of version 1.0, the fast path (roughly, two space-separated, titlecase ASCII
words) for name parsing takes ~250ns and does not heap-allocate. Comparing two
simple names for consistency takes ~100ns if the surname hash matches, and ~1ns
otherwise. Pathological cases for either can take an order of magnitude longer.

# Contributing

Contributions, feature requests and bug reports are welcome. Please open a GitHub issue
or pull request, include as much helpful context as you can, and we'll figure it out from there.
As this is a small personal project, you might not get an immediate response, but I'll follow up
as soon as I'm able.

Conversations related to this project must follow the [Contributor Covenent's Code of Conduct](https://www.contributor-covenant.org/version/2/1/code_of_conduct/code_of_conduct.md).
Please report any related concerns to the contact email in [Cargo.toml](https://github.com/djudd/human-name/blob/master/Cargo.toml).

# Credit

Inspiration, heuristics, and test cases were taken from:
* [`people` (Ruby)](https://github.com/academia-edu/people)
* [`nameparser` (Python)](https://github.com/derek73/python-nameparser/)
* [`HumanNameParse` (Java)](https://github.com/tupilabs/HumanNameParser.java)
* [`namae` (Ruby)](https://github.com/berkmancenter/namae)
* [`Lingua::EN::NameParse` (Perl)](http://search.cpan.org/~kimryan/Lingua-EN-NameParse-1.33/lib/Lingua/EN/NameParse.pm) (probably the original for some of the other ports as well)
* [`Lingua::EN::Nickname` (Perl)](http://search.cpan.org/~brianl/Lingua-EN-Nickname-1.16/Nickname.pm)
* [`parse-full-name` (JS)](https://github.com/dschnelldavis/parse-full-name)
* [`PHP-Name-Parser` (PHP)](https://github.com/joshfraser/PHP-Name-Parser)

# License

Apache 2.0 - see LICENSE.
