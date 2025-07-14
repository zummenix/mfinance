FROM rust:alpine as builder
RUN apk add musl-dev
WORKDIR /app
COPY Cargo.toml Cargo.lock .
COPY src/ src/
RUN cargo build

FROM alpine:latest
COPY --from=builder /app/target/debug/mfinance /usr/local/bin/
EXPOSE 3000
ENTRYPOINT ["mfinance", "server", "--host", "0.0.0.0", "--port", "3000", "/data"]
