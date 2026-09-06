#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity { Fatal, Error, Warning, Info }

#[derive(Debug, Clone)]
pub struct Finding {
    pub severity:    Severity,
    pub rule:        &'static str,
    pub record_type: Option<&'static str>,
    pub file_offset: Option<u64>,
    pub message:     String,
}
impl Finding {
    pub fn new(
        severity: Severity, rule: &'static str,
        record_type: Option<&'static str>, file_offset: Option<u64>,
        message: impl Into<String>,
    ) -> Self {
        Self { severity, rule, record_type, file_offset, message: message.into() }
    }
    pub fn fatal(rule: &'static str, msg: impl Into<String>) -> Self {
        Self::new(Severity::Fatal, rule, None, None, msg)
    }
}

#[derive(Debug, Default)]
pub struct ValidationReport { pub findings: Vec<Finding> }
impl ValidationReport {
    pub fn is_clean(&self) -> bool { self.findings.is_empty() }
    pub fn has_severity(&self, s: &Severity) -> bool {
        self.findings.iter().any(|f| &f.severity == s)
    }
    pub fn by_rule<'a>(&'a self, rule: &'static str) -> impl Iterator<Item=&'a Finding> {
        self.findings.iter().filter(move |f| f.rule == rule)
    }
    pub fn likely_aborted(&self) -> bool {
        self.findings.iter().any(|f| f.rule.starts_with("ABORT"))
    }
    pub fn summary(&self) -> String {
        let f = |s: &Severity| self.findings.iter().filter(|x| &x.severity == s).count();
        format!("{} fatal  {} error  {} warning  {} info",
            f(&Severity::Fatal), f(&Severity::Error),
            f(&Severity::Warning), f(&Severity::Info))
    }
}
