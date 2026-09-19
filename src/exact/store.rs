use super::ExactWindow;
use crate::budget::Budget;
use crate::model::{
    CHECKPOINT_VERSION, EntityId, IngestError, PairCount, RelationProfile, ScopeId,
    WindowCheckpoint,
};
use alloc::collections::BTreeMap;

pub(super) fn checkpoint(window: &ExactWindow) -> WindowCheckpoint {
    WindowCheckpoint {
        version: CHECKPOINT_VERSION,
        seed: 0,
        watermark: window.watermark,
        closed: window.closed,
        width: window.budget.width,
        replicas: window.budget.replicas,
        pairs: window
            .counts
            .iter()
            .map(|(key, count)| PairCount {
                scope: key.0.clone(),
                relation: key.1.clone(),
                source: key.2.clone(),
                target: key.3.clone(),
                count: *count,
            })
            .collect(),
        keys: window.dedup.iter().cloned().collect(),
        witnesses: window.witnesses.clone(),
        counters: None,
        lateness: window.budget.lateness,
        max_pairs: window.budget.max_pairs,
        max_dedup: window.budget.max_dedup,
        max_witnesses: window.budget.max_witnesses,
        witnesses_truncated: window.witnesses.len() >= window.budget.max_witnesses
            && window.dedup.len() > window.witnesses.len(),
    }
}

pub(super) fn restore(
    budget: Budget,
    snapshot: WindowCheckpoint,
) -> Result<ExactWindow, IngestError> {
    if snapshot.version != CHECKPOINT_VERSION {
        return Err(IngestError::Checkpoint("version"));
    }
    if snapshot.counters.is_some() {
        return Err(IngestError::Checkpoint("exact-cannot-hold-sketch"));
    }
    let budget = budget
        .validate()
        .map_err(|_| IngestError::Checkpoint("budget"))?;
    if snapshot.lateness != budget.lateness
        || snapshot.max_pairs != budget.max_pairs
        || snapshot.max_dedup != budget.max_dedup
        || snapshot.max_witnesses != budget.max_witnesses
    {
        return Err(IngestError::Checkpoint("policy-mismatch"));
    }
    if snapshot.pairs.len() > budget.max_pairs
        || snapshot.keys.len() > budget.max_dedup
        || snapshot.witnesses.len() > budget.max_witnesses
    {
        return Err(IngestError::Budget("checkpoint"));
    }
    let mut counts = BTreeMap::new();
    for pair in snapshot.pairs {
        let key = (pair.scope, pair.relation, pair.source, pair.target);
        if counts.insert(key, pair.count).is_some() {
            return Err(IngestError::Checkpoint("duplicate-pair"));
        }
    }
    Ok(ExactWindow {
        budget,
        watermark: snapshot.watermark,
        closed: snapshot.closed,
        counts,
        dedup: snapshot.keys.into_iter().collect(),
        witnesses: snapshot.witnesses,
    })
}

pub(super) fn try_merge(left: &mut ExactWindow, right: &ExactWindow) -> Result<(), IngestError> {
    if left.dedup.iter().any(|key| right.dedup.contains(key)) {
        return Err(IngestError::Checkpoint("overlapping-event"));
    }
    let mut next_counts = left.counts.clone();
    for (key, extra) in &right.counts {
        let current = next_counts.get(key).copied().unwrap_or(0);
        if current == 0 && next_counts.len() >= left.budget.max_pairs {
            return Err(IngestError::Budget("pairs"));
        }
        let total = current.checked_add(*extra).ok_or(IngestError::Overflow)?;
        next_counts.insert(key.clone(), total);
    }
    if left.dedup.len() + right.dedup.len() > left.budget.max_dedup {
        return Err(IngestError::Budget("dedup"));
    }
    left.counts = next_counts;
    left.dedup.extend(right.dedup.iter().cloned());
    for witness in &right.witnesses {
        if left.witnesses.len() >= left.budget.max_witnesses {
            break;
        }
        left.witnesses.push(witness.clone());
    }
    if right.watermark > left.watermark {
        left.watermark = right.watermark;
    }
    left.closed = left.closed || right.closed;
    Ok(())
}

pub(super) fn exported_pairs(
    window: &ExactWindow,
) -> alloc::vec::Vec<(ScopeId, RelationProfile, EntityId, EntityId, u64)> {
    window
        .counts
        .iter()
        .map(|(key, count)| {
            (
                key.0.clone(),
                key.1.clone(),
                key.2.clone(),
                key.3.clone(),
                *count,
            )
        })
        .collect()
}
