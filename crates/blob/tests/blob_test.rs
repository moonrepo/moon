use bytes::Bytes;
use moon_blob::Blob;
use moon_hash::ContentHash;
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

mod blob {
    use super::*;

    #[test]
    fn from_bytes_retains_bytes_and_digest() {
        let blob = Blob::from_bytes(b"abc".to_vec()).unwrap();

        assert_eq!(blob.bytes, Bytes::from("abc"));
        assert_eq!(blob.digest.hash.as_hex(), ABC_SHA256);
        assert_eq!(blob.digest.size, 3);
    }

    #[test]
    fn from_data_round_trips_through_json() {
        let sample = Sample {
            name: "y",
            count: 7,
        };
        let blob = Blob::from_data(&sample).unwrap();

        // The blob's bytes should be the canonical serde_json output. We then
        // confirm that re-hashing those bytes reproduces the blob's digest.
        let canonical = serde_json::to_vec(&sample).unwrap();
        assert_eq!(blob.bytes, canonical);
        assert_eq!(
            blob.digest.hash,
            ContentHash::hash_bytes(&canonical).unwrap()
        );
    }

    #[test]
    fn from_file_round_trips() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("payload.bin", "hello bytes");

        let blob = Blob::from_file(sandbox.path().join("payload.bin")).unwrap();

        assert_eq!(blob.bytes, Bytes::from("hello bytes"));
        assert_eq!(blob.digest.size, "hello bytes".len() as i64);
        assert_eq!(
            blob.digest.hash,
            ContentHash::hash_bytes(b"hello bytes").unwrap()
        );
    }

    #[test]
    fn into_input_keeps_the_digest_and_inlines_the_bytes() {
        // Converting to an input must not re-hash: the digest travels with the
        // bytes, so a blob built from one source can be stored without a second
        // read or hash pass.
        let blob = Blob::from_bytes(b"abc".to_vec()).unwrap();
        let digest = blob.digest.clone();
        let input = blob.into_input();

        assert_eq!(input.digest, digest);
        assert_eq!(input.content.get_bytes(), Some(b"abc".as_slice()));
    }

    #[test]
    fn into_input_round_trips_back_into_a_blob() {
        let blob = Blob::from_bytes(b"abc".to_vec()).unwrap();
        let round_tripped = blob.clone().into_input().into_blob().unwrap();

        assert_eq!(round_tripped.bytes, blob.bytes);
        assert_eq!(round_tripped.digest, blob.digest);
    }

    #[test]
    fn debug_does_not_dump_bytes() {
        // Bytes may be large (or sensitive). The Debug impl deliberately omits
        // them — this regression guard catches an accidental `derive(Debug)`
        // that would expose the full payload.
        let blob = Blob::from_bytes(vec![0xAB, 0xCD, 0xEF]).unwrap();
        let dbg = format!("{:?}", blob);
        assert!(dbg.contains("digest"));
        assert!(!dbg.contains("bytes"));
    }
}
