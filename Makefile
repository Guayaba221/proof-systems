# Variables
# Known coverage limitations and issues:
# - https://github.com/rust-lang/rust/issues/79417
# - https://github.com/nextest-rs/nextest/issues/16
# FIXME: Update or remove the `codecov.yml` file to enable the `patch` coverage report and the corresponding PR check,
#        once situation with the Rust's Doctests will be improved.
COVERAGE_ENV = CARGO_INCREMENTAL=0 RUSTFLAGS='-Cinstrument-coverage' RUSTDOCFLAGS="-Cinstrument-coverage" LLVM_PROFILE_FILE=$(shell pwd)/target/profraw/cargo-test-%p-%m.profraw
# FIXME: In latest 0.8.19+ -t CLI argument can accept comma separated list of custom output types, hence, no need in double invocation
GRCOV_CALL = grcov ./target/profraw --binary-path ./target/release/deps/ -s . --branch --ignore-not-existing --ignore "**/tests/**"

# Default target
all: release

setup:
		@echo ""
		@echo "Syncing the Git submodules."
		@echo ""
		git submodule sync
		git submodule update --init --recursive
		@echo ""
		@echo "Git submodules synced."
		@echo ""

# Install test dependencies
# https://nexte.st/book/pre-built-binaries.html#using-nextest-in-github-actions
# FIXME: update to 0.9.68 when we get rid of 1.71 and 1.72.
# FIXME: latest 0.8.19+ requires rustc 1.74+
install-test-deps:
		@echo ""
		@echo "Installing the test dependencies."
		@echo ""
		rustup component add llvm-tools-preview
		cargo install cargo-nextest@=0.9.67 --locked
		cargo install grcov@=0.8.13 --locked
		@echo ""
		@echo "Test dependencies installed."
		@echo ""

# Clean the project
clean:
		cargo clean

# Build the project
build:
		cargo build --all-targets --all-features

# Build the project in release mode
release:
		cargo build --release --all-targets --all-features

# Test the project's docs comments
test-doc:
		cargo test --all-features --release --doc

test-doc-with-coverage:
		$(COVERAGE_ENV) $(MAKE) test-doc

# Test the project with non-heavy tests and using native cargo test runner
test:
		cargo test --all-features --release $(CARGO_EXTRA_ARGS) -- --nocapture --skip heavy $(BIN_EXTRA_ARGS)

test-with-coverage:
		$(COVERAGE_ENV) CARGO_EXTRA_ARGS="$(CARGO_EXTRA_ARGS)" BIN_EXTRA_ARGS="$(BIN_EXTRA_ARGS)" $(MAKE) test

# Test the project with heavy tests and using native cargo test runner
test-heavy:
		cargo test --all-features --release $(CARGO_EXTRA_ARGS) -- --nocapture heavy $(BIN_EXTRA_ARGS)

test-heavy-with-coverage:
		$(COVERAGE_ENV) CARGO_EXTRA_ARGS="$(CARGO_EXTRA_ARGS)" BIN_EXTRA_ARGS="$(BIN_EXTRA_ARGS)" $(MAKE) test-heavy

# Test the project with all tests and using native cargo test runner
test-all:
		cargo test --all-features --release $(CARGO_EXTRA_ARGS) -- --nocapture $(BIN_EXTRA_ARGS)

test-all-with-coverage:
		$(COVERAGE_ENV) CARGO_EXTRA_ARGS="$(CARGO_EXTRA_ARGS)" BIN_EXTRA_ARGS="$(BIN_EXTRA_ARGS)" $(MAKE) test-all

# Test the project with non-heavy tests and using nextest test runner
nextest:
		cargo nextest run --all-features --release --profile ci -E "not test(heavy)" $(BIN_EXTRA_ARGS)

nextest-with-coverage:
		$(COVERAGE_ENV) BIN_EXTRA_ARGS="$(BIN_EXTRA_ARGS)" $(MAKE) nextest

# Test the project with heavy tests and using nextest test runner
nextest-heavy:
		cargo nextest run --all-features --release --profile ci -E "test(heavy)" $(BIN_EXTRA_ARGS)

nextest-heavy-with-coverage:
		$(COVERAGE_ENV) BIN_EXTRA_ARGS="$(BIN_EXTRA_ARGS)" $(MAKE) nextest-heavy

# Test the project with all tests and using nextest test runner
nextest-all:
		cargo nextest run --all-features --release --profile ci $(BIN_EXTRA_ARGS)

nextest-all-with-coverage:
		$(COVERAGE_ENV) BIN_EXTRA_ARGS="$(BIN_EXTRA_ARGS)" $(MAKE) nextest-all

# Format the code
format:
		cargo +nightly fmt -- --check

# Lint the code
lint:
		cargo clippy --all-features --all-targets --tests $(CARGO_EXTRA_ARGS) -- -W clippy::all -D warnings

generate-test-coverage-report:
		@echo ""
		@echo "Generating the test coverage report."
		@echo ""
		mkdir -p ./target/coverage
		GRCOV_OUTPUT_TYPE=html GRCOV_OUTPUT_PATH=./target/coverage
		$(eval GRCOV_HTML_CMD=$(GRCOV_CALL) -t html -o ./target/coverage)
		$(GRCOV_HTML_CMD)
		$(eval GRCOV_LCOV_CMD=$(GRCOV_CALL) -t lcov -o ./target/coverage/lcov.info)
		$(GRCOV_LCOV_CMD)
		@echo ""
		@echo "The test coverage report is available at: ./target/coverage"
		@echo ""

generate-doc:
		@echo ""
		@echo "Generating the documentation."
		@echo ""
		RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps
		@echo ""
		@echo "The documentation is available at: ./target/doc"
		@echo ""

o1vm-test:
		@mips-linux-gnu-as -o ./o1vm/resources/programs/hello_world.bin ./o1vm/resources/programs/hello_world.mips

.PHONY: all setup install-test-deps clean build release test-doc test-doc-with-coverage test test-with-coverage test-heavy test-heavy-with-coverage test-all test-all-with-coverage nextest nextest-with-coverage nextest-heavy nextest-heavy-with-coverage nextest-all nextest-all-with-coverage format lint generate-test-coverage-report generate-doc
