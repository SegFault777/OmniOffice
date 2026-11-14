# Versioning

Build ids are `<year><ISO week number><letter>`: year 2026, week 46, letters `a`, `b`, `c`... in order of
release within that week. If a build is finished in a later week, it takes that week (e.g. `202647a`).

| Build id | Former name | Notes |
|---|---|---|
| 202646a | 0.2.0 | Rust + Skia rewrite, OmniDoc only |
| 202646b | roadmap alpha-3 / 0.3.0 | OmniCel, OmniShow, launcher, 12 OmniDoc features, DOC import |

The id is the constant `BUILD_ID` in `crates/omni-common/src/lib.rs`, shown in window titles and `.omd/.omc/.oms`
manifests. Cargo needs semver, so crate versions stay on a placeholder (`0.3.0`).
