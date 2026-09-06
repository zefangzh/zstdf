use crate::parser::StdfRecord;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct ValidationContext {
    pub record_count:     u64,
    pub current_offset:   u64,
    pub far_seen:         bool,
    pub mir_count:        u32,
    pub mrr_seen:         bool,
    pub open_wir_offsets: Vec<u64>,
    pub open_pir_offset:  Option<u64>,
    pub bps_depth:        u32,
    pub test_registry:    HashMap<u32, (String, u64)>,
    pub hb_counts_prr:    HashMap<u16, u64>,
    /// Keyed by (head_num, site_num, hbin_num) rather than just hbin_num,
    /// so that per-head/per-site HBR records for the same bin are summed
    /// instead of overwriting each other (matches how PRR counts are
    /// accumulated across every site/part).
    pub hb_counts_hbr:    HashMap<(u8, u8, u16), u64>,
    pub mir_start_t:      Option<u32>,
    pub part_count_prr:   u64,
}

impl ValidationContext {
    pub fn is_inside_wafer(&self) -> bool { !self.open_wir_offsets.is_empty() }
    pub fn is_inside_part(&self)  -> bool { self.open_pir_offset.is_some() }

    pub fn update(&mut self, record: &StdfRecord, offset: u64) {
        self.record_count   += 1;
        self.current_offset  = offset;
        match record {
            StdfRecord::Far(_)  => self.far_seen = true,
            StdfRecord::Mir(m)  => {
                self.mir_count  += 1;
                self.mir_start_t = Some(m.start_t);
            }
            StdfRecord::Mrr(_)  => self.mrr_seen = true,
            StdfRecord::Wir(_)  => self.open_wir_offsets.push(offset),
            StdfRecord::Wrr(_)  => { self.open_wir_offsets.pop(); }
            StdfRecord::Pir(_)  => self.open_pir_offset = Some(offset),
            StdfRecord::Prr(p)  => {
                self.open_pir_offset = None;
                self.part_count_prr += 1;
                *self.hb_counts_prr.entry(p.hard_bin).or_default() += 1;
            }
            StdfRecord::Hbr(h)  => {
                *self.hb_counts_hbr.entry((h.head_num, h.site_num, h.hbin_num)).or_default() += h.hbin_cnt as u64;
            }
            StdfRecord::Bps(_)  => self.bps_depth += 1,
            StdfRecord::Eps(_)  => self.bps_depth = self.bps_depth.saturating_sub(1),
            _ => {}
        }
    }
}
