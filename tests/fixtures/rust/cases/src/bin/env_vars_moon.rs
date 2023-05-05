use std::env;

fn main() {
    let mut vars = vec![];

    for (key, value) in env::vars() {
        if key.starts_with("MOON_") && !key.starts_with("MOON_TEST") && key != "MOON_VERSION" {
            vars.push(format!("{key}={}", value.replace('\\', "/")));
        }
    }

    vars.sort();

    println!("{}", vars.join("\n"));
}
