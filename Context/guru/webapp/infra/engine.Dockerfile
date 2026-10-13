FROM rust:1.97.1-slim-bookworm AS build
WORKDIR /build
ENV CARGO_BUILD_JOBS=2
ENV RUSTFLAGS="-C codegen-units=2"
COPY backend/Cargo.toml backend/Cargo.lock ./
COPY backend/src ./src
RUN cargo build --release --locked
FROM debian:bookworm-slim
COPY --from=build /build/target/release/guru /usr/local/bin/guru
USER 65534:65534
ENTRYPOINT ["guru"]
