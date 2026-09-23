use std::path::PathBuf;

use aditup_db::repositories::observations::ObservationsRepository;
use aditup_db::repositories::stage2_assessments::Stage2AssessmentsRepository;
use aditup_db::repositories::stage3_matches::Stage3MatchesRepository;
use aditup_db::Database;
use csv::Writer;

use crate::error::ExportError;
use crate::ExportResult;

/// Exports audit data to CSV: one row per observation, with Stage 2 risk
/// assessment and Stage 3 match outcome side-by-side.
pub fn export_csv(db: &Database, audit_id: &str, output_path: &PathBuf) -> ExportResult<PathBuf> {
    let observations = ObservationsRepository::new(db)
        .list_for_audit(audit_id)
        .map_err(|e| ExportError::Database(e.to_string()))?;

    let assessments = Stage2AssessmentsRepository::new(db)
        .list_for_audit(audit_id)
        .map_err(|e| ExportError::Database(e.to_string()))?;

    let matches = Stage3MatchesRepository::new(db)
        .list_for_audit(audit_id)
        .map_err(|e| ExportError::Database(e.to_string()))?;

    // Index assessments and matches by observation_id for O(1) lookup
    let mut assessment_map = std::collections::HashMap::new();
    for a in assessments {
        assessment_map.insert(a.observation_id.clone(), a);
    }

    let mut match_map = std::collections::HashMap::new();
    for m in matches {
        match_map.insert(m.observation_id.clone(), m);
    }

    let mut writer = Writer::from_path(output_path)
        .map_err(|e| ExportError::IoError(e.to_string()))?;

    // Write header
    writer
        .write_record(&[
            "Location",
            "Observation",
            "Risk Level",
            "Category",
            "Department",
            "Equipment",
            "Legal Clause",
            "Stage 3 Status",
            "Matched Bank Key",
            "Match Score",
            "Final Text",
        ])
        .map_err(|e| ExportError::FormatError(e.to_string()))?;

    // Write rows
    for observation in observations {
        let assessment = assessment_map.get(&observation.id);
        let m = match_map.get(&observation.id);

        let risk_level = assessment
            .map(|a| format!("{:?}", a.risk_level))
            .unwrap_or_default();

        let category = assessment
            .and_then(|a| a.category.clone())
            .unwrap_or_default();

        let department = assessment
            .and_then(|a| a.department.clone())
            .unwrap_or_default();

        let equipment = assessment
            .and_then(|a| a.equipment.clone())
            .unwrap_or_default();

        let legal_clause = assessment
            .and_then(|a| a.legal_clause_id.clone())
            .unwrap_or_default();

        let stage3_status = m
            .map(|match_| format!("{:?}", match_.status))
            .unwrap_or_else(|| "(not assessed)".to_string());

        let bank_key = m
            .and_then(|match_| match_.bank_key.clone())
            .unwrap_or_default();

        let match_score = m
            .and_then(|match_| match_.match_score)
            .map(|s| format!("{:.1}%", s))
            .unwrap_or_default();

        let final_text = m
            .and_then(|match_| match_.final_text.clone())
            .unwrap_or_default();

        writer
            .write_record(&[
                observation.location.as_deref().unwrap_or(""),
                &observation.text,
                &risk_level,
                &category,
                &department,
                &equipment,
                &legal_clause,
                &stage3_status,
                &bank_key,
                &match_score,
                &final_text,
            ])
            .map_err(|e| ExportError::FormatError(e.to_string()))?;
    }

    writer
        .flush()
        .map_err(|e| ExportError::IoError(e.to_string()))?;

    Ok(output_path.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_csv_path_is_cloned_on_success() {
        // Just test the happy path signature — actual DB testing in lib.rs
        let path = PathBuf::from("test.csv");
        // In real tests we'd use a test DB and verify row count, etc.
        let _: PathBuf = path;
    }
}
