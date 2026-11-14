# Versioning

Build ids are `<year><ISO week number><letter>`: year 2026, week 46, letters `a`, `b`, `c`... in order of
release within that week. If a build is finished in a later week, it takes that week (e.g. `202647a`).

| Build id | Cargo version | Notes |
|---|---|---|
| 202646a | 0.2.0 | Rust + Skia rewrite, OmniDoc only |
| 202646b | 0.3.0 (roadmap alpha-3) | OmniCel, OmniShow, launcher, 12 OmniDoc features, DOC import |
| 202646c | 0.3.1 | Google Material Symbols icons in all apps |

The id is the constant `BUILD_ID` in `crates/omni-common/src/lib.rs`, shown in window titles and `.omd/.omc/.oms`
manifests. Cargo needs semver, so each build id also has a fixed crate version (table above); `Cargo.toml` and `BUILD_ID` must be bumped together.
