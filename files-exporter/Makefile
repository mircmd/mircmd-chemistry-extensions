INSTALL_DIR ?= $(HOME)/.config/mircmd/extensions/mircmd/chemistry-files-exporter

.PHONY: init build test install clean
init:
	rustup target add wasm32-wasip2

build:
	cargo build --target wasm32-wasip2 --release

test:
	cargo test

install: build
	mkdir -p "$(INSTALL_DIR)"
	cp manifest.yaml "$(INSTALL_DIR)/"
	cp target/wasm32-wasip2/release/files_exporter_extension.wasm "$(INSTALL_DIR)/extension.wasm"

clean:
	cargo clean
