## Active

## Closed
- [x] App placeholder: cargo scaffold #app-placeholder
- [x] Minimal harness: AGENTS.md and Makefile #minimal-harness <- [app-placeholder]
- [x] Config model and schema validation #config-model <- [app-placeholder]
- [x] Config resolution order #config-resolution <- [app-placeholder]
- [x] Default pattern registry #default-patterns <- [config-model]
- [x] Message parsing: comment lines and trailers #msg-parsing <- [app-placeholder]
- [x] Match engine with trailer gate #match-engine <- [config-model, msg-parsing]
- [x] config defaults #cmd-config-defaults <- [default-patterns]
- [x] config create #cmd-config-create <- [cmd-config-defaults]
- [x] config show #cmd-config-show <- [config-resolution]
- [x] Identity targets via git var #identity-targets <- [match-engine]
- [x] verify command: human output and exit codes #cmd-verify <- [config-resolution, match-engine]
- [x] End-to-end acceptance suite #acceptance <- [cmd-config-create, cmd-verify]
- [x] Decide whether aider.author-name should match a rendered ident #ehpbco
- [x] Make install target: release artifact then install to ~/.local/bin #gizbn3
- [x] Document installing the hook through krok #tvn67
- [x] Add a Motivation section to the README #hss5iy
- [x] README: title the tool, fix the summary line, shorten Motivation #qobkz3
- [x] README: move Motivation below the table of contents #lde4d
- [x] Add Apache-2.0 license #kzrww2
- [x] verify --json output #verify-json <- [cmd-verify]
- [x] Hook integration and smoke test #hook-integration <- [cmd-verify]
- [x] Dogfood the checker on this repo #dogfood <- [hook-integration]
