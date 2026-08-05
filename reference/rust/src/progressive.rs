//! Progressive restoration: materialize a system from a blob, in stages.
//!
//! Like a progressive JPEG: useful immediately, sharpens over time.
//! The user sees the shell in <16ms, can interact in <200ms, and full
//! state arrives in <1s.

use crate::state_blob::{Blob, BlobError};

/// The four stages of progressive restoration.
///
/// Each stage is a superset of the previous — once you reach stage N,
/// you have everything stages 0..N provided.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RestoreStage {
    /// Stage 0 — Shell: static capture (bitmap/DOM skeleton) painted immediately.
    Shell,
    /// Stage 1 — Structure: DOM tree, layout, styles. Page looks complete.
    Structure,
    /// Stage 2 — Interactivity: JS handlers attached, forms editable.
    Interactivity,
    /// Stage 3 — Full state: network connections, live data, workers.
    FullState,
}

impl RestoreStage {
    /// All stages, in restoration order.
    pub const ALL: [RestoreStage; 4] = [
        RestoreStage::Shell,
        RestoreStage::Structure,
        RestoreStage::Interactivity,
        RestoreStage::FullState,
    ];

    /// Human-readable name.
    pub fn name(&self) -> &'static str {
        match self {
            RestoreStage::Shell => "shell",
            RestoreStage::Structure => "structure",
            RestoreStage::Interactivity => "interactivity",
            RestoreStage::FullState => "full_state",
        }
    }
}

/// A progressive restore orchestrator.
///
/// Given a blob, restores it stage by stage, invoking the appropriate
/// service for each stage.
pub struct RestoreOrchestrator {
    current_stage: RestoreStage,
    target_stage: RestoreStage,
}

/// Errors that can occur during restoration.
#[derive(Debug, thiserror::Error)]
pub enum RestoreError {
    #[error("restore failed at stage {stage:?}: {reason}")]
    StageFailed { stage: RestoreStage, reason: String },
    #[error("blob error: {0}")]
    Blob(String),
}

impl From<BlobError> for RestoreError {
    fn from(e: BlobError) -> Self {
        RestoreError::Blob(e.to_string())
    }
}

impl RestoreOrchestrator {
    /// Create a new orchestrator that restores up to `target_stage`.
    pub fn new(target_stage: RestoreStage) -> Self {
        Self {
            current_stage: RestoreStage::Shell,
            target_stage,
        }
    }

    /// Restore a blob progressively.
    ///
    /// Returns the restored blob and the highest stage reached.
    pub fn restore(&mut self, blob: &Blob) -> Result<(Blob, RestoreStage), RestoreError> {
        blob.validate()?;

        let mut current_blob = blob.clone();

        for stage in RestoreStage::ALL {
            if stage > self.target_stage {
                break;
            }
            current_blob = self.restore_stage(&current_blob, stage)?;
            self.current_stage = stage;
        }

        Ok((current_blob, self.current_stage))
    }

    /// Restore a single stage.
    ///
    /// In a real implementation, this would invoke the appropriate service.
    /// Here we model it as a pure function over the blob.
    fn restore_stage(&self, blob: &Blob, stage: RestoreStage) -> Result<Blob, RestoreError> {
        // Each stage adds its metadata to the blob payload.
        let stage_key = format!("_restored_{}", stage.name());
        let mut new_payload = blob.payload.clone();

        if let serde_json::Value::Object(map) = &mut new_payload {
            map.insert(stage_key, serde_json::json!(true));
        }

        blob.child(new_payload)
            .map_err(|e| RestoreError::StageFailed {
                stage,
                reason: e.to_string(),
            })
    }
}

/// Convenience: restore a blob to full state.
pub fn restore(blob: &Blob) -> Result<Blob, RestoreError> {
    let mut orch = RestoreOrchestrator::new(RestoreStage::FullState);
    let (blob, _) = orch.restore(blob)?;
    Ok(blob)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progressive_restore_marks_stages() {
        let blob = Blob::new(
            "test://schema",
            serde_json::json!({ "url": "https://example.com" }),
        );
        let mut orch = RestoreOrchestrator::new(RestoreStage::FullState);
        let (restored, stage) = orch.restore(&blob).unwrap();

        assert_eq!(stage, RestoreStage::FullState);

        let map = restored.payload.as_object().unwrap();
        assert!(map.contains_key("_restored_shell"));
        assert!(map.contains_key("_restored_structure"));
        assert!(map.contains_key("_restored_interactivity"));
        assert!(map.contains_key("_restored_full_state"));
    }

    #[test]
    fn partial_restore_stops_at_target() {
        let blob = Blob::new("test://schema", serde_json::json!({}));
        let mut orch = RestoreOrchestrator::new(RestoreStage::Structure);
        let (restored, stage) = orch.restore(&blob).unwrap();

        assert_eq!(stage, RestoreStage::Structure);

        let map = restored.payload.as_object().unwrap();
        assert!(map.contains_key("_restored_shell"));
        assert!(map.contains_key("_restored_structure"));
        assert!(!map.contains_key("_restored_interactivity"));
        assert!(!map.contains_key("_restored_full_state"));
    }

    #[test]
    fn restore_convenience_function() {
        let blob = Blob::new("test://schema", serde_json::json!({}));
        let restored = restore(&blob).unwrap();
        assert_eq!(restored.payload["_restored_full_state"], true);
    }

    #[test]
    fn restore_stage_ordering() {
        assert!(RestoreStage::Shell < RestoreStage::Structure);
        assert!(RestoreStage::Structure < RestoreStage::Interactivity);
        assert!(RestoreStage::Interactivity < RestoreStage::FullState);
    }
}
