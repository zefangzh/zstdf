pub mod context;
pub mod finding;
pub mod rule;
pub mod rules;

use crate::{
    parser::StdfRecord,
    sanity::{context::ValidationContext, finding::ValidationReport, rule::RecordRule},
};

pub struct ValidationEngine {
    rules:   Vec<Box<dyn RecordRule>>,
    context: ValidationContext,
    report:  ValidationReport,
}

impl ValidationEngine {
    pub fn new(rules: Vec<Box<dyn RecordRule>>) -> Self {
        Self {
            rules,
            context: ValidationContext::default(),
            report:  ValidationReport::default(),
        }
    }

    pub fn feed(&mut self, record: &StdfRecord, offset: u64) {
        for rule in &mut self.rules {
            let findings = rule.on_record(record, &self.context, offset);
            self.report.findings.extend(findings);
        }
        self.context.update(record, offset);
    }

    pub fn finish(mut self) -> ValidationReport {
        for rule in &mut self.rules {
            let findings = rule.on_eof(&self.context);
            self.report.findings.extend(findings);
        }
        self.report
    }
}

pub fn full_engine() -> ValidationEngine {
    use rules::{abort::AbortDetectionRule, fields::*, semantic::*, structural::*};
    ValidationEngine::new(vec![
        Box::new(PirPrrPairRule::default()),
        Box::new(WirWrrPairRule::default()),
        Box::new(BpsEpsBalanceRule::default()),
        Box::new(MirSingletonRule::default()),
        Box::new(AbortDetectionRule::default()),
        Box::new(PtrLimitOrderRule::default()),
        Box::new(PtrResultRule::default()),
        Box::new(MirLotIdRule::default()),
        Box::new(TestNameConsistencyRule::default()),
        Box::new(HbrBinCountRule::default()),
    ])
}
