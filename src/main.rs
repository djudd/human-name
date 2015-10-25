extern crate human_name;

fn main() {
    let names = [
        "Emma Goldman", 
        "Emma ('Em') Goldman", 
        "Emma \"the anarchist\" Goldman", 
        "Emma Goldman, M.D.", 
        "Emma Goldman, esq", 
        "Goldman, Emma",
    ];

    for name in names.iter() {
        let name = human_name::Name::parse(name).unwrap();
        println!("{}", name.display());
    }
}
