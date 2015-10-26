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
        "Em Goldman",
    ];

    for name in names.iter() {
        let name = human_name::Name::parse(name).unwrap();
        println!("{}", name.display());
    }
}
