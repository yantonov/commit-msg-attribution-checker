# Public targets are what a contributor needs day to day; internal targets are
# the individual steps `check` calls, callable on their own for a quick loop.

.PHONY: help help-all check fmt lint test build

.DEFAULT_GOAL := help

help:
	@echo "Usage: make [target]"
	@echo ""
	@echo "  check   - run fmt, clippy and tests, then print one verdict"
	@echo ""
	@echo "Run 'make help-all' for the full list, including internal targets."

help-all:
	@echo "Public targets:"
	@echo "  check   - run fmt, clippy and tests, then print one verdict"
	@echo ""
	@echo "Internal targets (the steps of check, plus build):"
	@echo "  fmt     - fail if formatting differs (cargo fmt --check)"
	@echo "  lint    - clippy with warnings as errors"
	@echo "  test    - cargo test, including the harness placeholder guard"
	@echo "  build   - cargo build"

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
