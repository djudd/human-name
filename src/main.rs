extern crate human_name;

fn main() {
    let names = [
        "Emma Goldman", 
        "Emma ('Em') Goldman", 
        "Emma \"anarchy\" Goldman", 
        "Emma Goldman, M.D.", 
        "Emma Goldman, esq", 
        "Emma Goldman Jr.", 
        "Goldman, Emma",
        "Deputy Secretary of State Emma Goldman",
        "Dr. Emma Goldman",
        "EM Goldman",
        "Em Goldman",
        "Goldman, Em",
        "Emma de la Goldman",
        "Emma May Goldman",
        "Emma Van Goldman",
        "Emma Goldman y Rodriguez",
        "E.M. Goldman", 
        "EM GOLDMAN",
        "MM GOLDMAN",
        "EM Goldman",
        "em goldman",
        "e. goldman",
        "e goldman",
        "e. m. m. goldman",
        "E M M GOLDMAN",
        "E. Emma Goldman",
        "Emma M. Goldman",
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
