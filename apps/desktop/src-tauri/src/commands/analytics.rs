use std::collections::HashMap;

use aditup_db::repositories::observations::ObservationsRepository;
use aditup_db::repositories::stage2_assessments::{RiskLevel, Stage2AssessmentsRepository};
use aditup_db::repositories::stage3_matches::{Stage3MatchStatus, Stage3MatchesRepository};
use serde::Serialize;
use tauri::State;

use crate::state::AppState;

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskLevelCounts {
    pub low: usize,
    pub medium: usize,
    pub high: usize,
    pub critical: usize,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stage3StatusCounts {
    pub matched: usize,
    pub written: usize,
    pub escalated: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LabeledCount {
    pub label: String,
    pub count: usize,
}

/// Module 10 (part 6/7) — Analytics. Everything here is derived from the
/// same three `list_for_audit` queries Stage 2/3 already use; no new
/// repository methods or aggregation tables were needed. This is
/// single-audit analytics (the "Audit finalized" screen's dashboard), not
/// the cross-audit/cross-project reporting the blueprint leaves for later.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditAnalyticsDto {
    pub audit_id: String,
    pub total_observations: usize,
    pub assessed_count: usize,
    pub risk_levels: RiskLevelCounts,
    /// Top 8 categories/departments by observation count, most-common first.
    /// Capped rather than exhaustive since a long tail of one-off values
    /// wouldn't read as a useful chart.
    pub top_categories: Vec<LabeledCount>,
    pub top_departments: Vec<LabeledCount>,
    pub stage3_status: Stage3StatusCounts,
    /// `None` when no `stage3_matches` row has a `match_score` yet (nothing
    /// selected from suggestions, or every card was written/escalated).
    pub average_match_score: Option<f64>,
    /// (matched + written) / total_observations — the Stage 3 completion
    /// gate's own definition of "resolved" (see `Stage3MatchesRepository`).
    pub resolution_rate: f64,
}

fn top_n(counts: HashMap<String, usize>, n: usize) -> Vec<LabeledCount> {
    let mut rows: Vec<LabeledCount> = counts
        .into_iter()
        .map(|(label, count)| LabeledCount { label, count })
        .collect();
    rows.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.label.cmp(&b.label)));
    rows.truncate(n);
    rows
}

#[tauri::command]
pub fn get_audit_analytics(
    audit_id: String,
    state: State<AppState>,
) -> Result<AuditAnalyticsDto, String> {
    state.with_db(|db| {
        let observations = ObservationsRepository::new(db)
            .list_for_audit(&audit_id)
            .map_err(|e| e.to_string())?;
        let assessments = Stage2AssessmentsRepository::new(db)
            .list_for_audit(&audit_id)
            .map_err(|e| e.to_string())?;
        let matches = Stage3MatchesRepository::new(db)
            .list_for_audit(&audit_id)
            .map_err(|e| e.to_string())?;

        let total_observations = observations.len();

        let mut risk_levels = RiskLevelCounts::default();
        let mut category_counts: HashMap<String, usize> = HashMap::new();
        let mut department_counts: HashMap<String, usize> = HashMap::new();
        for assessment in &assessments {
            match assessment.risk_level {
                RiskLevel::Low => risk_levels.low += 1,
                RiskLevel::Medium => risk_levels.medium += 1,
                RiskLevel::High => risk_levels.high += 1,
                RiskLevel::Critical => risk_levels.critical += 1,
            }
            if let Some(category) = &assessment.category {
                *category_counts.entry(category.clone()).or_insert(0) += 1;
            }
            if let Some(department) = &assessment.department {
                *department_counts.entry(department.clone()).or_insert(0) += 1;
            }
        }

        let mut stage3_status = Stage3StatusCounts::default();
        let mut score_sum = 0.0;
        let mut score_count = 0usize;
        for m in &matches {
            match m.status {
                Stage3MatchStatus::Matched => stage3_status.matched += 1,
                Stage3MatchStatus::Written => stage3_status.written += 1,
                Stage3MatchStatus::Escalated => stage3_status.escalated += 1,
            }
            if let Some(score) = m.match_score {
                score_sum += score;
                score_count += 1;
            }
        }
        let average_match_score = if score_count > 0 {
            Some(score_sum / score_count as f64)
        } else {
            None
        };

        let resolved = stage3_status.matched + stage3_status.written;
        let resolution_rate = if total_observations > 0 {
            resolved as f64 / total_observations as f64
        } else {
            0.0
        };

        Ok(AuditAnalyticsDto {
            audit_id,
            total_observations,
            assessed_count: assessments.len(),
            risk_levels,
            top_categories: top_n(category_counts, 8),
            top_departments: top_n(department_counts, 8),
            stage3_status,
            average_match_score,
            resolution_rate,
        })
    })
}
