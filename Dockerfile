FROM rust:latest AS dependencies

RUN apt-get -y update && \
    apt-get install -y --no-install-recommends \
        protobuf-compiler \
        openjdk-11-jdk-headless

ENV JAVA_HOME /usr/local/openjdk-11
ENV PATH $JAVA_HOME/bin:$PATH

FROM dependencies AS builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM dependencies

COPY --from=builder /app/target/release/temporal-worker /usr/local/bin/temporal-worker
CMD ["temporal-worker"]
