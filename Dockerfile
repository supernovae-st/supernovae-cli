# syntax=docker/dockerfile:1.6
# =============================================================================
# SuperNovae CLI (spn) — Multi-Platform Docker Image
# =============================================================================
# This Dockerfile uses pre-built binaries from GitHub Actions.
# It does NOT compile Rust — binaries are injected at build time.
#
# Build context structure (created by CI):
#   docker-context/
#   ├── amd64/spn    (x86_64-unknown-linux-gnu binary)
#   └── arm64/spn    (aarch64-unknown-linux-gnu binary)
#
# Usage:
#   docker run --rm ghcr.io/supernovae-st/spn:latest --version
#   docker run --rm -v $(pwd):/workspace ghcr.io/supernovae-st/spn:latest list
# =============================================================================

FROM gcr.io/distroless/cc-debian12:nonroot

# OCI Labels (https://github.com/opencontainers/image-spec/blob/main/annotations.md)
LABEL org.opencontainers.image.source="https://github.com/supernovae-st/supernovae-cli"
LABEL org.opencontainers.image.description="SuperNovae CLI — Unified package manager for AI workflows"
LABEL org.opencontainers.image.licenses="AGPL-3.0-or-later"
LABEL org.opencontainers.image.vendor="SuperNovae Studio"
LABEL org.opencontainers.image.title="spn"
LABEL org.opencontainers.image.url="https://supernovae.studio"
LABEL org.opencontainers.image.documentation="https://github.com/supernovae-st/supernovae-cli#readme"

# Build arguments (set by GitHub Actions)
ARG TARGETARCH
ARG VERSION=dev

LABEL org.opencontainers.image.version="${VERSION}"

# Copy pre-built binary for target architecture
# CI creates: docker-context/{amd64,arm64}/spn
COPY --chown=nonroot:nonroot ${TARGETARCH}/spn /usr/local/bin/spn

# Run as non-root user (UID 65532 in distroless)
USER nonroot:nonroot

# Working directory for mounted projects
WORKDIR /workspace

# Default entrypoint
ENTRYPOINT ["/usr/local/bin/spn"]
CMD ["--help"]
