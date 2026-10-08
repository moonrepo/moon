# warpgate: stop serializing concurrent plugin calls

A plan for the warpgate crate in the proto repository. It was found while investigating the 1000
project workspace graph bench in moon, on 2026-10-07, against warpgate 0.36.4 (the version in moon's
`Cargo.lock`) and moon's `3.0-async-graph` at `f0159dd9c6`. Line references are for 0.36.4, and may
have drifted in the proto repository.

The moon side of the same problem is already fixed in moon (`PluginRegistry::get_instance` and
`load_many`, the toolchain location caches, and task state reads). This plan is the remaining half.

## Summary

scc's `entry_async` (and `get_async`) take an **exclusive** bucket lock, even when the entry already
exists. warpgate uses `entry_async` on every call to `PluginContainer::has_func` and
`PluginContainer::cache_func_with`, which are hit for nearly every plugin call. When many tasks call
the same plugin concurrently, they all queue on the same bucket lock, and each handoff needs a tokio
wakeup. Under CPU contention a slow wakeup stalls everything queued behind it, which becomes a lock
convoy that lasts until the queue drains.

In moon, project builds run 10 at a time, and every task calls `has_func` and `cache_func_with` (via
`define_requirements`) on each toolchain. In the bench, whole iterations stalled 10 to 20x, and on a
busy machine the outliers reached multiple seconds.

## Evidence

moon's bench (`crates/workspace/benches/workspace_graphs.rs`, `build_graphs/1000`), 100 samples
each:

| Variant                             | Median  | Mean     | Std dev  | Max       | > 2x median |
| :---------------------------------- | :------ | :------- | :------- | :-------- | :---------- |
| Before                              | 84.3 ms | 206.6 ms | 305.7 ms | 1410.6 ms | 18          |
| moon fix only                       | 83.2 ms | 88.5 ms  | 24.9 ms  | 283.2 ms  | 2           |
| moon fix, and this plan (W1 and W2) | 83.4 ms | 84.6 ms  | 6.8 ms   | 146.4 ms  | 0           |

With 10 extra busy processes (`yes > /dev/null`), 100 iterations of the same build:

| Variant                             | Median   | Max       | > 2x median |
| :---------------------------------- | :------- | :-------- | :---------- |
| Before                              | 162.1 ms | 4116.2 ms | 40          |
| moon fix only                       | 132.0 ms | 2267.4 ms | 20          |
| moon fix, and this plan (W1 and W2) | 124.5 ms | 180.9 ms  | 0           |

So the moon fix alone isn't enough, and W1 and W2 are required. Ruled out along the way: concurrent
plugin loads (exactly 1 per build), the allocator (mimalloc had the same outliers), and config
loading threads. A concurrency of 1 had no outliers at all.

## Changes

### W1. `has_func`: read with a shared lock first

[plugin.rs:360](https://github.com/moonrepo/proto/blob/master/crates/warpgate/src/plugin.rs) uses
`self.func_cache.entry_async(func.into())`, and on a miss holds the entry across
`self.plugin.read().await`. Read first, and only insert on a miss, without holding a map lock across
the await:

```rust
if let Some(exists) = self.func_cache.read_async(func, |_, data| data[0] == 1).await {
    return exists;
}

let exists = self.plugin.read().await.function_exists(func);
let _ = self.func_cache.insert_async(func.into(), vec![exists as u8]).await;

exists
```

**Alternative (preferred if possible):** a plugin's exports never change, so if extism can list
them, collect them once in `PluginContainer::new` into an immutable set, and `has_func` becomes a
lookup with no lock or await. Check whether extism 1.30 exposes the module's exported function
names.

### W2. `cache_func_with`: read with a shared lock first, and never call under a lock

[plugin.rs:211](https://github.com/moonrepo/proto/blob/master/crates/warpgate/src/plugin.rs) uses
`self.func_cache.entry_async(cache_key)`, and on a miss holds the entry across
`self.call(func, input).await`, which waits for the plugin's `RwLock` and a `spawn_blocking` WASM
call. Keep the `!self.cache` early return and the cache key, and replace the match with:

```rust
if let Some(output) = self
    .func_cache
    .read_async(&cache_key, |_, data| self.parse_output(func, data))
    .await
{
    return output;
}

let data = self.call(func, input).await?;
let output: O = self.parse_output(func, &data)?;

let _ = self.func_cache.insert_async(cache_key, data).await;

Ok(output)
```

**Trade-off:** concurrent misses for the same input now all call the plugin, instead of waiting for
the first. Misses are rare, and the cached functions are deterministic for a given input, so this is
acceptable. Note it in a code comment. If single-flight is wanted, use a per-key `OnceCell` cloned
out of the map (as moon's task graph does), never a map entry held across an await.

### W3. `check_cache_or_save`: don't hold a map entry while waiting for the load lock

[loader.rs:341](https://github.com/moonrepo/proto/blob/master/crates/warpgate/src/loader.rs) does
`let entry = self.locks.entry_async(hash.clone()).await.or_default();` and then
`let _lock = entry.lock().await;`. The `OccupiedEntry` (and its bucket lock) is held for the whole
load, which may download a plugin for seconds. Other plugins whose hashes share the bucket are
blocked for that long. This is plugin loading, not the per-call hot path, so it's lower priority,
but it's the same pattern that moon's registry loader warns can deadlock. Clone the mutex out, and
drop the entry before awaiting:

```rust
let mutex = Arc::clone(&*self.locks.entry_async(hash.clone()).await.or_default());
let _lock = mutex.lock().await;
```

The map's values are already `Arc<Mutex<()>>`, so this is a small change.

### Also check

Search the proto workspace for other `get_async`, `get_sync`, `entry_async`, and `entry_sync` calls
that only read, or that hold the entry across an `.await`, especially on paths that run per call or
per tool (like proto's tool and version caches), and apply the same fix.

## Tests

- `has_func` returns the same result for existing and missing functions, on the first and later
  calls.
- `cache_func_with` returns the cached output on a second call with the same input, without calling
  the plugin again (assert with the `on_call_func` callback, or a call counter in a test plugin),
  and still calls the plugin when `WARPGATE_NO_FUNC_CACHE` is set.
- If the test suite has a pattern for it: many concurrent `has_func` and `cache_func_with` calls on
  one container all complete, and return the same results.

## Verifying in moon

1. In moon's root `Cargo.toml`, add a temporary patch (the existing `[patch.crates-io]` section is
   commented out, so add a new one at the end):

   ```toml
   [patch.crates-io]
   warpgate = { path = "../proto/crates/warpgate" }
   ```

   If proto's warpgate version no longer matches the one in moon's `Cargo.lock`, the patch won't
   apply, so bump moon's `warpgate` dependency to that version as well.

2. Run the bench, and compare against the table above:

   ```shell
   CARGO_TARGET_DIR=~/.cargo/shared-target cargo bench -p moon_workspace --bench workspace_graphs -- 'build_graphs/1000$'
   ```

   Per-sample times are in
   `~/.cargo/shared-target/criterion/WorkspaceBuilder/build_graphs/1000/new/sample.json`. Expect a
   standard deviation in the single digits of milliseconds, and no sample over 2x the median.

3. To stress it, run the bench again while 10 `yes > /dev/null` processes run in the background
   (stop them with `pkill -x yes` afterwards). Outliers come back without W1 and W2.

4. Remove the patch, release warpgate (and proto_core, if it re-exports it), and bump moon's
   `warpgate` dependency.
