use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCategory {
    Cache,
    Git,
    Workspace,
    Parse,
}

impl DiagnosticCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cache => "cache",
            Self::Git => "git",
            Self::Workspace => "workspace",
            Self::Parse => "parse",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub category: DiagnosticCategory,
    pub message: String,
}

impl Diagnostic {
    pub fn new(category: DiagnosticCategory, message: impl Into<String>) -> Self {
        Self {
            category,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Diagnostics {
    entries: Vec<Diagnostic>,
}

impl Diagnostics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn warn(&mut self, category: DiagnosticCategory, message: impl Into<String>) {
        self.entries.push(Diagnostic::new(category, message));
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.entries.iter()
    }

    pub fn into_entries(self) -> Vec<Diagnostic> {
        self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_collects_warnings() {
        let mut diagnostics = Diagnostics::new();
        diagnostics.warn(DiagnosticCategory::Cache, "failed to clear cache");
        assert!(!diagnostics.is_empty());

        let entries = diagnostics.into_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].category.as_str(), "cache");
        assert_eq!(entries[0].message, "failed to clear cache");
    }
}
