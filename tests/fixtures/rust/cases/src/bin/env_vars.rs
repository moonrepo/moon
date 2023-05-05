use std::env;

fn main() {
    if let Ok(value) = env::var("MOON_FOO") {
        println!("MOON_FOO={}", value);
    }

    if let Ok(value) = env::var("MOON_BAR") {
        println!("MOON_BAR={}", value);
    }

    if let Ok(value) = env::var("MOON_BAZ") {
        println!("MOON_BAZ={}", value);
    }
}
