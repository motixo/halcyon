# Authoritative Settings fixtures

This isolated fixture host depends on the unchanged production
`companion/dock-islands-core` crate. It serializes `Settings` using RON 0.12.2
and serde 1.0.229, writes the bytes, then reads them with
`ron::de::from_bytes::<Settings>` and `Settings::validate`, the same calls used
by patched `islands.rs::load_settings`. Each valid serialized value must round
trip equal. Negative mutations must produce the recorded deserialization or
validation error. Valid extensions outside Python's bounded syntax must load
as enabled in Rust and raise `UnsupportedSettings` in Python.

`../settings-rust/` retains compact and pretty enabled/disabled values,
saved-output arrangements and bounds, native-invalid array/list, length,
order and position cases, and valid unsupported syntax. `outcomes.json`
records Rust's asserted results. `provenance.json` binds the fixtures, generator,
shared source and lock, and records the retained patched companion lock identity
and matching dependency checksums. The Python regression verifies these hashes.
Do not replace positives with hand-authored RON.

Reproduce from the product root (no network or app build):

```sh
cargo +1.93.0 run --locked --offline -j 1 \
  --target-dir /tmp/halcyon-settings-fixture-target \
  --manifest-path companion/fixtures/settings-generator/Cargo.toml \
  -- /tmp/halcyon-settings-fixtures-reproduced
diff -r --exclude=provenance.json companion/fixtures/settings-rust \
  /tmp/halcyon-settings-fixtures-reproduced
```

The generator emits fixtures and `outcomes.json`; `provenance.json` is the
separate retained source/byte attestation, so exclude it when comparing regenerated
output. The initial lock was reduced offline from the retained patched companion
lock (monitor-pinning evidence); every registry version and checksum was checked
against that lock and the cached archive. The fixture host has fewer dependency
features than COSMIC and is **not** a native panel build, login test, compilation
attestation for a companion ELF, catalog admission or release qualification.
Final combined artifact/native acceptance remains parent-owned and on hold.
