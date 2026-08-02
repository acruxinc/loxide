.PHONY: check test build example clean clippy fmt fmt-check doc

## Run formatting check, clippy, and tests (what CI runs)
check: fmt-check clippy test doc

## Run all tests (unit + doc tests)
test:
	cargo test --verbose

## Build the library
build:
	cargo build

## Run the demo example
example:
	cargo run --example demo

## Format the code
fmt:
	cargo fmt

## Verify formatting without modifying files
fmt-check:
	cargo fmt -- --check

## Run clippy lints, denying warnings
clippy:
	cargo clippy --all-targets -- -D warnings

## Build the documentation, denying warnings
doc:
	RUSTDOCFLAGS="-D warnings" cargo doc --no-deps

## Remove build artifacts
clean:
	cargo clean
