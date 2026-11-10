---
status: open
priority: 1
deps: [app-placeholder]
attachments: []
created: 2026-11-10T06:39:01Z
updated: 2026-11-10T06:39:11Z
---
# Message parsing: comment lines and trailers

Turn a message file into the values that patterns test, per spec §5.1 and §5.2.

Scope:
- Read the file, split into lines, keep 1-based line numbers that count every line including comments.
- Drop a line whose first byte is `#`. Do not drop `  # …` with leading spaces, which is git's own rule.
- Trailer parse: `^([A-Za-z0-9][A-Za-z0-9-]*):[ \t]*(.+)$`, key compared without case.
- Expose, per line: raw text, whether it parses as a trailer and with which key and value, and whether a trailer pattern claimed it.
- Subject is the first non-comment line.
- Handle CRLF without leaving a stray `\r` in the tested value.

Done when:
- Tests cover: first-byte `#` dropped, indented `#` kept, `body#notcomment` kept.
- Tests cover trailer parse success and failure, and key case-insensitivity.
- An empty file, or one with comments only, yields no lines to match (acceptance criterion 8).
- Line numbers in results match the file, including comment lines.
