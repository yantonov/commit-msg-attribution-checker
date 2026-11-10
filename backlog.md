## Active
- [ ] Config model and schema validation #config-model <- [app-placeholder]
- [ ] Config resolution order #config-resolution <- [app-placeholder]
- [ ] Default pattern registry #default-patterns <- [config-model]
- [ ] Message parsing: comment lines and trailers #msg-parsing <- [app-placeholder]
- [ ] Match engine with trailer gate #match-engine <- [config-model, msg-parsing]
- [ ] config defaults #cmd-config-defaults <- [default-patterns]
- [ ] config create #cmd-config-create <- [cmd-config-defaults]
- [ ] config show #cmd-config-show <- [config-resolution]
- [ ] Identity targets via git var #identity-targets <- [match-engine]
- [ ] verify command: human output and exit codes #cmd-verify <- [config-resolution, match-engine]
- [ ] End-to-end acceptance suite #acceptance <- [cmd-config-create, cmd-verify]
- [ ] verify --json output #verify-json <- [cmd-verify]
- [ ] Hook integration and smoke test #hook-integration <- [cmd-verify]
- [ ] Dogfood the checker on this repo #dogfood <- [hook-integration]

## Closed
- [x] App placeholder: cargo scaffold #app-placeholder
- [x] Minimal harness: AGENTS.md and Makefile #minimal-harness <- [app-placeholder]
