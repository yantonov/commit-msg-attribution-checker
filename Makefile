# Public targets are what a contributor needs day to day; internal targets are
# the individual steps `check` calls, callable on their own for a quick loop.

.PHONY: help help-all check install fmt lint test build build-release

.DEFAULT_GOAL := help

help:
	@echo "Usage: make [target]"
	@echo ""
	@echo "  check   - run fmt, clippy and tests, then print one verdict"
	@echo "  install - release build, copied to $$HOME/.local/bin"
	@echo ""
	@echo "Run 'make help-all' for the full list, including internal targets."

help-all:
	@echo "Public targets:"
	@echo "  check   - run fmt, clippy and tests, then print one verdict"
	@echo "  install - release build, copied to $$HOME/.local/bin"
	@echo ""
	@echo "Internal targets (the steps of check, plus the build targets):"
	@echo "  fmt           - fail if formatting differs (cargo fmt --check)"
	@echo "  lint          - clippy with warnings as errors"
	@echo "  test          - cargo test, including the harness placeholder guard"
	@echo "  build         - cargo build"
	@echo "  build-release - cargo build --release, the artifact install copies"

check: fmt lint test
	@echo "ALL CHECKS PASSED"

fmt:
	cargo fmt --check

lint:
	cargo clippy --all-targets -- -D warnings

test:
	cargo test

build:
	cargo build

build-release:
	cargo build --release

# install is a prerequisite of nothing and check never calls it, so verification
# stays free of writes outside target/. Being a prerequisite rather than a recipe
# line is what keeps a broken build from replacing a working binary on PATH:
# make stops before the copy runs.
install: build-release
	@mkdir -p "$$HOME/.local/bin"
	@cp target/release/commit-msg-attribution-checker "$$HOME/.local/bin/commit-msg-attribution-checker"
	@echo "Installed: $$HOME/.local/bin/commit-msg-attribution-checker"
