extern crate human_name;

fn main() {
    let names = [
        "Poppy P E L D Donahue"
    ];

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
