all: lint

lint:
	cargo clippy

build:
	cargo build

release:
	cargo build --release

fmt:
	cargo fmt

clean:
	git clean -fdx
	cargo clean

update:
	cargo update

run:
	cargo run --release
