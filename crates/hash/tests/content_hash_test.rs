use moon_hash::{ContentHash, hash_sha256};
use starbase_sandbox::create_empty_sandbox;

// Known SHA-256 digests for fixed inputs. Hard-coding these guards against an
// accidental hash algorithm swap or change in canonical encoding.
const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
const ABC_SHA256: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

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
    fn rejects_long_hex() {
        // Off-by-one guard: 65 chars must fail to match the 64-char invariant.
        let hex = "a".repeat(65);
        assert!(ContentHash::from_hex(hex).is_err());
    }

    #[test]
    fn rejects_empty_hex() {
        assert!(ContentHash::from_hex("").is_err());
    }

    #[test]
    fn display_shows_hex() {
        let hex = "b".repeat(64);
        let hash = ContentHash::from_hex(&hex).unwrap();
        assert_eq!(format!("{hash}"), hex);
    }

    #[test]
    fn as_ref_str_returns_hex() {
        let hex = "c".repeat(64);
        let hash = ContentHash::from_hex(&hex).unwrap();
        // Exercises both AsRef<str> and the Deref<Target = str> impls so a
        // refactor to either trait surface gets caught here.
        let as_ref: &str = hash.as_ref();
        assert_eq!(as_ref, hex);
        assert_eq!(&*hash, hex);
    }

    #[test]
    fn hash_bytes_matches_known_sha256() {
        // Pins ContentHash::hash_bytes to SHA-256, not just "some" hash algo.
        let hash = ContentHash::hash_bytes(b"abc").unwrap();
        assert_eq!(hash.as_hex(), ABC_SHA256);
    }

    #[test]
    fn hash_bytes_is_deterministic() {
        let a = ContentHash::hash_bytes(b"hello world").unwrap();
        let b = ContentHash::hash_bytes(b"hello world").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn hash_bytes_distinguishes_inputs() {
        let a = ContentHash::hash_bytes(b"abc").unwrap();
        let b = ContentHash::hash_bytes(b"abd").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn hash_bytes_handles_empty_input() {
        let hash = ContentHash::hash_bytes(b"").unwrap();
        assert_eq!(hash.as_hex(), EMPTY_SHA256);
    }
}

mod hash_sha256_fn {
    use super::*;

    #[test]
    fn matches_known_digest() {
        // Free-function `hash_sha256` is also a public API surface — pin it
        // separately so future refactors that route through different code
        // paths still produce the same digest.
        assert_eq!(hash_sha256(b"abc"), ABC_SHA256);
        assert_eq!(hash_sha256(b""), EMPTY_SHA256);
    }

    #[test]
    fn agrees_with_content_hash_hash_bytes() {
        // Cross-checks the two public hashing entry points so they can't
        // diverge silently.
        let direct = hash_sha256(b"some payload");
        let via_content = ContentHash::hash_bytes(b"some payload").unwrap();
        assert_eq!(direct, via_content.as_hex());
    }
}

mod hash_file {
    use super::*;

    #[test]
    fn matches_hash_bytes_for_same_contents() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("payload.txt", "abc");

        let from_file = ContentHash::hash_file(sandbox.path().join("payload.txt")).unwrap();
        let from_bytes = ContentHash::hash_bytes(b"abc").unwrap();

        assert_eq!(from_file, from_bytes);
        assert_eq!(from_file.as_hex(), ABC_SHA256);
    }

    #[test]
    fn handles_empty_file() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("empty.txt", "");

        let hash = ContentHash::hash_file(sandbox.path().join("empty.txt")).unwrap();

        assert_eq!(hash.as_hex(), EMPTY_SHA256);
    }

    #[test]
    fn errors_on_missing_file() {
        let sandbox = create_empty_sandbox();
        let result = ContentHash::hash_file(sandbox.path().join("does-not-exist"));

        assert!(result.is_err());
    }
}
