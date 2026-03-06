# Unified Registry Architecture Plan

**Date:** 2026-03-06
**Status:** DRAFT
**Author:** Claude + Thibaut
**Affects:** supernovae-registry, supernovae-powers, supernovae-cli

---

## Executive Summary

Unifier l'architecture de tous les registries SuperNovae avec un pattern **HYBRIDE** cohérent:
- **Métadonnées**: toujours dans notre registry (supernovae-registry)
- **Contenu**: source appropriée selon le type (nous, npm, Ollama, HuggingFace)

---

## 1. État Actuel

### 1.1 Inventaire

| Registry | Packages | Pattern | Problèmes |
|----------|----------|---------|-----------|
| supernovae-registry | 46 (workflows, skills, agents...) | Self-hosted | ✅ OK |
| supernovae-powers | 45 (studio, qrcodeai...) | Self-hosted | ✅ OK |
| MCP Servers | 48 aliases | Hardcodé dans npm.rs | ❌ Pas de registry |
| Models | 0 | Proxy Ollama | ❌ Pas de registry |

### 1.2 Problèmes Identifiés

```
MCP SERVERS:
├── 48 aliases hardcodés dans crates/spn/src/interop/npm.rs
├── Pour ajouter un MCP → modifier code, rebuild, release
├── Pas de métadonnées (description, tags, docs)
├── Pas de recherche possible
└── Pas de vérification de version

MODELS:
├── Aucune liste de modèles recommandés
├── Pas de métadonnées (benchmarks, use cases)
├── Pas de recherche
└── User doit connaître le nom Ollama exact
```

---

## 2. Architecture Cible

### 2.1 Pattern Hybride Unifié

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  HYBRID PATTERN: Registry = Index, Upstream = Source                           │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  supernovae-registry (SOURCE DE VÉRITÉ pour métadonnées)                        │
│  ┌────────────────────────────────────────────────────────────────────────┐    │
│  │  registry.json                                                          │    │
│  │  ├── @workflows/... (type: workflow)                                    │    │
│  │  ├── @skills/...    (type: skill)                                       │    │
│  │  ├── @agents/...    (type: agent)                                       │    │
│  │  ├── @mcp/...       (type: mcp)      ← NOUVEAU                          │    │
│  │  └── @models/...    (type: model)    ← NOUVEAU                          │    │
│  └────────────────────────────────────────────────────────────────────────┘    │
│                                                                                 │
│  Chaque package a un champ "source" qui indique où télécharger:                │
│                                                                                 │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐             │
│  │ source.type:    │    │ source.type:    │    │ source.type:    │             │
│  │ "tarball"       │    │ "npm"           │    │ "ollama"        │             │
│  │                 │    │                 │    │                 │             │
│  │ → Our releases/ │    │ → npm registry  │    │ → Ollama API    │             │
│  │ (workflows,     │    │ (MCP servers)   │    │ (models)        │             │
│  │  skills, etc)   │    │                 │    │                 │             │
│  └─────────────────┘    └─────────────────┘    └─────────────────┘             │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 Nouveau Schema de Package

```yaml
# Champs communs à TOUS les types
name: "@mcp/neo4j"           # Identifiant unique
version: "1.2.0"             # Version du package dans notre registry
type: "mcp"                  # workflow | skill | agent | mcp | model | prompt | job | schema
description: "Neo4j graph database MCP server"
author: "neo4j"
license: "MIT"
tags: ["database", "graph", "neo4j", "cypher"]
homepage: "https://github.com/neo4j/mcp-server-neo4j"

# NOUVEAU: Source du contenu
source:
  type: "npm"                           # tarball | npm | pypi | binary | ollama | huggingface
  package: "@neo4j/mcp-server-neo4j"    # Nom du package upstream
  version: "^1.2.0"                     # Contrainte de version upstream

# Métadonnées enrichies (spécifiques au type)
mcp:                         # Pour type: mcp
  tools:
    - name: "read_neo4j_cypher"
      description: "Execute read Cypher query"
    - name: "write_neo4j_cypher"
      description: "Execute write Cypher query"
  resources:
    - name: "neo4j://schema"
      description: "Database schema"
  env_vars:
    - name: "NEO4J_URI"
      required: true
      description: "Neo4j connection URI"
    - name: "NEO4J_PASSWORD"
      required: true
      secret: true
      description: "Neo4j password"

# Exemple pour un model
model:                       # Pour type: model
  ollama_name: "deepseek-coder:6.7b"
  variants:
    - size: "6.7b"
      vram: "5GB"
      quantization: "Q4_K_M"
  benchmarks:
    humaneval: 73.8
    mbpp: 65.2
  capabilities: ["code-generation", "code-review"]
  recommended_for: ["nika infer: for code tasks"]
```

### 2.3 Source Types

| Type | Utilisé pour | Exemple source | Stockage contenu |
|------|--------------|----------------|------------------|
| `tarball` | workflows, skills, agents, prompts, jobs, schemas | `releases/@w/code-review/1.0.0.tar.gz` | Notre registry |
| `npm` | MCP servers (Node.js) | `@neo4j/mcp-server-neo4j` | npm registry |
| `pypi` | MCP servers (Python) | `mcp-server-xyz` | PyPI |
| `binary` | MCP servers (standalone) | `releases/@mcp/neo4j/1.0.0-darwin-arm64` | Notre registry |
| `ollama` | Models | `deepseek-coder:6.7b` | Ollama local |
| `huggingface` | Models (direct GGUF) | `TheBloke/deepseek-coder-6.7B-GGUF` | HuggingFace |

---

## 3. Structure Registry

### 3.1 Nouvelle Structure

```
supernovae-registry/
├── registry.json              # Catalogue complet (tous types)
├── config.json                # Configuration (dl URLs, API)
├── index/
│   ├── @w/...                 # Workflows (existant)
│   ├── @s/...                 # Skills (existant)
│   ├── @a/...                 # Agents (existant)
│   ├── @mcp/                  # MCP Servers (NOUVEAU)
│   │   ├── neo4j
│   │   ├── github
│   │   ├── filesystem
│   │   └── ...
│   └── @models/               # Models (NOUVEAU)
│       ├── code/
│       │   ├── deepseek-coder
│       │   ├── codellama
│       │   └── qwen2.5-coder
│       ├── chat/
│       │   ├── llama3.2
│       │   └── mistral
│       └── embed/
│           └── nomic-embed-text
├── releases/
│   ├── @w/...                 # Tarballs workflows
│   ├── @s/...                 # Tarballs skills
│   └── @mcp/                  # Binaires MCP (optionnel)
│       └── custom-server/
│           └── 1.0.0-darwin-arm64
└── packages/
    ├── @workflows/...         # Source packages
    ├── @mcp/                  # MCP package.yaml (NOUVEAU)
    │   ├── neo4j/
    │   │   └── package.yaml
    │   ├── github/
    │   │   └── package.yaml
    │   └── ...
    └── @models/               # Model package.yaml (NOUVEAU)
        ├── code/
        │   └── deepseek-coder/
        │       └── package.yaml
        └── ...
```

### 3.2 registry.json Étendu

```json
{
  "version": 3,
  "name": "supernovae-registry",
  "types": {
    "workflow": { "scopes": ["@workflows", "@nika"], "file_pattern": "*.nika.yaml" },
    "skill": { "scopes": ["@skills"], "file_pattern": "*.skill.md" },
    "agent": { "scopes": ["@agents"], "file_pattern": "*.agent.yaml" },
    "mcp": { "scopes": ["@mcp"], "source_types": ["npm", "pypi", "binary"] },
    "model": { "scopes": ["@models"], "source_types": ["ollama", "huggingface"] },
    "prompt": { "scopes": ["@prompts"], "file_pattern": "*.md" },
    "job": { "scopes": ["@jobs"], "file_pattern": "*.job.toml" },
    "schema": { "scopes": ["@schemas"], "file_pattern": "*.yaml" }
  },
  "packages": {
    "@mcp/neo4j": {
      "version": "1.2.0",
      "type": "mcp",
      "description": "Neo4j graph database MCP server",
      "source": { "type": "npm", "package": "@neo4j/mcp-server-neo4j" },
      "tags": ["database", "graph", "cypher"]
    },
    "@models/code/deepseek-coder": {
      "version": "1.0.0",
      "type": "model",
      "description": "Best open-source coding model",
      "source": { "type": "ollama", "model": "deepseek-coder:6.7b" },
      "tags": ["code", "python", "javascript"]
    }
  },
  "stats": {
    "total_packages": 120,
    "by_type": {
      "workflow": 18,
      "skill": 8,
      "agent": 5,
      "mcp": 48,
      "model": 25,
      "prompt": 6,
      "job": 5,
      "schema": 5
    }
  }
}
```

---

## 4. Implémentation spn

### 4.1 Refactor: Supprimer les Hardcodes

```rust
// AVANT (npm.rs) - 48 aliases hardcodés
pub fn mcp_aliases() -> FxHashMap<&'static str, &'static str> {
    FxHashMap::from_iter([
        ("neo4j", "@neo4j/mcp-server-neo4j"),
        // ... 47 autres
    ])
}

// APRÈS - fetch depuis registry
pub async fn resolve_mcp(name: &str) -> Result<McpPackage> {
    let registry = fetch_registry().await?;
    let pkg = registry.get(&format!("@mcp/{}", name))?;

    match &pkg.source.type_ {
        SourceType::Npm { package, .. } => Ok(McpPackage::Npm(package.clone())),
        SourceType::Binary { platforms, .. } => {
            let url = platforms.get(&current_platform())?;
            Ok(McpPackage::Binary(url.clone()))
        }
        _ => Err(Error::InvalidSource),
    }
}
```

### 4.2 Nouvelles Commandes

```bash
# MCP (amélioré)
spn mcp search database        # Cherche dans @mcp/*
spn mcp info neo4j             # Affiche métadonnées enrichies
spn mcp add neo4j              # Résout via registry, installe via npm/binary

# Models (nouveau)
spn model search code          # Cherche dans @models/*
spn model info deepseek-coder  # Benchmarks, recommended_for, etc.
spn model add deepseek-coder   # Résout via registry, pull via Ollama
spn model recommend --for code # Recommandations intelligentes

# Unifié
spn search "code review"       # Cherche TOUS les types
spn info @mcp/neo4j            # Info any package
spn add @models/code/deepseek  # Add any package
```

### 4.3 Flow Unifié

```rust
pub async fn add_package(name: &str) -> Result<()> {
    // 1. Fetch metadata from registry
    let pkg = fetch_package_metadata(name).await?;

    // 2. Install from source based on type
    match &pkg.source {
        Source::Tarball { url, checksum } => {
            download_and_extract(url, checksum).await?;
        }
        Source::Npm { package, version } => {
            npm_install(package, version).await?;
        }
        Source::Ollama { model } => {
            ollama_pull(model).await?;
        }
        Source::HuggingFace { repo, quantization } => {
            hf_download(repo, quantization).await?;
        }
    }

    // 3. Update local state
    update_state(name, &pkg).await?;

    // 4. Sync to editors if needed
    sync_to_editors(&pkg).await?;
}
```

---

## 5. MCP Registry Content

### 5.1 Initial Seed (48 MCP servers)

Migration des 48 aliases de `npm.rs` vers le registry:

```yaml
# packages/@mcp/neo4j/package.yaml
name: "@mcp/neo4j"
version: "1.0.0"
type: mcp
description: "Neo4j graph database MCP server - Cypher queries and schema inspection"
author: "neo4j"
license: "MIT"
homepage: "https://github.com/neo4j/mcp-server-neo4j"
tags: ["database", "graph", "neo4j", "cypher", "knowledge-graph"]

source:
  type: npm
  package: "@neo4j/mcp-server-neo4j"
  version: "^1.0.0"

mcp:
  tools:
    - name: read_neo4j_cypher
      description: "Execute a read-only Cypher query"
    - name: write_neo4j_cypher
      description: "Execute a write Cypher query"
    - name: get_neo4j_schema
      description: "Get the database schema"
  env_vars:
    - name: NEO4J_URI
      required: true
      example: "bolt://localhost:7687"
    - name: NEO4J_USERNAME
      required: false
      default: "neo4j"
    - name: NEO4J_PASSWORD
      required: true
      secret: true

integration:
  nika:
    example: |
      mcp:
        neo4j:
          provider: "@mcp/neo4j"
  claude_code:
    config: |
      "neo4j": {
        "command": "npx",
        "args": ["-y", "@neo4j/mcp-server-neo4j"]
      }
```

### 5.2 MCP Categories

```
@mcp/
├── databases/
│   ├── neo4j, postgres, sqlite, mysql
│   ├── supabase, neon, planetscale
│   └── qdrant, pinecone, weaviate, milvus
├── cloud/
│   ├── aws-*, gcp-*, azure-*
│   ├── vercel, cloudflare
│   └── stripe, linear
├── productivity/
│   ├── github, gitlab, notion
│   ├── slack, gdrive, airtable
│   └── sentry, raygun
├── search/
│   ├── brave-search, perplexity
│   ├── exa, tavily
│   └── google-maps
├── browser/
│   ├── puppeteer, browserbase
│   └── firecrawl
└── dev-tools/
    ├── filesystem, memory, fetch
    ├── docker, kubernetes
    └── sequential-thinking
```

---

## 6. Models Registry Content

### 6.1 Initial Seed (~25 models)

```yaml
# packages/@models/code/deepseek-coder/package.yaml
name: "@models/code/deepseek-coder"
version: "1.0.0"
type: model
description: "Best open-source coding model, rivals GPT-4 for code tasks"
author: "deepseek-ai"
license: "MIT"
homepage: "https://github.com/deepseek-ai/DeepSeek-Coder"
tags: ["code", "python", "javascript", "rust", "completion", "review"]

source:
  type: ollama
  model: "deepseek-coder"

model:
  category: code
  variants:
    - name: "6.7b"
      ollama: "deepseek-coder:6.7b"
      size: "4.1GB"
      vram: "5GB"
      quantization: "Q4_K_M"
    - name: "33b"
      ollama: "deepseek-coder:33b"
      size: "19GB"
      vram: "24GB"
      quantization: "Q4_K_M"

  benchmarks:
    humaneval: 73.8
    mbpp: 65.2

  capabilities:
    - code-generation
    - code-completion
    - code-review
    - refactoring
    - bug-fixing

  recommended_for:
    - "nika infer: for code generation"
    - "Code review workflows"
    - "Refactoring automation"

  not_recommended_for:
    - "General chat (use llama3.2)"
    - "Long documents (use mistral)"

integration:
  nika:
    example: |
      providers:
        default: ollama/deepseek-coder:6.7b

      tasks:
        - infer: "Review this code"
          model: deepseek-coder
```

### 6.2 Model Categories

```
@models/
├── code/
│   ├── deepseek-coder (6.7b, 33b)
│   ├── codellama (7b, 13b, 34b)
│   ├── starcoder2 (3b, 7b, 15b)
│   ├── qwen2.5-coder (7b, 14b, 32b)
│   └── codegemma (7b)
├── chat/
│   ├── llama3.2 (1b, 3b, 8b, 70b)
│   ├── mistral (7b)
│   ├── phi3 (mini, small, medium)
│   ├── gemma2 (2b, 9b, 27b)
│   └── qwen2.5 (7b, 14b, 32b)
├── embed/
│   ├── nomic-embed-text
│   ├── mxbai-embed-large
│   ├── all-minilm
│   └── snowflake-arctic-embed
├── vision/
│   ├── llava (7b, 13b)
│   ├── llava-phi3
│   └── moondream2
└── reasoning/
    ├── deepseek-r1 (quand dispo)
    └── qwq (quand dispo)
```

---

## 7. Migration Plan

### Phase 1: Schema & Structure (Day 1)

```
Tasks:
├── [1.1] Update registry.json schema to v3
│   ├── Add "mcp" and "model" types
│   └── Add "source" field to package schema
│
├── [1.2] Create packages/@mcp/ structure
│   └── Migrate 48 aliases from npm.rs to package.yaml files
│
├── [1.3] Create packages/@models/ structure
│   └── Create ~25 model package.yaml files
│
├── [1.4] Update index/ structure
│   ├── Create index/@mcp/
│   └── Create index/@models/
│
└── [1.5] Run registry build script
    └── Generate registry.json, index files
```

### Phase 2: spn Refactor (Day 2)

```
Tasks:
├── [2.1] Add Source enum to spn-core
│   └── Tarball, Npm, PyPi, Binary, Ollama, HuggingFace
│
├── [2.2] Refactor MCP commands
│   ├── Remove hardcoded mcp_aliases()
│   ├── Fetch from registry instead
│   └── Support npm/pypi/binary sources
│
├── [2.3] Implement model commands
│   ├── spn model search
│   ├── spn model info
│   ├── spn model add (via registry → Ollama)
│   └── spn model recommend
│
├── [2.4] Unified search
│   └── spn search <query> searches all types
│
└── [2.5] Update tests
```

### Phase 3: spn-ollama Completion (Day 2)

```
Tasks:
├── [3.1] Add /api/chat endpoint
├── [3.2] Add /api/embeddings endpoint
├── [3.3] Add /api/generate (full, not just warmup)
└── [3.4] Tests
```

### Phase 4: Documentation & Polish (Day 3)

```
Tasks:
├── [4.1] Update README with new commands
├── [4.2] Document package.yaml schema
├── [4.3] Create contribution guide for adding MCP/models
├── [4.4] Update CHANGELOG
└── [4.5] Test E2E flows
```

---

## 8. Benefits

### 8.1 Pour les Users

| Avant | Après |
|-------|-------|
| `spn mcp add neo4j` (doit connaître le nom) | `spn mcp search database` → trouve neo4j |
| Pas d'info sur les MCP | `spn mcp info neo4j` → tools, env vars, examples |
| Pas de modèles recommandés | `spn model recommend --for code` → deepseek-coder |
| Doit connaître Ollama | `spn model add deepseek-coder` → just works |

### 8.2 Pour les Mainteneurs

| Avant | Après |
|-------|-------|
| Ajouter MCP = modifier npm.rs, rebuild, release | Ajouter MCP = PR sur registry |
| 48 aliases hardcodés | Métadonnées externalisées |
| Pas de versioning MCP | Version tracking |
| Incohérence entre types | Pattern unifié |

### 8.3 Métriques

| Métrique | Avant | Après |
|----------|-------|-------|
| Packages total | 46 | ~120 |
| Types supportés | 6 | 8 |
| MCP avec métadonnées | 0 | 48 |
| Models avec benchmarks | 0 | 25 |
| Hardcoded aliases | 48 | 0 |

---

## 9. Future Enhancements

### 9.1 Post-MVP

- **Model download progress in TUI**: Progress bar comme `spn model add`
- **Offline cache**: Cache registry.json localement
- **Private registries**: Support supernovae-powers dans spn
- **Version constraints**: `spn add @mcp/neo4j@^1.0`
- **Dependency resolution**: MCP servers qui dépendent d'autres packages

### 9.2 Community

- **spn publish @mcp/my-server**: Publier son propre MCP
- **Rating/reviews**: Notation des packages
- **Download stats**: Popularité des packages

---

## 10. Success Criteria

- [ ] 48 MCP servers dans @mcp/ avec métadonnées
- [ ] 25 models dans @models/ avec benchmarks
- [ ] `spn mcp search` fonctionne
- [ ] `spn model search` fonctionne
- [ ] `spn model recommend` fonctionne
- [ ] Zero aliases hardcodés dans npm.rs
- [ ] registry.json v3 avec tous les types
- [ ] Tests passent
- [ ] Documentation à jour

---

## 11. Files to Modify

### supernovae-registry

| File | Action |
|------|--------|
| `registry.json` | Upgrade to v3, add @mcp and @models |
| `packages/@mcp/*/package.yaml` | CREATE 48 files |
| `packages/@models/*/package.yaml` | CREATE 25 files |
| `index/@mcp/*` | CREATE index files |
| `index/@models/*` | CREATE index files |
| `scripts/build-registry.sh` | Update for new types |

### supernovae-cli

| File | Action |
|------|--------|
| `crates/spn-core/src/lib.rs` | Add Source enum |
| `crates/spn/src/interop/npm.rs` | Remove mcp_aliases(), use registry |
| `crates/spn/src/commands/mcp.rs` | Refactor to use registry |
| `crates/spn/src/commands/model.rs` | Add search, info, recommend |
| `crates/spn-ollama/src/client.rs` | Add chat, embeddings endpoints |
| `README.md` | Document new commands |

---

## Appendix A: Example Package Files

### A.1 MCP Package

```yaml
# packages/@mcp/github/package.yaml
name: "@mcp/github"
version: "1.0.0"
type: mcp
description: "GitHub API MCP server - repos, issues, PRs, actions"
author: "anthropic"
license: "MIT"
homepage: "https://github.com/modelcontextprotocol/servers"
tags: ["github", "git", "repos", "issues", "prs", "actions"]

source:
  type: npm
  package: "@modelcontextprotocol/server-github"
  version: "^0.6.0"

mcp:
  tools:
    - name: search_repositories
      description: "Search GitHub repositories"
    - name: get_file_contents
      description: "Get contents of a file"
    - name: create_or_update_file
      description: "Create or update a file"
    - name: push_files
      description: "Push multiple files"
    - name: create_issue
      description: "Create a new issue"
    - name: create_pull_request
      description: "Create a pull request"
    - name: list_commits
      description: "List commits in a branch"
  env_vars:
    - name: GITHUB_TOKEN
      required: true
      secret: true
      description: "Personal access token with repo scope"
```

### A.2 Model Package

```yaml
# packages/@models/chat/llama3.2/package.yaml
name: "@models/chat/llama3.2"
version: "1.0.0"
type: model
description: "Meta's Llama 3.2 - excellent general-purpose chat model"
author: "meta"
license: "Llama 3.2 Community License"
homepage: "https://ai.meta.com/llama/"
tags: ["chat", "general", "reasoning", "multilingual"]

source:
  type: ollama
  model: "llama3.2"

model:
  category: chat
  variants:
    - name: "1b"
      ollama: "llama3.2:1b"
      size: "1.3GB"
      vram: "2GB"
      best_for: "Quick responses, edge devices"
    - name: "3b"
      ollama: "llama3.2:3b"
      size: "2.0GB"
      vram: "3GB"
      best_for: "Balanced speed/quality"
    - name: "8b"
      ollama: "llama3.2"
      size: "4.7GB"
      vram: "6GB"
      best_for: "Best quality"
    - name: "70b"
      ollama: "llama3.2:70b"
      size: "40GB"
      vram: "48GB"
      best_for: "Maximum capability"

  benchmarks:
    mmlu: 73.0
    hellaswag: 85.0
    arc: 78.0

  capabilities:
    - chat
    - reasoning
    - summarization
    - translation
    - creative-writing

  languages:
    - en
    - es
    - fr
    - de
    - it
    - pt
    - zh
    - ja
    - ko
```

---

**Timeline:** 3 jours
**Priority:** HIGH (unifie l'architecture, élimine les hardcodes)
