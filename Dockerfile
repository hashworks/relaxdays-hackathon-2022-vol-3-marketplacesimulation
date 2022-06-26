FROM rust:1.61 AS build

WORKDIR /usr/src
COPY Cargo.lock Cargo.toml ./
COPY src ./src
RUN cargo build --release

# hadolint ignore=DL3007
FROM archlinux:latest AS runtime

COPY --from=build /usr/src/target/release/marketplacesimulation /usr/bin
ENTRYPOINT ["/usr/bin/marketplacesimulation"]
