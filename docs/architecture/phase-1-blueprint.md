# ADITUP — Phase 1 Architecture Blueprint

**Status:** Approved for implementation (Phase 2)
**Scope:** Mode 1 — Rule-based + Self-Learning Engine (no LLM)

This is the reference architecture Phase 2 implementation must follow. The full
narrative version (with diagrams) was delivered as a standalone document; this
file is the durable, in-repo copy engineering should treat as source of truth
for module boundaries, schema, and the Mode 2/3 extension seam.

## Module map (build order)

1. Project setup
2. Database (encrypted SQLite / SQLCipher)
3. Authentication (Argon2id, session vault, Admin elevation)
4. Observation Bank
5. Recommendation Bank
6. Keyword Engine (keyword/synonym bank, preprocessing, TF-IDF)
7. Search Engine (inverted index, fuzzy search, ranking, confidence)
8. Rule-Based Engine (`recommendation-provider` trait + Mode-1 impl)
9. Self-Learning Engine / KLE (staging, clustering, feedback loop)
10. UI (Stage 1/2/3, Admin console, Analytics)
11. Export Engine (XLSX/PPTX/PDF/CSV)
12. Backup & Restore
13. Security hardening (audit log hash-chain, project keys)
14. Settings (thresholds, weights, theme, mode switch)
15. Testing
16. Packaging

## System architecture

Layered, offline-first, desktop shell (Tauri) wrapping a Rust backend and a
React/TypeScript renderer:

- **Presentation** — Tauri window, React UI components, view-layer state.
- **Application/Domain** — Workflow engine (Stage 1→2→3 state machine),
  Scoring & Ranking engine, Self-Learning Engine (KLE), and the
  `recommendation-provider` interface Stage 3 depends on (Mode 1 today; Modes
  2/3 later implement the same interface — see below).
- **Data Access** — Repositories (one per aggregate), migration runner, export
  engine.
- **Storage** — Encrypted SQLite (SQLCipher) file, encrypted evidence blob
  store, separately-keyed backup bundles.
- **Security envelope** — wraps every layer: Argon2id auth + session vault,
  AES-256 encryption service, hash-chained append-only audit log.

Rust owns everything that touches the filesystem, crypto, or the database
(db, security, scoring-engine, kle, recommendation-provider, export crates in
a Cargo workspace). TypeScript/React owns presentation only (`ui`, `domain`
view-state, `shared-types` IPC contracts) and talks to Rust exclusively
through typed Tauri commands — the renderer is never a trusted boundary.

## Database design (summary)

`projects → sites → audits → observations → {evidence, stage2_assessments,
stage3_matches}`. Knowledge banks (`observation_bank`, `recommendation_bank`,
`legal_bank`, `keyword_bank`, `synonym_bank`, `official_meta`) are
project-independent, versioned reference data. KLE staging/history tables
(`kle_pending_*`, `kle_history_*`) link back to observations via a stable
`entry_hash` (`hash(location + text)`), not a surrogate FK, so history survives
export/import and cross-machine merge. `admin_settings` and `audit_log`
(hash-chained) round out the platform tables.

## Search & ranking (summary)

Inverted keyword index + fuzzy/edit-distance pass, combined into a weighted
score: keyword overlap (0.30), TF-IDF term weight (0.25), fuzzy similarity
(0.15), synonym contribution (0.10), category/department/equipment match
(0.20). Confidence badge = rescaled top score, banded `<50% low / 50-74%
medium / ≥75% high`; below a novelty threshold (default 40%) the system stops
suggesting and routes the entry into KLE staging instead.

## Security (summary)

Argon2id password hashing (replaces the ADITUP prototype's unsalted PIN
hash), SQLCipher AES-256 at rest (replaces plain IndexedDB), separate Admin
elevation gate, per-project optional encryption keys, separately-keyed backup
bundles, hash-chained audit log for every publish/merge/delete, no backdoor
password reset (one-time recovery key issued at setup).

## Mode 2 / Mode 3 extension seam

Stage 3 never calls the scoring engine directly — it calls the
`recommendation-provider` interface. Mode 1 (this build) implements it with
the rule-based engine above. Mode 2 (cloud agentic RAG) and Mode 3 (local LLM
agentic RAG) implement the same interface against the same knowledge-bank
tables (which double as the RAG corpus) and the same KLE history tables
(which double as feedback/fine-tuning signal). Switching modes is a Settings
change, never a data migration.

---

_Original planning-phase deliverable produced 2026-07-03; this file is the
condensed in-repo copy used to keep Phase 2 implementation aligned with the
approved design._
