-- Module 2 — full schema per docs/architecture/phase-1-blueprint.md §6.
-- Knowledge banks are created here (schema is a single stable unit) even
-- though their repositories/business logic belong to later modules
-- (4 Observation Bank, 5 Recommendation Bank, 6 Keyword Engine, 9 KLE).

-- ===================== Identity & Scope =====================

CREATE TABLE users (
  id TEXT PRIMARY KEY,
  username TEXT NOT NULL UNIQUE,
  password_hash TEXT NOT NULL,
  role TEXT NOT NULL CHECK (role IN ('auditor', 'admin')),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE projects (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  client TEXT,
  encryption_key_id TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  archived_at TEXT
);

CREATE TABLE sites (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  location TEXT
);
CREATE INDEX idx_sites_project_id ON sites (project_id);

CREATE TABLE audits (
  id TEXT PRIMARY KEY,
  site_id TEXT NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
  auditor_user_id TEXT NOT NULL REFERENCES users (id),
  audit_date TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('draft', 'stage2', 'stage3', 'finalized')) DEFAULT 'draft',
  scope_notes TEXT
);
CREATE INDEX idx_audits_site_id ON audits (site_id);
CREATE INDEX idx_audits_status ON audits (status);

-- ===================== Knowledge Banks =====================
-- (project-independent, versioned reference data — defined before the
-- observation pipeline tables below so stage2_assessments can reference
-- legal_bank without a forward declaration)

-- `bank_key` groups rows into the topical dataset they were published from
-- (e.g. one distinct value per imported standard: fire extinguishers, fire
-- hydrants, sprinklers, ISO 45001, NBC Part F, gas cylinder rules...). It is
-- what `official_meta.bank_key` versions — a *dataset's* published version,
-- not any single row's. `id` is the row's own identity within that dataset.
CREATE TABLE observation_bank (
  id TEXT PRIMARY KEY,
  bank_key TEXT NOT NULL,
  topic TEXT NOT NULL,
  label TEXT NOT NULL,
  text TEXT NOT NULL,
  version INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX idx_observation_bank_bank_key ON observation_bank (bank_key);
CREATE INDEX idx_observation_bank_topic ON observation_bank (topic);
CREATE INDEX idx_observation_bank_label ON observation_bank (label);

CREATE TABLE recommendation_bank (
  id TEXT PRIMARY KEY,
  observation_bank_id TEXT REFERENCES observation_bank (id) ON DELETE CASCADE,
  topic TEXT NOT NULL,
  text TEXT NOT NULL,
  version INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX idx_recommendation_bank_observation_bank_id ON recommendation_bank (observation_bank_id);

CREATE TABLE legal_bank (
  id TEXT PRIMARY KEY,
  standard TEXT NOT NULL,
  clause TEXT NOT NULL,
  text TEXT NOT NULL,
  version INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX idx_legal_bank_standard ON legal_bank (standard);

CREATE TABLE keyword_bank (
  id TEXT PRIMARY KEY,
  term TEXT NOT NULL UNIQUE,
  contexts TEXT NOT NULL DEFAULT '[]',
  weight REAL NOT NULL DEFAULT 1.0
);

CREATE TABLE synonym_bank (
  id TEXT PRIMARY KEY,
  term_a TEXT NOT NULL,
  term_b TEXT NOT NULL,
  confidence REAL NOT NULL DEFAULT 1.0,
  UNIQUE (term_a, term_b)
);

CREATE TABLE official_meta (
  bank_key TEXT PRIMARY KEY,
  current_version INTEGER NOT NULL DEFAULT 1,
  published_at TEXT
);

-- ===================== Observation Pipeline =====================

CREATE TABLE observations (
  id TEXT PRIMARY KEY,
  audit_id TEXT NOT NULL REFERENCES audits (id) ON DELETE CASCADE,
  entry_hash TEXT NOT NULL,
  text TEXT NOT NULL,
  location TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_observations_audit_id ON observations (audit_id);
CREATE INDEX idx_observations_entry_hash ON observations (entry_hash);

CREATE TABLE evidence (
  id TEXT PRIMARY KEY,
  observation_id TEXT NOT NULL REFERENCES observations (id) ON DELETE CASCADE,
  encrypted_blob_ref TEXT NOT NULL,
  mime TEXT NOT NULL,
  size INTEGER NOT NULL
);
CREATE INDEX idx_evidence_observation_id ON evidence (observation_id);

CREATE TABLE stage2_assessments (
  id TEXT PRIMARY KEY,
  observation_id TEXT NOT NULL UNIQUE REFERENCES observations (id) ON DELETE CASCADE,
  risk_level TEXT NOT NULL CHECK (risk_level IN ('low', 'medium', 'high', 'critical')),
  category TEXT,
  department TEXT,
  equipment TEXT,
  legal_clause_id TEXT REFERENCES legal_bank (id)
);

CREATE TABLE stage3_matches (
  id TEXT PRIMARY KEY,
  observation_id TEXT NOT NULL REFERENCES observations (id) ON DELETE CASCADE,
  bank_key TEXT,
  matched_row_id TEXT,
  match_score REAL,
  status TEXT NOT NULL CHECK (status IN ('matched', 'written', 'escalated')) DEFAULT 'escalated',
  final_text TEXT
);
CREATE INDEX idx_stage3_matches_observation_id ON stage3_matches (observation_id);

-- ===================== Self-Learning Engine (KLE) =====================

CREATE TABLE kle_pending_observations (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  status TEXT NOT NULL CHECK (status IN ('pending', 'approved', 'rejected', 'merged')) DEFAULT 'pending',
  bank_key_guess TEXT,
  source_entry_hash TEXT NOT NULL,
  payload TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_kle_pending_obs_status ON kle_pending_observations (status);
CREATE INDEX idx_kle_pending_obs_bank_key_guess ON kle_pending_observations (bank_key_guess);
CREATE INDEX idx_kle_pending_obs_source_entry_hash ON kle_pending_observations (source_entry_hash);

CREATE TABLE kle_pending_keywords (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  term TEXT NOT NULL,
  contexts TEXT NOT NULL DEFAULT '[]',
  frequency_observed INTEGER NOT NULL DEFAULT 1,
  status TEXT NOT NULL CHECK (status IN ('pending', 'approved', 'rejected')) DEFAULT 'pending'
);

CREATE TABLE kle_pending_synonyms (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  term_a TEXT NOT NULL,
  term_b TEXT NOT NULL,
  confidence REAL NOT NULL DEFAULT 1.0,
  status TEXT NOT NULL CHECK (status IN ('pending', 'approved', 'rejected')) DEFAULT 'pending'
);

CREATE TABLE kle_merge_requests (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  candidate_a_id INTEGER NOT NULL,
  candidate_b_id INTEGER NOT NULL,
  similarity REAL NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('pending', 'merged', 'kept_both', 'discarded')) DEFAULT 'pending'
);

CREATE TABLE kle_history_search (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  entry_hash TEXT NOT NULL,
  timestamp TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  query TEXT NOT NULL
);
CREATE INDEX idx_kle_hist_search_entry_hash ON kle_history_search (entry_hash);
CREATE INDEX idx_kle_hist_search_timestamp ON kle_history_search (timestamp);

CREATE TABLE kle_history_selection (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  entry_hash TEXT NOT NULL,
  timestamp TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  chosen_bank_key TEXT
);
CREATE INDEX idx_kle_hist_selection_entry_hash ON kle_history_selection (entry_hash);
CREATE INDEX idx_kle_hist_selection_timestamp ON kle_history_selection (timestamp);

CREATE TABLE kle_history_rejection (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  entry_hash TEXT NOT NULL,
  timestamp TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  rejected_bank_key TEXT
);
CREATE INDEX idx_kle_hist_rejection_entry_hash ON kle_history_rejection (entry_hash);
CREATE INDEX idx_kle_hist_rejection_timestamp ON kle_history_rejection (timestamp);

CREATE TABLE kle_history_edit (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  entry_hash TEXT NOT NULL,
  timestamp TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  before TEXT,
  after TEXT
);
CREATE INDEX idx_kle_hist_edit_entry_hash ON kle_history_edit (entry_hash);
CREATE INDEX idx_kle_hist_edit_timestamp ON kle_history_edit (timestamp);

CREATE TABLE kle_history_approval (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  entry_hash TEXT NOT NULL,
  timestamp TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  admin_user_id TEXT
);
CREATE INDEX idx_kle_hist_approval_entry_hash ON kle_history_approval (entry_hash);
CREATE INDEX idx_kle_hist_approval_timestamp ON kle_history_approval (timestamp);

CREATE TABLE kle_history_confidence (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  entry_hash TEXT NOT NULL,
  timestamp TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  breakdown_json TEXT NOT NULL
);
CREATE INDEX idx_kle_hist_confidence_entry_hash ON kle_history_confidence (entry_hash);
CREATE INDEX idx_kle_hist_confidence_timestamp ON kle_history_confidence (timestamp);

-- ===================== Platform =====================

CREATE TABLE admin_settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE TABLE audit_log (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  timestamp TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  actor_user_id TEXT,
  action TEXT NOT NULL,
  payload_hash TEXT NOT NULL,
  prev_hash TEXT
);
