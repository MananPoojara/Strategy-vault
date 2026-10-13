FROM node:24.11.1-bookworm-slim AS frontend-build
WORKDIR /frontend
COPY frontend/react/package.json frontend/react/package-lock.json ./react/
RUN --mount=type=secret,id=build_ca --mount=type=cache,id=vault-frontend-npm,target=/root/.npm if [ -s /run/secrets/build_ca ]; then export NODE_EXTRA_CA_CERTS=/run/secrets/build_ca; fi; npm --prefix react ci --no-audit --no-fund
COPY frontend ./
RUN npm --prefix react run build

FROM rust:1.97.1-slim-bookworm AS build
WORKDIR /build
ENV CARGO_BUILD_JOBS=2
COPY webapp/gateway/Cargo.toml webapp/gateway/Cargo.lock ./
COPY webapp/gateway/src ./src
RUN --mount=type=secret,id=build_ca --mount=type=cache,id=vault-gateway-target,target=/build/target if [ -s /run/secrets/build_ca ]; then cp /run/secrets/build_ca /usr/local/share/ca-certificates/build.crt; update-ca-certificates; fi; cargo build --release --locked && cp target/release/vault-webapp /usr/local/bin/vault-webapp
FROM debian:bookworm-slim
WORKDIR /app
COPY --from=build /usr/local/bin/vault-webapp /usr/local/bin/vault-webapp
COPY --from=frontend-build --chown=65534:65534 /frontend/react/dist ./frontend
COPY --chown=65534:65534 webapp/public ./public
USER 65534:65534
CMD ["vault-webapp"]
