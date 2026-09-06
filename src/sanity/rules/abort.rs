use std::collections::HashMap;
use crate::{
    parser::StdfRecord,
    sanity::{context::ValidationContext, finding::{Finding, Severity}, rule::RecordRule},
};

#[derive(Default)]
pub struct AbortDetectionRule {
    mrr_seen:       bool,
    open_wir_count: u32,
    /// Open PIRs keyed by (head_num, site_num) → offset where they opened.
    /// A single bool would let one site's PRR mask another site's PIR that
    /// never closed, hiding aborts in multi-site files.
    open_pir:       HashMap<(u8, u8), u64>,
    bps_depth:      u32,
    last_offset:    u64,
}

impl RecordRule for AbortDetectionRule {
    fn name(&self) -> &'static str { "ABORT" }

    fn on_record(&mut self, record: &StdfRecord, _ctx: &ValidationContext, offset: u64) -> Vec<Finding> {
        self.last_offset = offset;
        match record {
            StdfRecord::Mrr(_) => self.mrr_seen = true,
            StdfRecord::Wir(_) => self.open_wir_count += 1,
            StdfRecord::Wrr(_) => self.open_wir_count = self.open_wir_count.saturating_sub(1),
            StdfRecord::Pir(p) => { self.open_pir.insert((p.head_num, p.site_num), offset); }
            StdfRecord::Prr(p) => { self.open_pir.remove(&(p.head_num, p.site_num)); }
            StdfRecord::Bps(_) => self.bps_depth += 1,
            StdfRecord::Eps(_) => self.bps_depth = self.bps_depth.saturating_sub(1),
            _ => {}
        }
        vec![]
    }

    fn on_eof(&mut self, _ctx: &ValidationContext) -> Vec<Finding> {
        let mut out = vec![];
        if !self.mrr_seen {
            out.push(Finding::new(Severity::Warning, "ABORT/NO_MRR", None, None,
                "No MRR found — writer likely aborted; bin/lot summary may be incomplete."));
        }
        if self.open_wir_count > 0 {
            out.push(Finding::new(Severity::Error, "ABORT/OPEN_WIR", Some("WIR"),
                Some(self.last_offset),
                format!("{} WIR(s) unclosed by WRR — aborted mid-wafer", self.open_wir_count)));
        }
        if !self.open_pir.is_empty() {
            let mut leftovers: Vec<_> = self.open_pir.iter().collect();
            leftovers.sort_by_key(|&(_, &off)| off);
            for (&(head, site), &off) in leftovers {
                out.push(Finding::new(Severity::Error, "ABORT/OPEN_PIR", Some("PIR"),
                    Some(off),
                    format!("PIR for head={head} site={site} never closed by PRR — aborted during part test")));
            }
        }
        if self.bps_depth > 0 {
            out.push(Finding::new(Severity::Warning, "ABORT/OPEN_BPS", Some("BPS"), None,
                format!("{} BPS section(s) not closed by EPS", self.bps_depth)));
        }
        out
    }
}
