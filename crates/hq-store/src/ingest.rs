use crate::Store;
use crate::StoreError;
use hq_core::{Event, EventDraft, redact_json};

pub struct Ingestor {
    pub store: Store,
}

pub struct IngestReport {
    pub accepted: Vec<Event>,
    pub duplicates: usize,
}

impl Ingestor {
    pub fn new(store: Store) -> Self {
        Self { store }
    }

    pub fn store(&self) -> Store {
        self.store.clone()
    }

    pub async fn ingest(&self, mut drafts: Vec<EventDraft>) -> Result<IngestReport, StoreError> {
        // validate first (atomic: any invalid aborts batch)
        for d in &drafts {
            if let Err(e) = d.validate() {
                return Err(StoreError::Validation(format!(
                    "invalid event subject={} kind={}: {}",
                    d.subject.key(),
                    d.kind.as_str(),
                    e
                )));
            }
        }
        // redact before persist
        for d in &mut drafts {
            redact_json(&mut d.payload);
        }
        let before = drafts.len();
        let accepted = self.store.append(drafts).await?;
        let duplicates = before.saturating_sub(accepted.len());
        Ok(IngestReport {
            accepted,
            duplicates,
        })
    }
}
