# Docker Distribution Plan — spn v0.12.0

> **Status**: Draft
> **Author**: Claude + Thibaut
> **Date**: 2026-03-05
> **Target**: v0.12.0

---

## Executive Summary

Add Docker distribution to `spn` CLI via GitHub Container Registry (ghcr.io), providing a 4th distribution channel alongside Homebrew, crates.io, and GitHub Releases.

```
╔═══════════════════════════════════════════════════════════════════════════════╗
║  DISTRIBUTION CHANNELS (v0.12.0)                                              ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║  PRIMARY (End Users)                                                          ║
║  ├── Homebrew       brew install supernovae-st/tap/spn                        ║
║  └── Cargo          cargo install spn-cli                                     ║
║                                                                               ║
║  SECONDARY (CI/CD, Containers)                                                ║
║  ├── Docker         docker run ghcr.io/supernovae-st/spn:latest              ║
║  └── Binaries       GitHub Releases (tar.gz + sha256)                        ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

---

## 1. Architecture Decision

### Recommended Approach: Reuse Pre-Built Binaries

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  DOCKER BUILD PIPELINE (Optimized)                                              │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  EXISTING: build job (4 targets, ~12 min)                                       │
│  ├── aarch64-apple-darwin      → tar.gz artifact                               │
│  ├── x86_64-apple-darwin       → tar.gz artifact                               │
│  ├── aarch64-unknown-linux-gnu → tar.gz artifact  ──┐                          │
│  └── x86_64-unknown-linux-gnu  → tar.gz artifact  ──┼── Docker uses these      │
│                                                      │                          │
│  NEW: docker-publish job (needs: build, ~2 min)      │                          │
│  ├── Download Linux artifacts ◄──────────────────────┘                          │
│  ├── Extract binaries (amd64 + arm64)                                           │
│  ├── Build multi-platform image (no cargo!)                                     │
│  └── Push to ghcr.io/supernovae-st/spn                                         │
│                                                                                 │
│  BENEFIT: Zero duplicate compilation, +2 min total                              │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Why Not Build Inside Docker?

| Approach | Build Time | Consistency | Complexity |
|----------|------------|-------------|------------|
| **Reuse artifacts** | +2 min | ✅ Same binary as releases | Low |
| Build in Docker | +12 min | ⚠️ Different binary | High (cross-compile) |

**Decision**: Reuse existing Linux artifacts from the build job.

---

## 2. Image Strategy

### Single Multi-Arch Image

```
ghcr.io/supernovae-st/spn:latest
ghcr.io/supernovae-st/spn:0.12.0
ghcr.io/supernovae-st/spn:0.12
ghcr.io/supernovae-st/spn:0
ghcr.io/supernovae-st/spn:sha-abc1234
```

### Base Image Selection

| Option | Size | Security | Shell | D-Bus | Recommendation |
|--------|------|----------|-------|-------|----------------|
| `scratch` | 0 MB | ✅ Best | ❌ | ❌ | Too minimal |
| `gcr.io/distroless/static` | 2 MB | ✅ Best | ❌ | ❌ | ⚠️ No debug |
| `gcr.io/distroless/cc` | 10 MB | ✅ Good | ❌ | ❌ | **✅ RECOMMENDED** |
| `alpine:3.21` | 7 MB | ✅ Good | ✅ | ⚠️ | Alternative |
| `debian:bookworm-slim` | 80 MB | ⚠️ CVEs | ✅ | ✅ | Too large |

**Decision**: `gcr.io/distroless/cc-debian12` — minimal, secure, includes libc for glibc binaries.

### Limitations (Documented)

```
╔═══════════════════════════════════════════════════════════════════════════════╗
║  DOCKER LIMITATIONS                                                           ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║  ❌ OS Keychain Access                                                        ║
║     Container cannot access macOS Keychain or Linux Secret Service.           ║
║     → Use environment variables: ANTHROPIC_API_KEY, OPENAI_API_KEY, etc.      ║
║                                                                               ║
║  ❌ Daemon Socket                                                              ║
║     Unix socket IPC requires volume mount.                                     ║
║     → Mount: -v ~/.spn:/root/.spn                                             ║
║                                                                               ║
║  ⚠️ Ollama Integration                                                        ║
║     Must connect to host Ollama or run Ollama in sidecar container.           ║
║     → Use: --network host or docker-compose                                   ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

---

## 3. Implementation Details

### 3.1 Dockerfile

```dockerfile
# syntax=docker/dockerfile:1.6
# =============================================================================
# SuperNovae CLI (spn) — Multi-Platform Docker Image
# =============================================================================
# This Dockerfile uses pre-built binaries from GitHub Actions.
# It does NOT compile Rust — binaries are injected at build time.
# =============================================================================

FROM --platform=$TARGETPLATFORM gcr.io/distroless/cc-debian12:nonroot

# OCI Labels (https://github.com/opencontainers/image-spec/blob/main/annotations.md)
LABEL org.opencontainers.image.source="https://github.com/supernovae-st/supernovae-cli"
LABEL org.opencontainers.image.description="SuperNovae CLI — Unified package manager for AI workflows"
LABEL org.opencontainers.image.licenses="AGPL-3.0-or-later"
LABEL org.opencontainers.image.vendor="SuperNovae Studio"
LABEL org.opencontainers.image.title="spn"

# Build arguments (set by GitHub Actions)
ARG TARGETARCH
ARG VERSION=dev

LABEL org.opencontainers.image.version="${VERSION}"

# Copy pre-built binary for target architecture
# Binary path: docker-context/${TARGETARCH}/spn
COPY --chown=nonroot:nonroot ${TARGETARCH}/spn /usr/local/bin/spn

# Ensure executable
USER nonroot:nonroot

# Working directory for mounted projects
WORKDIR /workspace

# Default entrypoint
ENTRYPOINT ["/usr/local/bin/spn"]
CMD ["--help"]
```

### 3.2 GitHub Actions Workflow

```yaml
# .github/workflows/docker-publish.yml
name: Docker Publish

on:
  push:
    tags:
      - 'v*'
  workflow_dispatch:
    inputs:
      tag:
        description: 'Tag to publish (e.g., v0.12.0)'
        required: true

env:
  REGISTRY: ghcr.io
  IMAGE_NAME: supernovae-st/spn

jobs:
  docker-publish:
    name: Build & Push Docker Image
    needs: build  # From release.yml (or use workflow_call)
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write
      attestations: write
      id-token: write

    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Download Linux amd64 artifact
        uses: actions/download-artifact@v4
        with:
          name: spn-x86_64-unknown-linux-gnu
          path: artifacts/amd64

      - name: Download Linux arm64 artifact
        uses: actions/download-artifact@v4
        with:
          name: spn-aarch64-unknown-linux-gnu
          path: artifacts/arm64

      - name: Extract binaries
        run: |
          mkdir -p docker-context/amd64 docker-context/arm64
          tar -xzf artifacts/amd64/spn-x86_64-unknown-linux-gnu.tar.gz -C docker-context/amd64
          tar -xzf artifacts/arm64/spn-aarch64-unknown-linux-gnu.tar.gz -C docker-context/arm64
          chmod +x docker-context/*/spn

      - name: Set up QEMU
        uses: docker/setup-qemu-action@v3

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Log in to GitHub Container Registry
        uses: docker/login-action@v3
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Extract version from tag
        id: version
        run: echo "version=${GITHUB_REF_NAME#v}" >> $GITHUB_OUTPUT

      - name: Docker metadata
        id: meta
        uses: docker/metadata-action@v5
        with:
          images: ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}
          tags: |
            # v0.12.0 -> 0.12.0, 0.12, 0, latest
            type=semver,pattern={{version}}
            type=semver,pattern={{major}}.{{minor}}
            type=semver,pattern={{major}}
            type=raw,value=latest,enable={{is_default_branch}}
            # Immutable SHA reference
            type=sha,prefix=sha-,format=short

      - name: Build and push
        id: push
        uses: docker/build-push-action@v6
        with:
          context: docker-context
          file: Dockerfile
          platforms: linux/amd64,linux/arm64
          push: true
          tags: ${{ steps.meta.outputs.tags }}
          labels: ${{ steps.meta.outputs.labels }}
          build-args: |
            VERSION=${{ steps.version.outputs.version }}
          cache-from: type=gha
          cache-to: type=gha,mode=max
          provenance: true
          sbom: true

      - name: Generate artifact attestation
        uses: actions/attest-build-provenance@v1
        with:
          subject-name: ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}
          subject-digest: ${{ steps.push.outputs.digest }}
          push-to-registry: true
```

### 3.3 Integration with release.yml

```yaml
# Modify .github/workflows/release.yml

jobs:
  build:
    # ... existing build job (unchanged)

  docker-publish:
    name: Publish Docker Image
    needs: build
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write
      attestations: write
      id-token: write
    steps:
      # ... (steps from docker-publish.yml above)

  release:
    name: Create Release
    needs: [build, docker-publish]  # Add docker-publish dependency
    # ... rest unchanged

  publish-crates:
    # ... unchanged

  update-homebrew:
    # ... unchanged
```

### 3.4 .dockerignore

```dockerignore
# Ignore everything except what we need
*

# Include Dockerfile
!Dockerfile

# Include pre-built binaries (created by CI)
!amd64/
!arm64/
```

---

## 4. Tagging Strategy

```
╔═══════════════════════════════════════════════════════════════════════════════╗
║  DOCKER TAGGING STRATEGY                                                      ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║  Tag                          When Created         Purpose                   ║
║  ─────────────────────────    ──────────────────   ─────────────────────────║
║  ghcr.io/supernovae-st/spn:0.12.0     v0.12.0 tag    Immutable version       ║
║  ghcr.io/supernovae-st/spn:0.12       v0.12.x tags   Minor version tracking  ║
║  ghcr.io/supernovae-st/spn:0          v0.x.x tags    Major version tracking  ║
║  ghcr.io/supernovae-st/spn:latest     Any tag        Latest stable           ║
║  ghcr.io/supernovae-st/spn:sha-abc12  Every build    Immutable SHA reference ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

---

## 5. Usage Examples

### Basic Usage

```bash
# Show help
docker run --rm ghcr.io/supernovae-st/spn:latest --help

# Check version
docker run --rm ghcr.io/supernovae-st/spn:latest --version

# List packages in a project
docker run --rm -v $(pwd):/workspace ghcr.io/supernovae-st/spn:latest list
```

### CI/CD Usage (GitHub Actions)

```yaml
# .github/workflows/test.yml
jobs:
  test:
    runs-on: ubuntu-latest
    container:
      image: ghcr.io/supernovae-st/spn:latest
    env:
      ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
    steps:
      - uses: actions/checkout@v4
      - run: spn install --frozen
      - run: spn nk run ci/test.yaml
```

### With Environment Variables (Secrets)

```bash
# Pass API keys via environment
docker run --rm \
  -e ANTHROPIC_API_KEY="sk-ant-..." \
  -e OPENAI_API_KEY="sk-..." \
  -v $(pwd):/workspace \
  ghcr.io/supernovae-st/spn:latest provider list
```

### With Host Daemon (Advanced)

```bash
# Connect to host daemon via socket mount
docker run --rm \
  -v ~/.spn:/root/.spn \
  -v $(pwd):/workspace \
  ghcr.io/supernovae-st/spn:latest daemon status
```

### Docker Compose (Full Stack)

```yaml
# docker-compose.yml
version: '3.8'
services:
  spn:
    image: ghcr.io/supernovae-st/spn:latest
    volumes:
      - .:/workspace
      - ~/.spn:/root/.spn
    environment:
      - ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}
    command: ["daemon", "start", "--foreground"]

  ollama:
    image: ollama/ollama:latest
    volumes:
      - ollama-models:/root/.ollama
    ports:
      - "11434:11434"

volumes:
  ollama-models:
```

---

## 6. Security Considerations

### Supply Chain Security

| Measure | Implementation |
|---------|----------------|
| **SLSA Provenance** | `provenance: true` in build-push-action |
| **SBOM Generation** | `sbom: true` in build-push-action |
| **Attestation** | `actions/attest-build-provenance@v1` |
| **Image Signing** | Future: Sigstore/Cosign integration |
| **Vulnerability Scanning** | Future: Trivy in CI |

### Container Security

| Measure | Implementation |
|---------|----------------|
| **Non-root User** | `USER nonroot:nonroot` in Dockerfile |
| **Minimal Base** | `distroless/cc` (no shell, no package manager) |
| **Read-only FS** | Supported via `--read-only` flag |
| **No Capabilities** | Run with `--cap-drop=ALL` |

### Verification Commands

```bash
# Verify image provenance
gh attestation verify ghcr.io/supernovae-st/spn:latest

# Check image digest
docker inspect --format='{{index .RepoDigests 0}}' ghcr.io/supernovae-st/spn:latest
```

---

## 7. Documentation Updates

### README.md Addition

```markdown
### Docker

Run `spn` in a container:

```bash
# Quick usage
docker run --rm ghcr.io/supernovae-st/spn:latest --version

# With project mount
docker run --rm -v $(pwd):/workspace ghcr.io/supernovae-st/spn:latest install

# With API keys
docker run --rm \
  -e ANTHROPIC_API_KEY="$ANTHROPIC_API_KEY" \
  ghcr.io/supernovae-st/spn:latest provider test anthropic
```

**Note:** Docker cannot access OS Keychain. Use environment variables for secrets.

See [Docker Guide](docs/docker.md) for advanced usage.
```

---

## 8. Implementation Checklist

```
╔═══════════════════════════════════════════════════════════════════════════════╗
║  IMPLEMENTATION CHECKLIST                                                     ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║  Phase 1: Core Implementation                                                 ║
║  ├── [ ] Create Dockerfile                                                    ║
║  ├── [ ] Create .dockerignore                                                 ║
║  ├── [ ] Add docker-publish job to release.yml                               ║
║  ├── [ ] Test local Docker build                                              ║
║  └── [ ] Test multi-arch build with buildx                                   ║
║                                                                               ║
║  Phase 2: CI/CD Integration                                                   ║
║  ├── [ ] Verify GITHUB_TOKEN has packages:write permission                   ║
║  ├── [ ] Test workflow on feature branch                                     ║
║  ├── [ ] Create test release (v0.12.0-rc.1)                                  ║
║  └── [ ] Verify image appears in GitHub Packages                             ║
║                                                                               ║
║  Phase 3: Documentation                                                       ║
║  ├── [ ] Add Docker section to README.md                                     ║
║  ├── [ ] Create docs/docker.md guide                                         ║
║  ├── [ ] Update CHANGELOG.md                                                 ║
║  └── [ ] Add docker-compose.yml example                                      ║
║                                                                               ║
║  Phase 4: Release (v0.12.0)                                                   ║
║  ├── [ ] Bump version to 0.12.0                                              ║
║  ├── [ ] Create release tag                                                   ║
║  ├── [ ] Verify all 4 distribution channels work                            ║
║  └── [ ] Update GitHub repo description                                      ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

---

## 9. Cost & Performance Analysis

### Build Time Impact

| Job | Current | With Docker | Delta |
|-----|---------|-------------|-------|
| build (4 targets) | 12 min | 12 min | 0 |
| docker-publish | — | 2 min | +2 min |
| release | 1 min | 1 min | 0 |
| **Total** | **~15 min** | **~17 min** | **+13%** |

### Image Size

| Component | Size |
|-----------|------|
| Base image (distroless/cc) | 10 MB |
| spn binary (stripped) | 8 MB |
| **Total** | **~18 MB** |

### GitHub Actions Minutes

- Additional cost: ~2 min per release
- Monthly impact: Negligible (<10 min/month)

---

## 10. Future Enhancements (v0.13.0+)

| Feature | Priority | Description |
|---------|----------|-------------|
| **Docker Hub Mirror** | Medium | Push to docker.io/supernovae/spn |
| **Vulnerability Scanning** | High | Trivy scan in CI |
| **Image Signing** | Medium | Cosign + Sigstore |
| **Bundled Packages** | Low | Pre-installed common packages |
| **Alpine Variant** | Low | `spn:alpine` with shell access |

---

## Appendix A: Research Sources

1. **Agent: Web Researcher** — ghcr.io best practices, multi-stage Dockerfile patterns
2. **Agent: Code Architect** — Release workflow analysis, integration recommendations
3. **Agent: Strategy Brainstorm** — Base image comparison, use case analysis
4. **Perplexity Sonar** — 2024-2025 Docker/Rust/GitHub Actions trends
5. **Context7 Docker Docs** — Official multi-platform workflow patterns

---

## Appendix B: Alternative Approaches (Rejected)

### Build Inside Docker (Rejected)

```dockerfile
# NOT RECOMMENDED: Duplicates 12 min Rust compilation
FROM rust:1.75 as builder
RUN cargo build --release
```

**Why rejected:** Wastes CI minutes, produces different binary than releases.

### Separate Workflow (Rejected)

```yaml
# NOT RECOMMENDED: Creates orphan images on failed releases
on:
  release:
    types: [published]
```

**Why rejected:** Race conditions with release creation.

---

*Document generated by Claude Code + Research Agents*
