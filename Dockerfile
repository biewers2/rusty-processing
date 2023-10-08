FROM rust:latest AS dependencies

RUN apt-get -y update && \
    apt-get install -y \
        protobuf-compiler

FROM dependencies AS builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM dependencies

COPY --from=builder /app/target/release/temporal-worker /usr/local/bin/temporal-worker
CMD ["temporal-worker"]
