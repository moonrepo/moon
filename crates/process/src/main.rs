use std::process::Command;
use std::env;

fn main() {
    let (shell, ext) = if cfg!(windows) {
        ("pwsh", "ps1")
    } else {
        ("bash", "sh")
    };
    let script = format!("./crates/process/args.{ext}");


    let mut paths = env::split_paths(&env::var_os("PATH").unwrap()).collect::<Vec<_>>();
    paths.push(env::current_dir().unwrap());
    let paths = env::join_paths(paths).unwrap();

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
    //
    // Windows:
    //  N/A

    #[cfg(unix)]
    Command::new(&script)
        .arg("nospace")
        .arg("with space")
        .arg("'single quote'")
        .arg("\"double quote\"")
        .arg(r#""raw quote""#)
        .env("PATH", &paths)
        .spawn()
        .unwrap();

    // Unix: Quotes are not preserved, and the script receives the inner value without wrapping quotes.
    //
    // ```
    // Args: nospace with space single quote double quote
    // Arg 1: nospace ("nospace")
    // Arg 2: with space ("with space")
    // Arg 3: single quote ("single quote")
    // Arg 4: double quote ("double quote")
    // ```
    //
    // Windows: Quotes are not preserved, and the script receives the inner value without wrapping quotes.
    //
    // ```
    // Args: nospace with space single quote double quote
    // Arg 1: nospace ('nospace')
    // Arg 2: with space ('with space')
    // Arg 3: single quote ('single quote')
    // Arg 4: double quote ('double quote')
    // ```
    Command::new(shell)
        .arg("-c")
        .arg(format!("{script} nospace 'with space' 'single quote' \"double quote\" "))
        .env("PATH", &paths)
        .spawn()
        .unwrap();
}
