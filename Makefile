TARGET := x86_64-pc-windows-gnu

.PHONY: all test lint build build-console verify package check-win clean

all: test build verify

test:            ## portable unit tests, run natively on the Linux host
	cargo test --lib

lint:
	cargo fmt --all -- --check
	cargo clippy --lib --all-targets -- -D warnings
	cargo clippy --target $(TARGET) --release -- -D warnings

check-win:       ## type-check the Windows-only code without linking
	cargo check --target $(TARGET)

build:           ## dist/berry_unlock.exe (GUI subsystem, for launchers)
	./scripts/build.sh

build-console:   ## same, with a console subsystem for terminal debugging
	./scripts/build.sh --console

verify:
	./scripts/verify.sh

package:
	./scripts/package.sh

clean:
	cargo clean
	rm -rf dist
