use moon_env_var::GlobalEnvBag;
use starbase_styles::color::{no_color, supports_color};
use std::env;
use std::ffi::OsString;

pub fn is_arg_executable(arg: &str) -> bool {
    arg.ends_with("moon") || arg.ends_with("moon.exe") || arg.ends_with("moon.js")
}

pub fn gather_args() -> (Vec<OsString>, bool) {
    let mut args: Vec<OsString> = vec![];
    let mut leading_args: Vec<OsString> = vec![];
    let mut check_for_target = true;
    let mut has_executable = false;

    env::args_os().enumerate().for_each(|(index, arg)| {
        if let Some(a) = arg.to_str() {
            // Script being executed, so persist it
            if index == 0 && is_arg_executable(a) {
                leading_args.push(arg);
                has_executable = true;
                return;
            }

            // Find first non-option value
            if check_for_target && !a.starts_with('-') {
                check_for_target = false;

                // Looks like a target, but is not `run`, so prepend!
                if a.contains(':') {
                    leading_args.push(OsString::from("run"));
                }
            }
        }

        args.push(arg);
    });

    // We need a separate args list because options before the
    // target cannot be placed before "run"
    leading_args.extend(args);

    (leading_args, has_executable)
}

pub fn setup_no_colors() {
    let bag = GlobalEnvBag::instance();
    bag.set("NO_COLOR", "1");
    // https://github.com/mitsuhiko/clicolors-control/issues/19
    bag.set("CLICOLOR", "0");
    bag.remove("FORCE_COLOR");
}

pub fn setup_colors(force: bool) {
    let bag = GlobalEnvBag::instance();

    // If being forced by --color or other env vars
    if force || bag.has("MOON_COLOR") || bag.has("FORCE_COLOR") || bag.has("CLICOLOR_FORCE") {
        let mut color_level = bag
            .get("MOON_COLOR")
            .or_else(|| bag.get("FORCE_COLOR"))
            .unwrap_or("3".to_owned());

        // https://nodejs.org/api/cli.html#force_color1-2-3
        if color_level.is_empty() || color_level == "true" {
            color_level = "1".to_owned();
        } else if color_level == "false" {
            color_level = "0".to_owned();
        }

        if color_level == "0" {
            setup_no_colors();
        } else {
            // https://bixense.com/clicolors/
            bag.set("CLICOLOR_FORCE", &color_level);
            bag.set("FORCE_COLOR", &color_level);
            bag.remove("NO_COLOR");
        }

        return;
    }

    if no_color() {
        setup_no_colors();
    } else {
        bag.set("CLICOLOR", supports_color().to_string());
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serial_test::serial;

    fn reset_vars() {
        let bag = GlobalEnvBag::instance();
        bag.remove("NO_COLOR");
        bag.remove("CLICOLOR");
        bag.remove("CLICOLOR_FORCE");
        bag.remove("FORCE_COLOR");
        bag.remove("MOON_COLOR");
    }

    mod setup_color {
        use super::*;

        mod no_color {
            use super::*;

            #[test]
            #[serial]
            fn sets_vars() {
                let bag = GlobalEnvBag::instance();
                bag.set("NO_COLOR", "1");

                setup_colors(false);

                assert_eq!(bag.get("CLICOLOR").unwrap(), "0");
                assert_eq!(bag.get("NO_COLOR").unwrap(), "1");

                reset_vars();
            }
        }

        mod forced_color {
            use super::*;

            #[test]
            #[serial]
            fn disables_if_zero() {
                let bag = GlobalEnvBag::instance();

                for var in ["MOON_COLOR", "FORCE_COLOR"] {
                    bag.set(var, "0");

                    setup_colors(false);

                    assert_eq!(bag.get("CLICOLOR").unwrap(), "0");
                    assert_eq!(bag.get("NO_COLOR").unwrap(), "1");

                    reset_vars();
                }
            }

            #[test]
            #[serial]
            fn disables_if_false_string() {
                let bag = GlobalEnvBag::instance();

                for var in ["MOON_COLOR", "FORCE_COLOR"] {
                    bag.set(var, "false");

                    setup_colors(false);

                    assert_eq!(bag.get("CLICOLOR").unwrap(), "0");
                    assert_eq!(bag.get("NO_COLOR").unwrap(), "1");

                    reset_vars();
                }
            }

            #[test]
            #[serial]
            fn enables_if_empty_string() {
                let bag = GlobalEnvBag::instance();

                for var in ["MOON_COLOR", "FORCE_COLOR"] {
                    bag.set(var, "");

                    setup_colors(false);

                    assert_eq!(bag.get("CLICOLOR_FORCE").unwrap(), "1");
                    assert_eq!(bag.get("FORCE_COLOR").unwrap(), "1");

                    reset_vars();
                }
            }

            #[test]
            #[serial]
            fn enables_if_true_string() {
                let bag = GlobalEnvBag::instance();

                for var in ["MOON_COLOR", "FORCE_COLOR"] {
                    bag.set(var, "true");

                    setup_colors(false);

                    assert_eq!(bag.get("CLICOLOR_FORCE").unwrap(), "1");
                    assert_eq!(bag.get("FORCE_COLOR").unwrap(), "1");

                    reset_vars();
                }
            }
        }
    }
}
