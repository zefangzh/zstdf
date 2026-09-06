use std::collections::HashMap;
use crate::{
    parser::StdfRecord,
    sanity::{context::ValidationContext, finding::{Finding, Severity}, rule::RecordRule},
};

#[derive(Default)]
pub struct TestNameConsistencyRule { seen: HashMap<u32, (String, u64)> }
impl RecordRule for TestNameConsistencyRule {
    fn name(&self) -> &'static str { "SEMANTIC/TEST_NAME_CONSISTENCY" }
    fn on_record(&mut self, record: &StdfRecord, _ctx: &ValidationContext, offset: u64) -> Vec<Finding> {
        if let StdfRecord::Ptr(p) = record {
            let name = p.test_txt.clone().unwrap_or_default();
            if let Some((prev_name, prev_off)) = self.seen.get(&p.test_num) {
                if *prev_name != name {
                    return vec![Finding::new(
                        Severity::Warning, self.name(), Some("PTR"), Some(offset),
                        format!(
                            "test_num {} has name {:?} here (prev {:?} at {prev_off:#010x})",
                            p.test_num, name, prev_name))];
                }
            } else {
                self.seen.insert(p.test_num, (name, offset));
            }
        }
        vec![]
    }
}

#[derive(Default)]
pub struct HbrBinCountRule;
impl RecordRule for HbrBinCountRule {
    fn name(&self) -> &'static str { "SEMANTIC/HBR_BIN_COUNT" }
    fn on_record(&mut self, _: &StdfRecord, _: &ValidationContext, _: u64) -> Vec<Finding> { vec![] }
    fn on_eof(&mut self, ctx: &ValidationContext) -> Vec<Finding> {
        let mut out = vec![];

        // hb_counts_hbr is keyed by (head, site, bin). Per the STDF spec,
        // HEAD_NUM/SITE_NUM == 255 marks an "ALL heads"/"ALL sites"
        // rollup record. Some testers emit such a rollup *in addition to*
        // the true per-site breakdown for the same bin — that rollup
        // represents the *same* parts, not additional ones, so it must
        // not be summed together with the granular per-site records
        // (which would double-count). We therefore prefer the granular
        // breakdown when present, and only fall back to the rollup
        // value (deduplicated via `max`, since two rollups for the same
        // bin are almost certainly the same total reported at different
        // aggregation levels) when no granular records exist for a bin.
        let mut granular_by_bin: HashMap<u16, u64> = HashMap::new();
        let mut aggregate_by_bin: HashMap<u16, u64> = HashMap::new();
        for (&(head, site, bin), &cnt) in &ctx.hb_counts_hbr {
            if head == 255 || site == 255 {
                let entry = aggregate_by_bin.entry(bin).or_default();
                *entry = (*entry).max(cnt);
            } else {
                *granular_by_bin.entry(bin).or_default() += cnt;
            }
        }
        let mut hbr_by_bin: HashMap<u16, u64> = HashMap::new();
        for &bin in granular_by_bin.keys().chain(aggregate_by_bin.keys()) {
            hbr_by_bin.entry(bin).or_insert_with(|| {
                granular_by_bin.get(&bin).copied()
                    .unwrap_or_else(|| aggregate_by_bin.get(&bin).copied().unwrap_or(0))
            });
        }

        for (&bin, &hbr_cnt) in &hbr_by_bin {
            let prr_cnt = ctx.hb_counts_prr.get(&bin).copied().unwrap_or(0);
            if hbr_cnt != prr_cnt {
                out.push(Finding::new(
                    Severity::Warning, self.name(), Some("HBR"), None,
                    format!("bin {bin} HBR says {hbr_cnt} but PRR count is {prr_cnt}")));
            }
        }
        for (&bin, &prr_cnt) in &ctx.hb_counts_prr {
            if !hbr_by_bin.contains_key(&bin) {
                out.push(Finding::new(
                    Severity::Warning, self.name(), Some("HBR"), None,
                    format!("bin {bin} appears in PRRs ({prr_cnt} times) but has no HBR record")));
            }
        }
        out
    }
}
