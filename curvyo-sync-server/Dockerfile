# The Curvyo relay as a container (ADR 0031 decision 11, specs/0092 criterion 36): a static musl
# binary in a distroless nonroot image, `/data` as the volume, port 8787. Built by hand for the
# demo (no CI image build in slice 1). The build context is the repository root:
#
#   docker build -f curvyo-sync-server/Dockerfile -t curvyo-sync-server .
#
# `Dockerfile.dockerignore` next to this file keeps `target/`, `frontend/` and `.git` out of the
# context (BuildKit reads it because of its name).

FROM rust:1.99.0-bookworm AS build
RUN apt-get update \
    && apt-get install -y --no-install-recommends musl-tools \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /src
COPY . .
# The pinned toolchain of `rust-toolchain.toml` (1.99.0) is the image's own; only the musl target
# is added.
RUN rustup target add x86_64-unknown-linux-musl \
    && cargo build --release --locked -p curvyo-sync-server --target x86_64-unknown-linux-musl \
    && mkdir /data
# The volume's mount point belongs to the image's nonroot user (uid 65532), so a fresh named
# volume is writable without a root step.

FROM gcr.io/distroless/static-debian12:nonroot
COPY --from=build /src/target/x86_64-unknown-linux-musl/release/curvyo-sync-server /curvyo-sync-server
COPY --from=build --chown=65532:65532 /data /data
USER 65532:65532
ENV CURVYO_SYNC_LISTEN=0.0.0.0:8787 \
    CURVYO_SYNC_DATA=/data
VOLUME ["/data"]
EXPOSE 8787
ENTRYPOINT ["/curvyo-sync-server"]
