# Vendored protobufs

Third-party `.proto` files, copied verbatim. **Do not edit them** — refresh from upstream instead.

`daemon.proto` imports `build/bazel/remote/execution/v2/remote_execution.proto` so it can reference
`ActionResult` directly, which is the shape `moon_cache_storage::Manifest` already converts to and
from for the remote caches.

These copies exist only so `protoc` can resolve those names. Nothing here is code-generated: every
package is mapped by `extern_path` in [`build.rs`](../../build.rs) onto the types the
[`bazel-remote-apis`](https://crates.io/crates/bazel-remote-apis) crate already generates, so there
stays exactly one Rust type per message. That crate `exclude`s its own `vendor/` directory when
publishing, which is why we need a copy at all.

Because the types come from the crate and not from these files, a drift between the two only matters
for messages `daemon.proto` actually names — today just `ActionResult`.

## Sources

| Directory     | Upstream                                                            | Pinned at                                  |
| ------------- | ------------------------------------------------------------------- | ------------------------------------------ |
| `remote-api/` | [bazelbuild/remote-apis](https://github.com/bazelbuild/remote-apis) | `v2.12.0`                                  |
| `google-api/` | [googleapis/googleapis](https://github.com/googleapis/googleapis)   | `437254f595a380cd9323111700ce0fcf9d6d2c21` |

Only the transitive import closure of `remote_execution.proto` is vendored, not the full
repositories. Well-known types (`google/protobuf/*`) are supplied by `prost-build` and are not
vendored here.

## Refreshing

Update the refs below, run it from the repository root, then `just build` — `protoc` will report any
newly added import, which needs to be fetched into `googleapis/` as well.

```bash
RA=v2.12.0
GA=437254f595a380cd9323111700ce0fcf9d6d2c21
BASE=crates/daemon-proto/proto/vendor

for f in build/bazel/remote/execution/v2/remote_execution build/bazel/semver/semver; do
  curl -sSfL --create-dirs -o "$BASE/remote-api/$f.proto" \
    "https://raw.githubusercontent.com/bazelbuild/remote-apis/$RA/$f.proto"
done

for f in google/api/annotations google/api/client google/api/field_behavior google/api/http \
  google/api/launch_stage google/longrunning/operations google/rpc/status; do
  curl -sSfL --create-dirs -o "$BASE/google-api/$f.proto" \
    "https://raw.githubusercontent.com/googleapis/googleapis/$GA/$f.proto"
done
```
