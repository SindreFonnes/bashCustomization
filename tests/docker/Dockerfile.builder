FROM rust:1-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /build
COPY rust/Cargo.toml rust/Cargo.lock ./
COPY rust/src/ ./src/
RUN cargo build --release

FROM scratch
COPY --from=builder /build/target/release/bashc /bashc
