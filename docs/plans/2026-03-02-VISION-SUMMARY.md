# SuperNovae CLI - Vision Finale ✨

**Date:** 2026-03-02
**Version:** v0.7.0 Target
**Status:** ✅ Vision Clarifiée

---

## 🎯 Vision en Une Phrase

**SuperNovae = npm pour l'IA. Nika = Node.js pour l'IA.**

- **spn** → Package manager (comme npm)
- **nika** → Runtime pour workflows IA (comme node)
- **novanet** → Knowledge graph (optionnel, v2)

---

## 🧩 Qui Fait Quoi?

```
USER
  ↓ spn search seo
  ↓ spn add @workflows/seo-audit
  ↓
SPN (Package Manager)
  ├─ Télécharge depuis registry
  ├─ Installe dans ~/.spn/packages/
  ├─ Met à jour spn.yaml + spn.lock
  └─ Symlink vers .nika/.cache/ (optionnel)
  ↓
NIKA (Runtime)
  ├─ nika run @workflows/seo-audit
  ├─ Résout @workflows/seo-audit → ~/.spn/packages/...
  ├─ Charge workflow.nika.yaml
  └─ Exécute avec provider LLM (Claude, OpenAI, etc.)
  ↓
RÉSULTAT
```

---

## 📦 Les 6 Types de Packages (MVP)

| Package | Exemple | Fichier | Usage Nika |
|---------|---------|---------|------------|
| **@workflows/** | `@workflows/seo-audit` | `workflow.nika.yaml` | `nika run @workflows/name` |
| **@agents/** | `@agents/researcher` | `agent.md` | `agent: { pkg: @agents/name }` |
| **@prompts/** | `@prompts/seo-meta` | `prompt.md` | Dans tasks, référence prompts |
| **@jobs/** | `@jobs/daily-report` | `job.nika.yaml` | `nika jobs start` (daemon) |
| **@skills/** | `brainstorming` | `skill.md` | `skills: { x: pkg:name }` (proxy skills.sh ✅) |
| **@mcp/** | `neo4j` | npm package | `mcp: { neo4j: ... }` (proxy npm ✅) |

**v2:** `@schemas/` (NovaNet integration)

---

## 🌍 Agnostique = Liberté

**Nika & SPN sont agnostiques:**

1. **LLM Provider:**
   - Claude (Anthropic)
   - OpenAI (GPT-4, etc.)
   - Mistral AI
   - Groq
   - DeepSeek
   - Ollama (local)
   - N'importe quel provider compatible

2. **Platform:**
   - Linux
   - macOS
   - Windows
   - Cross-platform Rust binaries

**Pas de lock-in.** Change de provider avec `--provider openai` ou dans config.

---

## 🚫 Ce qu'on NE FAIT PAS

- ❌ **Sync vers éditeurs** (Claude Code, Cursor, Windsurf)
  - C'était dans les anciens plans
  - **ON NE LE FAIT PLUS**
  - SPN = POUR NIKA seulement

- ❌ **Gérer les éditeurs**
  - Les éditeurs ont leurs propres package managers
  - On se concentre sur Nika

---

## ✅ User Journey Simplifié

### Cas 1: Dev Solo Découvre un Workflow

```bash
# 1. DÉCOUVRIR
$ spn search seo
🔍 @workflows/seo-audit v1.2.0 (⭐ 45)

# 2. INSTALLER
$ spn add @workflows/seo-audit
✓ Installed to ~/.spn/packages/

# 3. UTILISER
$ nika run @workflows/seo-audit --url https://qrcode-ai.com
✓ SEO Score: 95/100
```

**3 commandes. 30 secondes. Ça marche.**

### Cas 2: Dev Crée son Propre Workflow

```bash
# Créer localement (sans package)
$ nika init
$ cat > .nika/workflows/custom.nika.yaml <<EOF
schema: nika/workflow@0.9
tasks:
  - id: hello
    infer: "Say hello"
EOF

$ nika run custom  # Auto-résout .nika/workflows/custom.nika.yaml
```

**Packages ET fichiers locaux coexistent. Pas obligé d'utiliser spn.**

### Cas 3: Team Lead Standardise Workflows

```bash
# Créer spn.yaml pour l'équipe
$ cat > spn.yaml <<EOF
dependencies:
  @workflows/code-review: "^1.0"
  @workflows/seo-audit: "^1.2"
  @agents/researcher: "^2.0"
EOF

# Team members installent
$ spn install
✓ Installed 3 packages

# Tout le monde utilise les mêmes versions (spn.lock)
```

---

## 🏗️ Architecture (Simple)

```
~/.spn/packages/          ← GLOBAL (1 install, tous les projets)
  └── @workflows/seo-audit/1.2.0/workflow.nika.yaml

project/
├── spn.yaml              ← Manifest (comme package.json)
├── spn.lock              ← Versions lockées
└── .nika/
    ├── config.toml       ← Config (provider, permissions)
    ├── .cache/           ← Symlinks vers ~/.spn/packages/ (optionnel)
    └── workflows/        ← Fichiers locaux (coexistent avec packages)
        └── custom.nika.yaml
```

**Résolution de `nika run @workflows/name`:**
1. Cherche dans `.nika/workflows/`
2. Cherche dans `~/.spn/packages/`
3. Erreur si pas trouvé

---

## 🛠️ CLI Interactif (Nouveau en v0.7.0)

### spn add (sans args) → Menu interactif

```bash
$ spn add
? Type de package:
  ❯ Workflow
    Agent
    Prompt
    Job
    Skill (proxy skills.sh)
    MCP (proxy npm)

? Query: seo

🔍 Résultats:
  ❯ @workflows/seo-audit v1.2.0 ⭐ 45
    @agents/seo-researcher v2.0.1 ⭐ 32

? Installer? (Y/n) y
✓ Installé

? Run now? (Y/n) y
→ nika run @workflows/seo-audit
```

### nika init (avec templates)

```bash
$ nika init
? Type de projet:
  ❯ Empty
    Content Generation
    Code Automation
    Research Pipeline

? Starter workflows:
  [x] @workflows/content-generator
  [ ] @workflows/code-review

✓ Created .nika/
✓ Installed 1 workflow
→ Run: nika run content-generator
```

---

## 🎯 Objectif v0.7.0 (4 Semaines)

### Semaine 1: Fix Bugs
- Fix `spn add` tokio panic
- Fix `nika init` invalid examples

### Semaine 1-2: Package Resolution
- `nika run @workflows/name` fonctionne
- `nika run @agents/name` fonctionne
- `nika run @prompts/name` fonctionne (nouveau)
- `nika run @jobs/name` fonctionne

### Semaine 2-3: Include Support
- `include: { pkg: @workflows/name }` dans workflows
- Fusion de DAG depuis packages

### Semaine 3: Sync .nika/
- `spn install` crée symlinks dans `.nika/.cache/`
- Fast lookups

### Semaine 4: CLI Interactif
- `spn add` menu interactif
- `nika init` avec templates
- Tests + docs

---

## 📊 Métriques de Succès

| Métrique | Cible | Comment Mesurer |
|----------|-------|-----------------|
| **Time to First Workflow** | < 30 sec | `spn search → add → nika run` |
| **Commands to Productivity** | 3 | `search`, `add`, `run` |
| **Package Discovery** | Intuitive | Search trouve 90%+ des use cases |
| **Local + Global Coexist** | Seamless | Users peuvent mix sans friction |

**Si un dev peut aller de 0 à workflow qui tourne en 30 secondes, on a gagné.**

---

## 🧪 Validation Checklist

Avant de dire "v0.7.0 est prêt":

- [ ] `spn add @workflows/name` fonctionne sans panic
- [ ] `nika run @workflows/name` résout et exécute
- [ ] `nika run @agents/name` résout et exécute
- [ ] `nika run @prompts/name` résout et exécute
- [ ] `nika run @jobs/name` résout et exécute
- [ ] `include: { pkg: @workflows/name }` fonctionne
- [ ] Packages ET fichiers locaux coexistent
- [ ] `spn add` (interactif) guide l'utilisateur
- [ ] `nika init` (interactif) avec templates
- [ ] `spn install` crée symlinks dans `.nika/.cache/`
- [ ] Tests integration passent (100%)
- [ ] Docs utilisateur complètes

---

## 🤔 Questions à Thibaut (Validation Finale)

### 1. Architecture Storage

**Question:** On utilise symlinks dans `.nika/.cache/` ou Nika cherche directement dans `~/.spn/packages/`?

**Option A (avec symlinks):**
```
.nika/.cache/workflows/seo-audit -> ~/.spn/packages/@workflows/seo-audit/1.2.0/
```
✅ Avantages: Fast lookups, explicit links
❌ Inconvénients: +60 LOC, Windows symlinks

**Option B (sans symlinks):**
```
Nika resolve @workflows/name → cherche dans ~/.spn/packages/
```
✅ Avantages: Simple, cross-platform
❌ Inconvénients: Lookup à chaque run

**Ta préférence?**

### 2. CLI Interactif Priorité

**Question:** CLI interactif en S4 ou plus tard?

- **Maintenant (S4):** Meilleure UX, découverte guidée
- **Plus tard (v0.8.0):** Focus sur fonctionnalités core d'abord

**Ta préférence?**

### 3. @prompts Format

**Question:** Format exact pour `@prompts/` packages?

**Option A:** Markdown avec frontmatter (comme agents)
```markdown
---
name: seo-meta
version: 1.0.0
variables:
  - url
  - keywords
---

Generate SEO meta description for {{url}} with keywords: {{keywords}}
```

**Option B:** Plain text avec template syntax
```
Generate SEO meta description for {{url}} with keywords: {{keywords}}
```

**Ta préférence?**

### 4. Workflow Includes Priority

**Question:** `include: { pkg: @workflows/name }` en S2-S3 ou peut attendre v0.8.0?

- **Maintenant:** Composition de workflows depuis packages
- **Plus tard:** Users utilisent `nika run @workflows/name` en standalone d'abord

**Ta préférence?**

---

## 📝 Prochaines Étapes

1. **Toi (Thibaut):** Réponds aux 4 questions ci-dessus
2. **Moi (Claude):** J'ajuste le plan final
3. **Ensemble:** On commence l'implémentation (bugs d'abord)

---

**Vision claire? Plan clair? Allons-y! 🚀**
