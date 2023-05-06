FROM lukemathwalker/cargo-chef:latest AS chef
WORKDIR /view

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
ARG COMPONENT

COPY --from=planner /view/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --bin ${COMPONENT}

# We do not need the Rust toolchain to run the binary!
FROM debian:bullseye-slim AS runtime

ARG COMPONENT

ENV USER=${COMPONENT}
ENV UID=10001

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"

WORKDIR /view
COPY --from=builder /view/target/release/${COMPONENT} /usr/local/bin/view

USER ${USER}:${USER}
ENTRYPOINT ["/usr/local/bin/view"]
