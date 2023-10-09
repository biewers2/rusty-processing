FROM rust:latest AS dependencies

RUN apt-get -y update && \
    \
    wget -O wkhtmltox.deb https://github.com/wkhtmltopdf/packaging/releases/download/0.12.6.1-2/wkhtmltox_0.12.6.1-2.bullseye_amd64.deb && \
    wget -O libssl1.1.deb http://archive.ubuntu.com/ubuntu/pool/main/o/openssl/libssl1.1_1.1.0g-2ubuntu4_amd64.deb && \
    \
    apt-get install -y \
        protobuf-compiler \
        ghostscript \
        pkg-config \
        ./libssl1.1.deb \
        ./wkhtmltox.deb && \
    \
    rm ./libssl1.1.deb ./wkhtmltox.deb


FROM dependencies AS builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM dependencies

COPY --from=builder /app/target/release/cli /usr/local/bin/cli
COPY --from=builder /app/target/release/temporal-worker /usr/local/bin/temporal-worker
CMD ["temporal-worker"]
