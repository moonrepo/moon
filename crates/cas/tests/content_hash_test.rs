use moon_cas::ContentHash;

mod content_hash {
    use super::*;

    #[test]
    fn valid_hex() {
        let hex = "a".repeat(64);
        let hash = ContentHash::from_hex(&hex).unwrap();
        assert_eq!(hash.as_hex(), hex);
        assert_eq!(hash.prefix(), "aa");
        assert_eq!(hash.suffix().len(), 62);
    }

    #[test]
    fn rejects_short_hex() {
        let result = ContentHash::from_hex("abcd");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_non_hex() {
        let hex = "g".repeat(64);
        let result = ContentHash::from_hex(&hex);
        assert!(result.is_err());
    }

    #[test]
    fn normalizes_to_lowercase() {
        let hex = "A".repeat(64);
        let hash = ContentHash::from_hex(&hex).unwrap();
        assert_eq!(hash.as_hex(), "a".repeat(64));
    }

    #[test]
    fn display_shows_hex() {
        let hex = "b".repeat(64);
        let hash = ContentHash::from_hex(&hex).unwrap();
        assert_eq!(format!("{hash}"), hex);
    }
}
