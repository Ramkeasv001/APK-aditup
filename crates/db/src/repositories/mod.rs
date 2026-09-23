//! One repository module per aggregate root: the Stage 1 critical path
//! (projects, sites, audits, observations), users and admin_settings
//! (Module 3, storage only — hashing/verification lives in aditup-security),
//! observation_bank (Module 4), recommendation_bank (Module 5),
//! keyword_bank/synonym_bank (Module 6, storage only — preprocessing and
//! TF-IDF math live in aditup-scoring-engine), legal_bank (Module 8,
//! consumed by aditup-recommendation-provider's clause-matching), and the
//! KLE staging/merge/history tables (Module 9, storage only — staging
//! decisions, clustering, and feedback-weight adjustment live in
//! aditup-kle), stage2_assessments (Module 10d), and stage3_matches
//! (Module 10e). `official_meta` is shared infrastructure the bank modules
//! publish through, not owned by any one of them.

pub mod admin_settings;
pub mod audits;
pub mod keyword_bank;
pub mod kle_history;
pub mod kle_merge_requests;
pub mod kle_pending_observations;
pub mod kle_pending_terms;
pub mod legal_bank;
pub mod observation_bank;
pub mod observations;
pub mod official_meta;
pub mod projects;
pub mod recommendation_bank;
pub mod sites;
pub mod stage2_assessments;
pub mod stage3_matches;
pub mod synonym_bank;
pub mod users;
