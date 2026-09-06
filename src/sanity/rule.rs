use crate::{parser::StdfRecord, sanity::{context::ValidationContext, finding::Finding}};

pub trait RecordRule: Send + Sync {
    fn name(&self) -> &'static str;

    fn on_record(
        &mut self,
        record:  &StdfRecord,
        context: &ValidationContext,
        offset:  u64,
    ) -> Vec<Finding>;

    fn on_eof(&mut self, context: &ValidationContext) -> Vec<Finding> {
        let _ = context;
        vec![]
    }
}
