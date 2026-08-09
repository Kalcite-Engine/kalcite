test:
    cargo test --workspace

check-example:
    cargo run -p kalcite-cli -- check examples/pong/src/main.klc

lint-example:
    cargo run -p kalcite-cli -- lint examples/pong/src/main.klc

build-example:
    cargo run -p kalcite-cli -- build examples/pong/src/main.klc --target numworks

numworks:
    cargo build -p kalcite-platform-numworks --target thumbv7em-none-eabihf --release
