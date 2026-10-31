//! Hello Mobile on desktop. The game is the `hello_mobile` library, which Android runs too.

fn main() {
    if let Err(e) = hello_mobile::run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
