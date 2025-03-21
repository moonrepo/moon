use console::{set_colors_enabled, set_colors_enabled_stderr};
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
    unsafe {
        env::set_var("NO_COLOR", "1");
        // https://github.com/mitsuhiko/clicolors-control/issues/19
        env::set_var("CLICOLOR", "0");
        env::remove_var("FORCE_COLOR");
    };

    set_colors_enabled(false);
    set_colors_enabled_stderr(false);
}

pub fn setup_colors(force: bool) {
    // If being forced by --color or other env vars
    if force
        || env::var("MOON_COLOR").is_ok()
        || env::var("FORCE_COLOR").is_ok()
        || env::var("CLICOLOR_FORCE").is_ok()
    {
        let mut color_level = env::var("MOON_COLOR")
            .or_else(|_| env::var("FORCE_COLOR"))
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
            set_colors_enabled(true);
            set_colors_enabled_stderr(true);

            // https://bixense.com/clicolors/
            unsafe {
                env::set_var("CLICOLOR_FORCE", &color_level);
                env::set_var("FORCE_COLOR", &color_level);
                env::remove_var("NO_COLOR");
            };
        }

        return;
    }

    if no_color() {
        setup_no_colors();
    } else {
        unsafe { env::set_var("CLICOLOR", supports_color().to_string()) };
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serial_test::serial;

    fn reset_vars() {
        unsafe {
            env::remove_var("NO_COLOR");
            env::remove_var("CLICOLOR");
            env::remove_var("CLICOLOR_FORCE");
            env::remove_var("FORCE_COLOR");
            env::remove_var("MOON_COLOR");
        };
    }

    mod setup_color {
        use super::*;

        mod no_color {
            use super::*;

            #[test]
            #[serial]
            fn sets_vars() {
                unsafe { env::set_var("NO_COLOR", "1") };

                setup_colors(false);

                assert_eq!(env::var("CLICOLOR").unwrap(), "0");
                assert_eq!(env::var("NO_COLOR").unwrap(), "1");

                reset_vars();
            }
        }

        mod forced_color {
            use super::*;

            // #[test]
            // #[serial]
            // fn forces_via_arg() {
            //     setup_colors(true);

            //     assert_eq!(env::var("CLICOLOR_FORCE").unwrap(), "2");
            //     assert_eq!(env::var("FORCE_COLOR").unwrap(), "2");
            //     assert!(env::var("NO_COLOR").is_err());

            //     reset_vars();
            // }

            // #[test]
            // #[serial]
            // fn forces_over_no_color() {
            //     env::set_var("NO_COLOR", "1");

            //     setup_colors(true);

            //     assert_eq!(env::var("CLICOLOR_FORCE").unwrap(), "2");
            //     assert_eq!(env::var("FORCE_COLOR").unwrap(), "2");
            //     assert_eq!(env::var("NO_COLOR").unwrap(), "1");

            //     reset_vars();
            // }

            #[test]
            #[serial]
            fn disables_if_zero() {
                for var in ["MOON_COLOR", "FORCE_COLOR"] {
                    unsafe { env::set_var(var, "0") };

                    setup_colors(false);

                    assert_eq!(env::var("CLICOLOR").unwrap(), "0");
                    assert_eq!(env::var("NO_COLOR").unwrap(), "1");

                    reset_vars();
                }
            }

            #[test]
            #[serial]
            fn disables_if_false_string() {
                for var in ["MOON_COLOR", "FORCE_COLOR"] {
                    unsafe { env::set_var(var, "false") };

                    setup_colors(false);

                    assert_eq!(env::var("CLICOLOR").unwrap(), "0");
                    assert_eq!(env::var("NO_COLOR").unwrap(), "1");

                    reset_vars();
                }
            }

            #[test]
            #[serial]
            fn enables_if_empty_string() {
                for var in ["MOON_COLOR", "FORCE_COLOR"] {
                    unsafe { env::set_var(var, "") };

                    setup_colors(false);

                    assert_eq!(env::var("CLICOLOR_FORCE").unwrap(), "1");
                    assert_eq!(env::var("FORCE_COLOR").unwrap(), "1");

                    reset_vars();
                }
            }

            #[test]
            #[serial]
            fn enables_if_true_string() {
                for var in ["MOON_COLOR", "FORCE_COLOR"] {
                    unsafe { env::set_var(var, "true") };

                    setup_colors(false);

                    assert_eq!(env::var("CLICOLOR_FORCE").unwrap(), "1");
                    assert_eq!(env::var("FORCE_COLOR").unwrap(), "1");

                    reset_vars();
                }
            }
        }
    }
}
