// https://github.com/clap-rs/clap/blob/master/src/util/mod.rs#L25
pub fn safe_exit(code: i32) -> ! {
    use std::io::Write;

    let _ = std::io::stdout().lock().flush();
    let _ = std::io::stderr().lock().flush();

    std::process::exit(code)
}

pub fn print_list(list: &[String]) {
    for item in list {
        println!("- {}", item);
    }
}
