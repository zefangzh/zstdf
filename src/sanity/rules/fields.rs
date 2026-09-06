use crate::{
    parser::StdfRecord,
    sanity::{context::ValidationContext, finding::{Finding, Severity}, rule::RecordRule},
};

#[derive(Default)]
pub struct PtrLimitOrderRule;
impl RecordRule for PtrLimitOrderRule {
    fn name(&self) -> &'static str { "FIELD/PTR_LIMIT_ORDER" }
    fn on_record(&mut self, record: &StdfRecord, _ctx: &ValidationContext, offset: u64) -> Vec<Finding> {
        if let StdfRecord::Ptr(p) = record {
            if let (Some(lo), Some(hi)) = (p.lo_limit, p.hi_limit) {
                if !p.lo_limit_invalid() && !p.hi_limit_invalid() && lo > hi {
                    return vec![Finding::new(
                        Severity::Error, self.name(), Some("PTR"), Some(offset),
                        format!("test {} lo_limit {lo} > hi_limit {hi}", p.test_num))];
                }
            }
        }
        vec![]
    }
}

#[derive(Default)]
pub struct PtrResultRule;
impl RecordRule for PtrResultRule {
    fn name(&self) -> &'static str { "FIELD/PTR_RESULT_PRESENT" }
    fn on_record(&mut self, record: &StdfRecord, _ctx: &ValidationContext, offset: u64) -> Vec<Finding> {
        if let StdfRecord::Ptr(p) = record {
            if !p.result_invalid() && p.result.is_none() {
                return vec![Finding::new(
                    Severity::Warning, self.name(), Some("PTR"), Some(offset),
                    format!("test {} result_invalid bit=0 but RESULT field absent", p.test_num))];
            }
        }
        vec![]
    }
}

#[derive(Default)]
pub struct MirLotIdRule;
impl RecordRule for MirLotIdRule {
    fn name(&self) -> &'static str { "FIELD/MIR_LOT_ID" }
    fn on_record(&mut self, record: &StdfRecord, _ctx: &ValidationContext, offset: u64) -> Vec<Finding> {
        if let StdfRecord::Mir(m) = record {
            if m.lot_id.is_none() {
                return vec![Finding::new(
                    Severity::Warning, self.name(), Some("MIR"), Some(offset),
                    "MIR.LOT_ID is absent")];
            }
        }
        vec![]
    }
}
