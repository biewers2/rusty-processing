FROM rust:latest as builder

WORKDIR /usr/src/rusty-processing
COPY . .

RUN apt-get update && \
    apt-get install -y protobuf-compiler

# RUN cargo install --bin cli --path cli && \
#    cargo install --bin temporal-worker --path temporal-worker
RUN cargo build --release

CMD ["./target/release/cli"]