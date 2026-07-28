fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto");

    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        // Generate `bytes` fields as `bytes::Bytes` instead of `Vec<u8>`, so
        // blob payloads don't have to be copied out of a request.
        .bytes(".")
        // `daemon.proto` imports the Bazel Remote Execution protos so it can
        // reference `ActionResult` directly. The vendored copies under
        // `proto/vendor` exist only so protoc can resolve those names —
        // `extern_path` maps every one of them onto the types the
        // `bazel-remote-apis` crate already generates, so nothing here is
        // regenerated and there is a single Rust type per message.
        .extern_path(
            ".build.bazel.remote.execution.v2",
            "::bazel_remote_apis::build::bazel::remote::execution::v2",
        )
        .extern_path(
            ".build.bazel.semver",
            "::bazel_remote_apis::build::bazel::semver",
        )
        .extern_path(".google.api", "::bazel_remote_apis::google::api")
        .extern_path(
            ".google.longrunning",
            "::bazel_remote_apis::google::longrunning",
        )
        .extern_path(".google.rpc", "::bazel_remote_apis::google::rpc")
        .compile_protos(
            &["proto/daemon.proto"],
            &["proto/", "vendor/google-api", "vendor/remote-api"],
        )?;

    Ok(())
}
