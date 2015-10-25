extern crate human_name;

fn main() {
    let name = human_name::Name::new("Emma Goldman").unwrap();
    println!("{}", name.display());
}
