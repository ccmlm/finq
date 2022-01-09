all: release_musl

lint:
	cargo clippy

build:
	cargo build

release:
	cargo build --release

release_musl:
	cargo build --release --target=x86_64-unknown-linux-musl

fmt:
	cargo fmt

clean:
	git clean -fdx
	cargo clean

update:
	cargo update

run:
	cargo run --release
