use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Attempt {
    pub source: String,
    pub run: String,
    pub offset: usize,
    pub sequence: u64,
    pub step: Option<usize>,
    pub identity: Option<String>,
    pub original_identity: Option<String>,
    pub lot: String,
    pub wafer: Option<String>,
    pub prr_x: Option<i16>,
    pub prr_y: Option<i16>,
    pub head: u8,
    pub site: u8,
    pub part_id: Option<String>,
    pub part_flg: u8,
    pub passed: Option<bool>,
    pub hard_bin: Option<u16>,
    pub soft_bin: Option<u16>,
    pub raw_hard_bin: u16,
    pub raw_soft_bin: u16,
    pub hardware: BTreeMap<String, Option<String>>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Run {
    pub program: Selector,
    pub start: Option<u32>,
    pub finish: Option<u32>,
    pub mir: BTreeMap<String, serde_json::Value>,
}
#[derive(Debug, Serialize)]
pub struct Cell {
    pub step: usize,
    pub status: &'static str,
    pub tests: usize,
    pub retests: usize,
    pub changes: Vec<&'static str>,
    pub incomplete: bool,
    pub order_unknown: bool,
    pub latest_passed: Option<bool>,
    pub attempts: Vec<Attempt>,
}
#[derive(Debug, Serialize)]
pub struct Device {
    pub key: String,
    pub identity: Option<Identity>,
    pub lots: BTreeSet<String>,
    pub closed: bool,
    pub issues: Vec<&'static str>,
    pub steps: Vec<Cell>,
    pub unidentified: Vec<Attempt>,
}

fn cell(step: usize, mut attempts: Vec<Attempt>, runs: &BTreeMap<String, Run>) -> Cell {
    // Stable display order is explicitly not a claim of temporal order.
    attempts.sort_by(|a, b| {
        (
            runs.get(&a.run).and_then(|r| r.start),
            &a.source,
            a.sequence,
        )
            .cmp(&(
                runs.get(&b.run).and_then(|r| r.start),
                &b.source,
                b.sequence,
            ))
    });
    let mut changes = Vec::new();
    if attempts
        .iter()
        .filter_map(|a| a.passed)
        .collect::<BTreeSet<_>>()
        .len()
        > 1
    {
        changes.push("verdict_changed");
    }
    if attempts
        .iter()
        .filter_map(|a| a.hard_bin)
        .collect::<BTreeSet<_>>()
        .len()
        > 1
    {
        changes.push("hard_bin_changed");
    }
    if attempts
        .iter()
        .filter_map(|a| a.soft_bin)
        .collect::<BTreeSet<_>>()
        .len()
        > 1
    {
        changes.push("soft_bin_changed");
    }
    let incomplete = attempts
        .iter()
        .any(|a| a.passed.is_none() || a.hard_bin.is_none() || a.soft_bin.is_none());
    // Merge source-local sequences by run start without using a partial-order
    // comparator (which would violate sort's transitivity contract).
    let mut sources: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (i, a) in attempts.iter().enumerate() {
        sources.entry(&a.source).or_default().push(i);
    }
    for indices in sources.values_mut() {
        indices.sort_by_key(|&i| attempts[i].sequence);
    }
    let mut fronts = BTreeSet::new();
    for (&source, indices) in &sources {
        fronts.insert((
            runs.get(&attempts[indices[0]].run).and_then(|r| r.start),
            source,
            0usize,
        ));
    }
    let mut chronological = Vec::with_capacity(attempts.len());
    while let Some((_, source, position)) = fronts.pop_first() {
        let indices = &sources[source];
        chronological.push(indices[position]);
        if let Some(&next) = indices.get(position + 1) {
            fronts.insert((
                runs.get(&attempts[next].run).and_then(|r| r.start),
                source,
                position + 1,
            ));
        }
    }
    // Source-local sequence can override that source's timestamps, so adjacent
    // pairs alone are insufficient: a long earlier run may overlap another
    // source even when the immediately preceding run does not.
    let mut order_unknown = false;
    if sources.len() > 1 {
        let mut finishes: BTreeMap<&str, u32> = BTreeMap::new();
        let mut endpoints = BTreeSet::new();
        for &index in &chronological {
            let a = &attempts[index];
            let interval = runs.get(&a.run).and_then(|r| r.start.zip(r.finish));
            let Some((start, finish)) = interval.filter(|(start, finish)| start <= finish) else {
                order_unknown = true;
                break;
            };
            if endpoints
                .iter()
                .rev()
                .find(|(_, source)| *source != a.source.as_str())
                .is_some_and(|(end, _)| *end >= start)
            {
                order_unknown = true;
                break;
            }
            let old = finishes.get(a.source.as_str()).copied().unwrap_or(0);
            endpoints.remove(&(old, a.source.as_str()));
            let end = old.max(finish);
            finishes.insert(a.source.as_str(), end);
            endpoints.insert((end, a.source.as_str()));
        }
    }
    let latest_passed = if order_unknown {
        None
    } else {
        chronological.last().and_then(|&i| attempts[i].passed)
    };
    if !order_unknown {
        let mut ordered: Vec<_> = chronological
            .into_iter()
            .map(|i| attempts[i].clone())
            .collect();
        std::mem::swap(&mut attempts, &mut ordered);
    }
    let status = if !changes.is_empty() {
        "retest_different"
    } else if incomplete {
        "indeterminate"
    } else if attempts.len() > 1 {
        "retest_consistent"
    } else {
        "tested"
    };
    Cell {
        step,
        status,
        tests: attempts.len(),
        retests: attempts.len().saturating_sub(1),
        changes,
        incomplete,
        order_unknown,
        latest_passed,
        attempts,
    }
}
pub fn reduce(
    key: String,
    attempts: Vec<Attempt>,
    flow: &Flow,
    closures: &[Closure],
    aliases: &BTreeMap<String, String>,
    runs: &BTreeMap<String, Run>,
) -> Device {
    let identity = attempts
        .first()
        .and_then(|a| a.identity.as_ref())
        .and_then(|s| serde_json::from_str::<Identity>(s).ok());
    let lots: BTreeSet<_> = attempts.iter().map(|a| a.lot.clone()).collect();
    let closed = closures.iter().any(|c| {
        c.lot.as_ref().is_some_and(|l| lots.contains(l))
            || c.device.as_ref().is_some_and(|d| {
                let original = d.key();
                aliases.get(&original).unwrap_or(&original) == &key
            })
    });
    let mut groups: Vec<Vec<Attempt>> = (0..flow.steps.len()).map(|_| Vec::new()).collect();
    let mut unidentified = Vec::new();
    for a in attempts {
        if let Some(s) = a.step {
            groups[s].push(a);
        } else {
            unidentified.push(a);
        }
    }
    unidentified.sort_by(|a, b| (&a.source, a.sequence).cmp(&(&b.source, b.sequence)));
    let mut issues = Vec::new();
    if identity.is_none() {
        issues.push("identity_unresolved");
    }
    if identity.as_ref().is_some_and(|i| i.0 == "lot-ptr") {
        issues.push("fallback_unlinked");
    }
    if !unidentified.is_empty() {
        issues.push("step_unidentified");
    }
    let unreliable = !issues.is_empty();
    let last_observed = groups.iter().rposition(|a| !a.is_empty());
    let mut stopped = false;
    let mut stop_uncertain = false;
    let mut steps = Vec::new();
    for (i, group) in groups.into_iter().enumerate() {
        let mut c = cell(i, group, runs);
        if c.tests == 0 {
            c.status = if !flow.steps[i].required {
                "not_applicable"
            } else if unreliable {
                "indeterminate"
            } else if stopped {
                "not_applicable"
            } else if stop_uncertain {
                "indeterminate"
            } else if closed || last_observed.is_some_and(|last| last > i) {
                "missing"
            } else {
                "pending"
            };
        } else if flow.steps[i].stop_on_fail {
            // Each stop decision uses the latest attempt, never an earlier fail.
            stopped |= c.latest_passed == Some(false) && !unreliable && !c.incomplete;
            stop_uncertain |= c.latest_passed.is_none() || c.incomplete;
        }
        if !c.changes.is_empty() {
            issues.push("retest_different");
        }
        if c.status == "missing" {
            issues.push("missing");
        }
        if c.incomplete {
            issues.push("result_incomplete");
        }
        if c.order_unknown {
            issues.push("order_unknown");
        }
        steps.push(c);
    }
    issues.sort_unstable();
    issues.dedup();
    Device {
        key,
        identity,
        lots,
        closed,
        issues,
        steps,
        unidentified,
    }
}
