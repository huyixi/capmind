CAP_PKG ?= cap-cli
CAP_PATH ?= apps/cli
CAP_BIN ?= cap
BIN_DIR ?= $(HOME)/.cargo/bin

.PHONY: help cap cap-build cap-install cap-link cap-release cap-release-link cap-run

help:
	@echo "Targets:"
	@echo "  make cap            Build debug binary"
	@echo "  make cap-link       Symlink target/debug/cap to ~/.cargo/bin/cap"
	@echo "  make cap-install    Install CLI to ~/.cargo/bin via cargo install"
	@echo "  make cap-release    Build release binary"
	@echo "  make cap-release-link Symlink target/release/cap to ~/.cargo/bin/cap"
	@echo "  make cap-run ARGS='--help'  Run CLI with custom args"

cap: cap-build

cap-build:
	cargo build -p $(CAP_PKG)

cap-install:
	cargo install --path $(CAP_PATH) --force

cap-link: cap-build
	mkdir -p $(BIN_DIR)
	ln -sf "$(PWD)/target/debug/$(CAP_BIN)" "$(BIN_DIR)/$(CAP_BIN)"

cap-release:
	cargo build -p $(CAP_PKG) --release

cap-release-link: cap-release
	mkdir -p $(BIN_DIR)
	ln -sf "$(PWD)/target/release/$(CAP_BIN)" "$(BIN_DIR)/$(CAP_BIN)"

cap-run:
	cargo run -p $(CAP_PKG) -- $(ARGS)
