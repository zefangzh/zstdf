use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Flow {
    pub flow_id: String,
    pub version: String,
    pub steps: Vec<Step>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub id: String,
    pub required: bool,
    #[serde(default)]
    pub stop_on_fail: bool,
    pub matches: Vec<Selector>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selector {
    pub job_nam: Option<String>,
    pub job_rev: Option<String>,
    pub test_cod: Option<String>,
    pub flow_id: Option<String>,
    pub tst_temp: Option<String>,
}
impl Selector {
    fn fields(&self) -> [&Option<String>; 5] {
        [
            &self.job_nam,
            &self.job_rev,
            &self.test_cod,
            &self.flow_id,
            &self.tst_temp,
        ]
    }
    fn accepts(&self, actual: &Selector) -> bool {
        self.fields()
            .iter()
            .zip(actual.fields())
            .all(|(wanted, got)| wanted.is_none() || *wanted == got)
    }
}
impl Flow {
    pub fn validate(&self) -> CliResult<()> {
        if self.flow_id.is_empty() || self.version.is_empty() || self.steps.is_empty() {
            return Err("flow ID, version and ordered steps must be nonempty".into());
        }
        let mut ids = BTreeSet::new();
        for step in &self.steps {
            if step.id.is_empty()
                || !ids.insert(&step.id)
                || step.matches.is_empty()
                || step
                    .matches
                    .iter()
                    .any(|m| m.fields().iter().all(|f| f.is_none()))
            {
                return Err(
                    "step IDs must be unique/nonempty; each step needs nonempty MIR selectors"
                        .into(),
                );
            }
        }
        Ok(())
    }
    pub fn match_step(&self, actual: &Selector) -> Option<usize> {
        let found: Vec<_> = self
            .steps
            .iter()
            .enumerate()
            .filter(|(_, s)| s.matches.iter().any(|m| m.accepts(actual)))
            .map(|(i, _)| i)
            .collect();
        if found.len() == 1 {
            Some(found[0])
        } else {
            None
        }
    }
}

/// Full identity tuple; intentionally no coordinate-only aliasing.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Identity(pub String, pub String, pub i64, pub i64);
impl Identity {
    pub fn key(&self) -> String {
        serde_json::to_string(self).expect("identity serializes")
    }
    pub fn validate(&self) -> CliResult<()> {
        if self.1.is_empty() || self.2 <= 0 || self.3 <= 0 {
            return Err("identity needs a nonempty ID and positive integer coordinates".into());
        }
        match self.0.as_str() {
            "lot-ptr" => Ok(()),
            "wafer-prr"
                if self.2 <= i16::MAX as i64
                    && self.3 <= i16::MAX as i64
                    && self
                        .1
                        .bytes()
                        .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit()) =>
            {
                Ok(())
            }
            _ => Err("invalid identity namespace, wafer ID or PRR coordinate range".into()),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mapping {
    pub from: Identity,
    pub to: Identity,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Closure {
    pub flow_id: String,
    pub version: String,
    pub lot: Option<String>,
    pub device: Option<Identity>,
}
pub fn mappings(rows: &[Mapping]) -> CliResult<BTreeMap<String, String>> {
    let mut map = BTreeMap::new();
    for row in rows {
        row.from.validate()?;
        row.to.validate()?;
        if row.from.0 != "lot-ptr" || row.to.0 != "wafer-prr" {
            return Err("identity mappings must go from lot-ptr to wafer-prr".into());
        }
        let key = row.from.key();
        let value = row.to.key();
        if map
            .insert(key, value.clone())
            .is_some_and(|old| old != value)
        {
            return Err("conflicting identity mapping".into());
        }
    }
    Ok(map)
}
pub fn validate_closures(rows: &[Closure], flow: &Flow) -> CliResult<()> {
    for row in rows {
        if row.flow_id != flow.flow_id || row.version != flow.version {
            return Err("closure flow ID/version does not match configured flow".into());
        }
        if row.lot.is_some() == row.device.is_some()
            || row.lot.as_ref().is_some_and(|v| v.is_empty())
        {
            return Err("closure needs exactly one nonempty lot or full device identity".into());
        }
        if let Some(id) = &row.device {
            id.validate()?;
        }
    }
    Ok(())
}
