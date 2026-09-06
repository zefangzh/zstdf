use std::collections::HashMap;
use crate::{
    parser::StdfRecord,
    sanity::{context::ValidationContext, finding::{Finding, Severity}, rule::RecordRule},
};

/// Tracks open PIRs per (head_num, site_num) so that multi-site files —
/// where several sites issue PIR before any of them issues its matching
/// PRR (e.g. PIR/PIR then PRR/PRR) — are not flagged as errors.
#[derive(Default)]
pub struct PirPrrPairRule { open: HashMap<(u8, u8), u64> }
impl RecordRule for PirPrrPairRule {
    fn name(&self) -> &'static str { "STRUCT/PIR_PRR_PAIR" }
    fn on_record(&mut self, record: &StdfRecord, _ctx: &ValidationContext, offset: u64) -> Vec<Finding> {
        match record {
            StdfRecord::Pir(p) => {
                let key = (p.head_num, p.site_num);
                if let Some(prev) = self.open.insert(key, offset) {
                    return vec![Finding::new(Severity::Error, self.name(), Some("PIR"), Some(offset),
                        format!(
                            "PIR at {offset:#010x} for head={} site={} opened while previous PIR at {prev:#010x} for the same site never closed",
                            p.head_num, p.site_num))];
                }
            }
            StdfRecord::Prr(p) => {
                let key = (p.head_num, p.site_num);
                if self.open.remove(&key).is_none() {
                    return vec![Finding::new(Severity::Error, self.name(), Some("PRR"), Some(offset),
                        format!(
                            "PRR at {offset:#010x} for head={} site={} has no matching PIR",
                            p.head_num, p.site_num))];
                }
            }
            _ => {}
        }
        vec![]
    }
    fn on_eof(&mut self, _ctx: &ValidationContext) -> Vec<Finding> {
        let mut leftovers: Vec<_> = self.open.iter().collect();
        leftovers.sort_by_key(|&(_, &off)| off);
        leftovers
            .into_iter()
            .map(|(&(head, site), &off)| Finding::new(
                Severity::Error, self.name(), Some("PIR"), Some(off),
                format!(
                    "PIR at {off:#010x} for head={head} site={site} never closed — file likely truncated mid-part")
            ))
            .collect()
    }
}

/// Tracks open WIRs per (head_num, site_grp) rather than a plain stack, so
/// a WRR for one head cannot accidentally "close" a WIR opened on a
/// different head (which a LIFO stack would silently allow whenever heads
/// interleave their WIR/WRR pairs, e.g. head 1 opens, head 2 opens, head 1
/// closes — a stack would incorrectly pop head 2's WIR).
#[derive(Default)]
pub struct WirWrrPairRule { open: HashMap<(u8, u8), u64> }
impl RecordRule for WirWrrPairRule {
    fn name(&self) -> &'static str { "STRUCT/WIR_WRR_PAIR" }
    fn on_record(&mut self, record: &StdfRecord, _ctx: &ValidationContext, offset: u64) -> Vec<Finding> {
        match record {
            StdfRecord::Wir(w) => {
                let key = (w.head_num, w.site_grp);
                if let Some(prev) = self.open.insert(key, offset) {
                    return vec![Finding::new(Severity::Error, self.name(), Some("WIR"), Some(offset),
                        format!(
                            "WIR at {offset:#010x} for head={} site_grp={} opened while previous WIR at {prev:#010x} for the same head/site_grp never closed",
                            w.head_num, w.site_grp))];
                }
            }
            StdfRecord::Wrr(w) => {
                let key = (w.head_num, w.site_grp);
                if self.open.remove(&key).is_none() {
                    return vec![Finding::new(Severity::Error, self.name(), Some("WRR"), Some(offset),
                        format!(
                            "WRR at {offset:#010x} for head={} site_grp={} has no matching WIR",
                            w.head_num, w.site_grp))];
                }
            }
            _ => {}
        }
        vec![]
    }
    fn on_eof(&mut self, _ctx: &ValidationContext) -> Vec<Finding> {
        let mut leftovers: Vec<_> = self.open.iter().collect();
        leftovers.sort_by_key(|&(_, &off)| off);
        leftovers
            .into_iter()
            .map(|(&(head, site_grp), &off)| Finding::new(
                Severity::Error, self.name(), Some("WIR"), Some(off),
                format!("WIR at {off:#010x} for head={head} site_grp={site_grp} never closed by WRR")
            ))
            .collect()
    }
}

#[derive(Default)]
pub struct BpsEpsBalanceRule { depth: u32, first_open: Option<u64> }
impl RecordRule for BpsEpsBalanceRule {
    fn name(&self) -> &'static str { "STRUCT/BPS_EPS_BALANCE" }
    fn on_record(&mut self, record: &StdfRecord, _ctx: &ValidationContext, offset: u64) -> Vec<Finding> {
        match record {
            StdfRecord::Bps(_) => {
                self.depth += 1;
                self.first_open.get_or_insert(offset);
            }
            StdfRecord::Eps(_) => {
                if self.depth == 0 {
                    return vec![Finding::new(Severity::Error, self.name(), Some("EPS"), Some(offset),
                        "EPS without matching BPS".to_string())];
                }
                self.depth -= 1;
                if self.depth == 0 { self.first_open = None; }
            }
            _ => {}
        }
        vec![]
    }
    fn on_eof(&mut self, _ctx: &ValidationContext) -> Vec<Finding> {
        if self.depth > 0 {
            return vec![Finding::new(Severity::Error, self.name(), Some("BPS"), self.first_open,
                format!("{} BPS record(s) never closed by EPS", self.depth))];
        }
        vec![]
    }
}

#[derive(Default)]
pub struct MirSingletonRule { count: u32 }
impl RecordRule for MirSingletonRule {
    fn name(&self) -> &'static str { "STRUCT/MIR_SINGLETON" }
    fn on_record(&mut self, record: &StdfRecord, _ctx: &ValidationContext, offset: u64) -> Vec<Finding> {
        if matches!(record, StdfRecord::Mir(_)) {
            self.count += 1;
            if self.count > 1 {
                return vec![Finding::new(Severity::Error, self.name(), Some("MIR"), Some(offset),
                    format!("MIR seen {} times; spec requires exactly one", self.count))];
            }
        }
        vec![]
    }
}
