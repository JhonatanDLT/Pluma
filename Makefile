.PHONY: install dev build check test fmt clippy release bundle-onnx fetch-models clean

install:
	pnpm install

dev:
	pnpm tauri dev

build:
	pnpm tauri build

check:
	pnpm check
	cargo check --manifest-path src-tauri/Cargo.toml --all-targets

test:
	pnpm check
	cargo test --manifest-path src-tauri/Cargo.toml --lib

fmt:
	cargo fmt --manifest-path src-tauri/Cargo.toml

clippy:
	cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings

release:
	./scripts/bundle-onnx.sh
	pnpm tauri build

bundle-onnx:
	./scripts/bundle-onnx.sh

fetch-models:
	./scripts/fetch-models.sh

clean:
	cargo clean --manifest-path src-tauri/Cargo.toml
	rm -rf build .svelte-kit
