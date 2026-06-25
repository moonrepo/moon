use moon_hash::Digest;
use serde::Serialize;
use starbase_sandbox::create_empty_sandbox;
use starbase_utils::json::serde_json;

// Sanity-pin: SHA-256 of "abc" — used to assert that the digest path matches
// the known-good algorithm rather than just "some" hash.
const ABC_SHA256: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

#[derive(Serialize)]
struct Sample {
    name: &'static str,
    count: u32,
}

mod digest {
    use super::*;

    #[test]
    fn from_bytes_populates_hash_and_size() {
        let digest = Digest::from_bytes(b"abc").unwrap();

        assert_eq!(digest.hash.as_hex(), ABC_SHA256);
        assert_eq!(digest.size, 3);
    }

    #[test]
    fn from_bytes_handles_empty() {
        // Empty input must still produce a valid digest — size is 0 but the
        // SHA-256 of the empty string is still a valid 64-char hex string, so
        // `is_valid` is the meaningful invariant here.
        let digest = Digest::from_bytes(b"").unwrap();

        assert_eq!(digest.size, 0);
        assert!(digest.is_valid());
    }

    #[test]
    fn from_data_serializes_then_hashes() {
        let sample = Sample {
            name: "x",
            count: 1,
        };
        let digest = Digest::from_data(&sample).unwrap();

        // We don't pin the exact hash here because that would couple this test
        // to the precise serde_json output formatting. Instead we cross-check
        // against `from_bytes` over the same canonical JSON.
        let expected = Digest::from_bytes(serde_json::to_vec(&sample).unwrap()).unwrap();

        assert_eq!(digest.hash, expected.hash);
        assert_eq!(digest.size, expected.size);
    }

    #[test]
    fn from_file_matches_from_bytes() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("payload.txt", "abc");

        let from_file = Digest::from_file(sandbox.path().join("payload.txt")).unwrap();
        let from_bytes = Digest::from_bytes(b"abc").unwrap();

        assert_eq!(from_file.hash, from_bytes.hash);
        assert_eq!(from_file.size, from_bytes.size);
    }

    #[test]
    fn from_file_errors_when_missing() {
        let sandbox = create_empty_sandbox();
        let result = Digest::from_file(sandbox.path().join("missing"));
        assert!(result.is_err());
    }

    #[test]
    fn is_valid_rejects_empty_hash() {
        // A default Digest has an empty hash + zero size; this must be flagged
        // as invalid so we never accept a "blank" digest as a real one.
        let blank = Digest::default();
        assert!(!blank.is_valid());
    }

    #[test]
    fn is_valid_rejects_negative_size() {
        // The CAS layer assumes `size >= 0`. Pin the invariant so a refactor
        // can't quietly accept a negative size.
        let mut digest = Digest::from_bytes(b"abc").unwrap();
        digest.size = -1;
        assert!(!digest.is_valid());
    }

    #[test]
    fn is_valid_accepts_zero_size_for_real_hash() {
        // Empty-but-real content (e.g. an intentionally empty output file) is
        // still valid as long as the hash itself is the SHA-256 of "".
        let digest = Digest::from_bytes(b"").unwrap();
        assert!(digest.is_valid());
    }

    #[test]
    fn equality_uses_hash_and_size() {
        let a = Digest::from_bytes(b"abc").unwrap();
        let b = Digest::from_bytes(b"abc").unwrap();
        assert_eq!(a, b);

        let mut c = a.clone();
        c.size = a.size + 1;
        // Size mismatch must break equality even when the hash collides — the
        // CAS layer trusts the (hash, size) pair, not just the hash.
        assert_ne!(a, c);
    }

    #[test]
    fn distinct_inputs_yield_distinct_hashes() {
        let a = Digest::from_bytes(b"abc").unwrap();
        let b = Digest::from_bytes(b"abd").unwrap();
        assert_ne!(a.hash, b.hash);
    }
}
