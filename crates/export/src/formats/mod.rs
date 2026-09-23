pub mod csv;
pub mod xlsx;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Xlsx,
    Csv,
}

impl ExportFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Xlsx => "xlsx",
            Self::Csv => "csv",
        }
    }

    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Xlsx => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            Self::Csv => "text/csv",
        }
    }
}
