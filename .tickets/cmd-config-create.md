---
title: "config create"
status: closed
priority: 2
deps: [cmd-config-defaults]
attachments: []
created: 2026-11-10T06:38:57Z
updated: 2026-11-10T07:17:14Z
closed-at: 2026-11-10T07:17:14Z
---
# config create

Write the built-in default config to the resolved path and print the absolute path.

Scope:
- Target path comes from the resolution order in `config-resolution`, so `--config <path>` and the env var both work and the test never touches a real home directory.
- Creates parent directories.
- Refuses when the target exists: exit 2, print the path, leave the file unchanged. There is no `--force`, per spec §6.
- Prints the absolute path of the file it wrote, resolved against the working directory when the target was relative.
- Writes through a temporary file in the same directory and renames, so an interrupted run cannot leave a half-written config that a later `verify` reads.

Acceptance criterion owned here: 10.

Done when:
- Criterion 10 passes: a second run exits 2, prints the path, and leaves the file byte-identical.
- A fresh path gets the file and the command prints that absolute path.
- The written file loads through the config model with no error.
- A test covers a target whose parent directories do not exist.
- A test covers a relative `--config` path and asserts the printed path is absolute.
