---
title: "Add Apache-2.0 license"
status: closed
priority: 2
deps: []
attachments: []
created: 2026-11-10T08:45:34Z
updated: 2026-11-10T08:45:54Z
closed-at: 2026-11-10T08:45:54Z
---
# Add Apache-2.0 license

Ship the Apache License 2.0 text as LICENSE, copyright 2026 Yury Antonov. Declare the license in Cargo.toml (license = "Apache-2.0") and point to it from README. Done when: cargo build still passes, LICENSE is byte-identical to the canonical Apache-2.0 text, and make check is green.
