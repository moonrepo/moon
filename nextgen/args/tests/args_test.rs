use moon_args::{join_args, split_args};

mod split_args {
    use super::*;

    #[test]
    fn normal_args() {
        assert_eq!(
            split_args("bin arg1 arg2 -o --opt val").unwrap(),
            vec!["bin", "arg1", "arg2", "-o", "--opt", "val"]
        );
    }

    #[test]
    fn with_delim() {
        assert_eq!(
            split_args("bin arg1 -- extra args").unwrap(),
            vec!["bin", "arg1", "--", "extra", "args"]
        );
    }

    #[test]
    fn single_quotes() {
        assert_eq!(split_args("bin 'foo bar'").unwrap(), vec!["bin", "foo bar"]);
    }

    #[test]
    fn double_quotes() {
        assert_eq!(
            split_args("bin \"foo bar\"").unwrap(),
            vec!["bin", "foo bar"]
        );
    }

    #[test]
    fn special_chars() {
        assert_eq!(
            split_args("bin @dir/path").unwrap(),
            vec!["bin", "@dir/path"]
        );
    }

    #[test]
    fn multi_and() {
        assert_eq!(
            split_args("bin1 arg && bin2 arg").unwrap(),
            vec!["bin1", "arg", "&&", "bin2", "arg"]
        );

        assert_eq!(
            split_args("bin1 arg  &&  bin2 arg").unwrap(),
            vec!["bin1", "arg", "&&", "bin2", "arg"]
        );
    }

    #[test]
    fn multi_semicolon() {
        assert_eq!(
            split_args("bin1 arg; bin2 arg").unwrap(),
            vec!["bin1", "arg", ";", "bin2", "arg"]
        );

        assert_eq!(
            split_args("bin1 arg  ;  bin2 arg").unwrap(),
            vec!["bin1", "arg", ";", "bin2", "arg"]
        );
    }
}

mod join_args {
    use super::*;

    #[test]
    fn normal_args() {
        assert_eq!(
            join_args(vec!["bin", "arg1", "arg2", "-o", "--opt", "val"]),
            "bin arg1 arg2 -o --opt val"
        );
    }

    #[test]
    fn with_delim() {
        assert_eq!(
            join_args(vec!["bin", "arg1", "--", "extra", "args"]),
            "bin arg1 -- extra args"
        );
    }

    #[test]
    fn quotes() {
        assert_eq!(join_args(vec!["bin", "foo bar"]), "bin 'foo bar'");
    }

    #[test]
    fn special_chars() {
        assert_eq!(join_args(vec!["bin", "@dir/path"]), "bin @dir/path");
    }

    #[test]
    fn multi_and() {
        assert_eq!(
            join_args(vec!["bin1", "arg", "&&", "bin2", "arg"]),
            "bin1 arg && bin2 arg"
        );
    }

    #[test]
    fn multi_semicolon() {
        assert_eq!(
            join_args(vec!["bin1", "arg", ";", "bin2", "arg"]),
            "bin1 arg ; bin2 arg"
        );
    }
}
