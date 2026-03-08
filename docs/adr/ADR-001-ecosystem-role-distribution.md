# ADR-001: SuperNovae Ecosystem Role Distribution

**Status:** Accepted
**Date:** 2026-03-08
**Authors:** Thibaut, Claude
**Supersedes:** None
**Related:** ADR-002, ADR-003

---

## Context

The SuperNovae ecosystem consists of three main components:
- **spn** — CLI tool for package management, secrets, models, MCP
- **Nika** — Workflow engine with DAG execution, TUI, LLM inference
- **NovaNet** — Knowledge graph with Neo4j, MCP server

As these tools evolve, responsibilities have become unclear:
- Where should secrets management live?
- Where should MCP server lifecycle management live?
- Where should scheduling/cron live?
- How should these components interact?

Without clear boundaries, we risk:
- Duplicated functionality
- Tight coupling preventing independent evolution
- Confused users who don't know which tool to use

---

## Decision

We adopt a **layered architecture** with clear separation of concerns:

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  LAYER 1: INFRASTRUCTURE (spn)                                                  │
│  Responsibility: WHEN, WHERE, WITH WHAT                                         │
├─────────────────────────────────────────────────────────────────────────────────┤
│  • Secrets (keychain, Vault, 1Password, rotation)                               │
│  • MCP Registry (config, lifecycle: start/stop/health)                          │
│  • Model Registry (Ollama: pull, load, status, simple inference)                │
│  • Scheduler/Cron (when to run anything)                                        │
│  • Packages (install, update, sync to editors)                                  │
│  • Daemon (long-running runtime, credential cache)                              │
│  • Editor Sync (Claude Code, Cursor, Windsurf, VS Code)                         │
└─────────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼ (optional, recommended)
┌─────────────────────────────────────────────────────────────────────────────────┐
│  LAYER 2: EXECUTION (Nika)                                                      │
│  Responsibility: HOW                                                            │
├─────────────────────────────────────────────────────────────────────────────────┤
│  • Workflow Execution (DAG, steps, dependencies, retries)                       │
│  • LLM Inference (infer: verb with context, streaming, multi-turn)              │
│  • MCP Tool Invocation (invoke: verb with error handling)                       │
│  • Agent Orchestration (agent: verb with memory, reasoning)                     │
│  • TUI (8 views, sessions, history, editing)                                    │
│  • Sessions/History (state management across runs)                              │
└─────────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼ (optional)
┌─────────────────────────────────────────────────────────────────────────────────┐
│  LAYER 3: KNOWLEDGE (NovaNet)                                                   │
│  Responsibility: WHAT                                                           │
├─────────────────────────────────────────────────────────────────────────────────┤
│  • Knowledge Graph (61 NodeClasses, 182 ArcClasses)                             │
│  • Context Assembly (novanet_generate for LLM prompts)                          │
│  • Locale Knowledge (terms, expressions, cultural refs)                         │
│  • MCP Server (exposes novanet_* tools)                                         │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Guiding Principles

1. **Each component CAN work standalone**
   - spn alone: infrastructure management without workflows
   - Nika alone: workflow execution with env vars (no spn)
   - NovaNet alone: knowledge graph with direct queries

2. **Each component is BETTER with others**
   - Nika + spn: no keychain popups, centralized config
   - Nika + NovaNet: knowledge-aware workflows
   - Full stack: production-ready AI infrastructure

3. **Communication via standard protocols**
   - spn ↔ Nika: MCP protocol (optional) or env vars
   - Nika ↔ NovaNet: MCP protocol
   - No direct library linking required

4. **Infrastructure owns lifecycle**
   - spn-daemon is the "systemd of AI"
   - Scheduler, secrets, MCP lifecycle all in spn
   - Nika focuses purely on execution logic

---

## Consequences

### Positive

1. **Clear mental model** — Users know which tool does what
2. **Independent evolution** — Each component can release independently
3. **Flexible adoption** — Teams can adopt incrementally
4. **Reduced complexity** — Each codebase has focused scope
5. **Better testing** — Clear boundaries enable mocking

### Negative

1. **Communication overhead** — Components must coordinate releases
2. **Duplication risk** — Temptation to add features to "wrong" layer
3. **Integration testing** — Need cross-component test suite

### Neutral

1. **Documentation** — Must clearly explain which tool for what
2. **Onboarding** — Users learn three tools instead of one

---

## Implementation

### Phase 1: Establish Boundaries (v0.15.0)
- Document role distribution (this ADR)
- Audit existing features for misplacement
- No feature moves yet

### Phase 2: Strengthen spn Infrastructure (v0.16.0)
- Add MCP server mode to daemon
- Add MCP lifecycle commands (start/stop/health)
- Remove scheduler code from Nika (prep)

### Phase 3: Move Scheduler (v0.17.0)
- Implement `spn schedule` commands
- Deprecate Nika scheduler
- Migration guide for users

### Phase 4: Stabilize (v0.18.0)
- Remove deprecated Nika scheduler
- Full ecosystem documentation
- Integration test suite

---

## Alternatives Considered

### Alternative A: Monolithic Tool
Put everything in one tool (e.g., Nika does infrastructure + execution).

**Rejected because:**
- Massive codebase
- Can't use parts independently
- Long compile times
- Harder to maintain

### Alternative B: Microservices
Each feature is a separate binary (spn-secrets, spn-mcp, spn-models, etc.).

**Rejected because:**
- Too many binaries to install
- Complex coordination
- Overkill for CLI tools

### Alternative C: Library-based Integration
Nika links spn-client library directly.

**Partially adopted:**
- spn-client exists for direct integration
- But MCP mode preferred for loose coupling

---

## References

- [NovaNet ADR-012: Realm Architecture](../novanet/ADR-012-realm-architecture.md)
- [Nika ADR-001: 5 Semantic Verbs](../nika/ADR-001-semantic-verbs.md)
- [Unix Philosophy](https://en.wikipedia.org/wiki/Unix_philosophy)
- [12 Factor App](https://12factor.net/)

---

## Changelog

| Date | Change |
|------|--------|
| 2026-03-08 | Initial version |
