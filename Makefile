.PHONY: lint build install fmt clippy check-loc test sync-packaging check-packaging release

CARGO ?= cargo

lint: fmt clippy check-loc

fmt:
	$(CARGO) fmt --all -- --check

clippy:
	$(CARGO) clippy --workspace --all-targets -- -D warnings

check-loc:
	$(CARGO) run -p release --bin check-loc

build:
	$(CARGO) build --workspace

test:
	$(CARGO) test --workspace

install:
	$(CARGO) install --path . --locked

sync-packaging:
	$(CARGO) run -p release --bin sync-packaging

check-packaging:
	$(CARGO) run -p release --bin sync-packaging -- --check

release:
	$(CARGO) run -p release
