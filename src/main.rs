use atc_router::parse;

fn main() {
    println!("{}", parse("a == 1.1.1.0/24").unwrap());
}
