use std::process::Command;

fn main() {
    // Unix:
    //	Quotes are preserved in strings, so they're passed as ""double quote"".
    //
    // ```
    // Args: nospace with space 'single quote' "double quote" "raw quote"
    // Arg 1: nospace ("nospace")
    // Arg 2: with space ("with space")
    // Arg 3: 'single quote' ("'single quote'")
    // Arg 4: "double quote" (""double quote"")
    // Arg 5: "raw quote" (""raw quote"")
    // ```

    Command::new("crates/process/args.sh")
        .arg("nospace")
        .arg("with space")
        .arg("'single quote'")
        .arg("\"double quote\"")
        .arg(r#""raw quote""#)
        .spawn()
        .unwrap();

    // Unix:
    //	Quotes are not preserved, and the script receives the inner value without wrapping quotes.
    //
    // ```
    // Args: nospace with space single quote double quote
    // Arg 1: nospace ("nospace")
    // Arg 2: with space ("with space")
    // Arg 3: single quote ("single quote")
    // Arg 4: double quote ("double quote")
    // ```
    Command::new("bash")
        .arg("-c")
        .arg("crates/process/args.sh nospace 'with space' 'single quote' \"double quote\" ")
        .spawn()
        .unwrap();
}
