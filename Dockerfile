# syntax=docker/dockerfile:1.6
# =============================================================================
# SuperNovae CLI (spn) — Static Multi-Platform Docker Image
# =============================================================================
# This Dockerfile uses pre-built STATIC binaries (musl) from GitHub Actions.
# No runtime dependencies needed - uses scratch base for minimal image (~5MB).
#
# Build context structure (created by CI):
#   docker-context/
#   ├── amd64/spn    (x86_64-unknown-linux-musl binary)
#   └── arm64/spn    (aarch64-unknown-linux-musl binary)
#
# Usage:
#   docker run --rm ghcr.io/supernovae-st/spn:latest --version
#   docker run --rm -v $(pwd):/workspace ghcr.io/supernovae-st/spn:latest list
#
# Note: This build uses --no-default-features --features docker which disables
# OS keychain support. Use environment variables for secrets:
#   -e ANTHROPIC_API_KEY="sk-ant-..."
# =============================================================================

FROM scratch

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

# Copy pre-built static binary for target architecture
# CI creates: docker-context/{amd64,arm64}/spn
COPY ${TARGETARCH}/spn /spn

# Working directory for mounted projects
WORKDIR /workspace

# Default entrypoint
ENTRYPOINT ["/spn"]
CMD ["--help"]
