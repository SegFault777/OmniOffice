# Versioning

Build ids are `<year><ISO week number><letter>`: year 2026, week 46, letters `a`, `b`, `c`... in order of
release within that week. If a build is finished in a later week, it takes that week (e.g. `202647a`).

| Build id | Cargo version | Notes |
|---|---|---|
| 202646a | 0.2.0 | Rust + Skia rewrite, OmniDoc only |
| 202646b | 0.3.0 (roadmap alpha-3) | OmniCel, OmniShow, launcher, 12 OmniDoc features, DOC import |
| 202646c | 0.3.1 | Google Material Symbols icons in all apps |
| 202646d | 0.3.2 | macOS CI fix: winit 0.30, rfd 0.15 (drops objc2 0.4.1), pinned CI toolchain |

The id is the constant `BUILD_ID` in `crates/omni-common/src/lib.rs`, shown in window titles and `.omd/.omc/.oms`
manifests. Cargo needs semver, so each build id also has a fixed crate version (table above); `Cargo.toml` and `BUILD_ID` must be bumped together.
| 202646e | 0.4.0 | PowerPoint-style OmniShow ribbon, DOCX/DOC compatibility, HWP import, OmniCel clipboard/undo, page guides |
| 202647a | 0.5.0 | OmniDoc tables/pictures/formatting/spell check, OmniShow pictures/presenter view/PNG export, OmniCel formats/charts/ribbon |

Note: 0.5.0 was first labelled 202646f by mistake. It was finished on Monday 16 November 2026, which is ISO week 47, so its build id is **202647a** (the next build of week 47 is 202647b).
