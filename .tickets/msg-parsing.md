---
status: open
priority: 1
deps: [app-placeholder]
attachments: []
created: 2026-11-10T06:39:01Z
updated: 2026-11-10T06:40:19Z
---
# Message parsing: comment lines and trailers

Turn a message file into the values that patterns test, per spec §5.1 and §5.2.

Scope:
- Read the file as bytes and split into lines. Line numbers start at 1 and count every line of the file, including comment lines.
- Drop a line whose first byte is `#`. Keep `  # …` with leading spaces and keep `body#notcomment`, because spec §5.1 follows git: only a `#` in the first column is a comment.
- Trailer parse: `^([A-Za-z0-9][A-Za-z0-9-]*):[ \t]*(.+)$`. The key comparison is case-insensitive; the value keeps its original case and spacing after the separator.
- Expose per line: the raw text, whether it parses as a trailer and with which key and value, and whether a trailer pattern claimed it. The last flag is set by the match engine, not here.
- Subject is the first non-comment line. An empty message, or one with comments only, produces no subject and no lines to match.
- Strip a trailing `\r` so a CRLF file does not leave `\r` inside the value a pattern tests. This is a parse concern, not a pattern concern.
- Invalid UTF-8 is an I/O error, exit 2, naming the file.

Done when:
- Tests cover: first-byte `#` dropped, indented `#` kept, `body#notcomment` kept.
- Tests cover trailer parse success and failure, and key case-insensitivity (`CO-AUTHORED-BY` matches `co-authored-by`).
- An empty file, or one with comments only, yields no lines to match (acceptance criterion 8).
- Line numbers in results match the file, including comment lines.
- A CRLF fixture produces values with no `\r`.
