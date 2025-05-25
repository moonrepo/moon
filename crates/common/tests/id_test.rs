use moon_common::Id;

mod id {
    use super::*;

    fn symbols() -> Vec<&'static str> {
        vec![".", "-", "_", "/"]
    }

    #[test]
    fn ascii() {
        for s in symbols() {
            assert!(Id::new(format!("abc{s}123")).is_ok());
        }

        assert!(Id::new("a.b-c_d/e").is_ok());
        assert!(Id::new("@a1").is_ok());
    }

    #[test]
    fn unicode() {
        for s in symbols() {
            assert!(Id::new(format!("ąęóąśłżźń{s}123")).is_ok());
        }

        assert!(Id::new("ą.ó-ą_ł/ń").is_ok());
        assert!(Id::new("@ż9").is_ok());
    }

    #[test]
    fn no_punc() {
        for p in ["'", "\"", "?", "?", "[", "}", "~", "`", "!", "@", "$"] {
            assert!(Id::new(format!("sbc{p}123")).is_err());
        }
    }
}
