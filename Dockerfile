FROM clux/muslrust:stable AS chef
RUN cargo install cargo-chef
RUN cargo install sqlx-cli --no-default-features --features sqlite
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
COPY . .
# sqlx needs database info to typecheck properly.
ENV DATABASE_URL=sqlite://./data/hubble.db
RUN mkdir ./data && sqlx database create && sqlx migrate run
RUN cargo build --release --target x86_64-unknown-linux-musl --bin hubble
RUN strip target/x86_64-unknown-linux-musl/release/hubble
RUN mkdir -p /data

FROM gcr.io/distroless/static:nonroot AS runtime
WORKDIR /app
ENV DATABASE_URL=sqlite:///data/hubble.db?mode=rwc
ENV DATA_DIR=/data
COPY --from=builder --chown=nonroot:nonroot /data /data
COPY --from=builder --chown=nonroot:nonroot /app/target/x86_64-unknown-linux-musl/release/hubble /usr/local/bin/hubble
USER nonroot
ENTRYPOINT ["/usr/local/bin/hubble"]