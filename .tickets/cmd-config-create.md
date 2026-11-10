---
status: open
priority: 2
deps: [cmd-config-defaults]
attachments: []
created: 2026-11-10T06:38:57Z
updated: 2026-11-10T06:39:11Z
---
# config create

Write the built-in default config to the resolved path and print the absolute path.

Scope:
- Creates parent directories.
- Refuses when the target exists: exit 2, print the path, leave the file unchanged. No `--force`, per spec §6.
- Honors `--config <path>` so the target path is testable without touching a real home directory.
- Prints the absolute path of the file it wrote.

Done when:
- A fresh path gets the file, and the command prints that absolute path.
- A second run exits 2, prints the path, and leaves the file byte-identical (acceptance criterion 10).
- The written file loads through the config model with no error.
- A test covers a path whose parent directories do not exist.
