.PHONY: build test run-api run-cli run-web dev clean check install

build:
	cargo build --workspace

test:
	cargo test --workspace

run-api:
	cargo run --bin nous-api

run-cli:
	cargo run --bin nous -- status

run-web:
	cd apps/web && npm run dev

dev:
	@echo "Starting API server and web app..."
	cargo run --bin nous-api &
	cd apps/web && npm run dev

clean:
	cargo clean

check:
	cargo clippy --workspace -- -D warnings
	cargo fmt --check

install:
	cargo install --path crates/nous-cli
	cargo install --path crates/nous-api
