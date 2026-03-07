# Planning Documents Index

This directory contains 36 planning documents for the SuperNovae CLI (`spn`) project.

**Current Version:** v0.14.2 | **Last Updated:** 2026-03-07

---

## Active Plans (v0.14.x - v0.15.x)

| Document | Date | Status | Description |
|----------|------|--------|-------------|
| [v014-v015-roadmap](2026-03-07-v014-v015-roadmap.md) | 2026-03-07 | **Active** | v0.14.0 "Delight Release" + v0.15.0 vision |
| [ux-waouh-plan](2026-03-07-ux-waouh-plan.md) | 2026-03-07 | **Active** | Semantic design system architecture |
| [ux-improvements-analysis](2026-03-07-ux-improvements-analysis.md) | 2026-03-07 | Complete | UX analysis and feature breakdown |
| [phase3-5-completion](2026-03-07-phase3-5-completion.md) | 2026-03-07 | Complete | UX phases 3-5 implementation summary |

---

## Recent Plans (2026-03-06)

| Document | Status | Description |
|----------|--------|-------------|
| [unified-registry-architecture](2026-03-06-unified-registry-architecture.md) | Complete | Registry and index design |
| [spn-publish-v07-plan](2026-03-06-spn-publish-v07-plan.md) | Complete | crates.io publishing strategy |
| [spn-cli-improvements](2026-03-06-spn-cli-improvements.md) | Complete | 22KB unified features spec |
| [phase2-implementation](2026-03-06-phase2-implementation.md) | Complete | Phase 2 "Quick Wins" |
| [optimization-session](2026-03-06-optimization-session.md) | Complete | Performance optimization results |
| [deduplication-refactoring](2026-03-06-deduplication-refactoring.md) | In Progress | Code deduplication refactor |

---

## Architecture Plans (2026-03-04 - 2026-03-05)

| Document | Status | Description |
|----------|--------|-------------|
| [spn-daemon-architecture](2026-03-04-spn-daemon-architecture.md) | **Implemented** | Unix socket IPC, OS keychain caching |
| [nika-spn-unified-architecture](2026-03-05-nika-spn-unified-architecture.md) | **Implemented** | spn-client SDK, unified secrets |
| [master-execution-plan](2026-03-05-master-execution-plan.md) | Complete | End-to-end execution strategy |
| [MASTER-EXECUTION-PLAN (v2)](2026-03-04-MASTER-EXECUTION-PLAN.md) | Superseded | Earlier master plan |

---

## Model Management Plans

| Document | Status | Description |
|----------|--------|-------------|
| [plan-model-cli-commands](2026-03-05-plan-model-cli-commands.md) | **Implemented** | 29KB model CLI spec (pull, load, unload) |
| [plan2-model-management](2026-03-05-plan2-model-management.md) | **Implemented** | Model management architecture |
| [plan2b-model-backend-trait](2026-03-05-plan2b-model-backend-trait.md) | **Implemented** | ModelBackend trait design |

---

## Secret Management Plans

| Document | Status | Description |
|----------|--------|-------------|
| [secrets-architecture-refactor](2026-03-05-secrets-architecture-refactor.md) | **Implemented** | 5-crate workspace design |
| [secrets-p2-implementation](2026-03-03-secrets-p2-implementation.md) | Complete | Phase 2 secrets impl |
| [secret-management-implementation](2026-03-03-secret-management-implementation.md) | Complete | Implementation details |
| [secret-management-design](2026-03-03-secret-management-design.md) | Complete | Design decisions |

---

## Early Plans (v0.1 - v0.7)

| Document | Status | Description |
|----------|--------|-------------|
| [v0.7.0-ENHANCED-PLAN](2026-03-02-v0.7.0-ENHANCED-PLAN.md) | Superseded | v0.7.0 foundation |
| [v0.3.0-full-scope](2026-03-01-v0.3.0-full-scope.md) | Superseded | v0.3.0 scope |
| [WEEK1-DAY1-PLAN](2026-03-02-WEEK1-DAY1-PLAN.md) | Complete | Initial kickoff |
| [VISION-SUMMARY](2026-03-02-VISION-SUMMARY.md) | Reference | Project vision |
| [SPN-NIKA-EXECUTION-PLAN](2026-03-02-SPN-NIKA-EXECUTION-PLAN.md) | Superseded | Early execution |
| [INTEGRATION-PLAN](2026-03-02-INTEGRATION-PLAN.md) | Complete | Integration strategy |
| [EXECUTION-PLAN](2026-03-02-EXECUTION-PLAN.md) | Superseded | v0.1 execution |
| [ecosystem-architecture-design](2026-03-02-ecosystem-architecture-design.md) | Reference | Ecosystem design |

---

## Reference Documents

| Document | Description |
|----------|-------------|
| [release-automation](release-automation.md) | release-plz + git-cliff setup |
| [docker-distribution](docker-distribution.md) | Docker multi-platform builds |
| [daemon-registry-ux-roadmap](daemon-registry-ux-roadmap.md) | Registry UX roadmap |
| [security-hardening-v0.13](security-hardening-v0.13.md) | Security hardening notes |

---

## Diagnostics & TODOs

| Document | Description |
|----------|-------------|
| [DIAGNOSTIC-TOKIO-PANIC](DIAGNOSTIC-TOKIO-PANIC.md) | Tokio panic debugging |
| [TODO-developer-id-signing](TODO-developer-id-signing.md) | macOS code signing TODO |
| [UX-secret-storage-options](UX-secret-storage-options.md) | Secret storage UX research |

---

## Status Legend

| Status | Meaning |
|--------|---------|
| **Active** | Currently being worked on |
| **Implemented** | Shipped in a release |
| Complete | Planning complete, may or may not be implemented |
| In Progress | Partially implemented |
| Superseded | Replaced by newer plan |
| Reference | Background/context document |

---

## Version History

| Version | Key Plans |
|---------|-----------|
| v0.14.x | v014-v015-roadmap, ux-waouh-plan, phase3-5-completion |
| v0.12.x | docker-distribution, unified-registry-architecture |
| v0.11.x | plan-model-cli-commands |
| v0.10.x | spn-daemon-architecture, secrets-architecture-refactor |
| v0.9.x | secret-management-design |
| v0.7.x | v0.7.0-ENHANCED-PLAN |

---

*Last updated: 2026-03-07*
