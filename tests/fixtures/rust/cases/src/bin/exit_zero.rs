#[allow(unreachable_code)]
fn main() {
    println!("stdout");
    eprintln!("stderr");

    std::process::exit(0);

    println!("This should not appear!");
}
