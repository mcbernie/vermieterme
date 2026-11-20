//! Bounded key-value store of the rate limiter: at most `capacity` keys; inserting a new
//! key into a full store evicts the oldest key (by first insertion). Eviction is
//! amortised O(1): an insertion-order queue holds `(sequence, key)` pairs, and pairs whose
//! key was removed or re-inserted since are skipped (and compacted away in bulk).

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

/// A stored value with the sequence number of its insertion.
#[derive(Debug)]
struct Slot<V> {
    seq: u64,
    value: V,
}

/// The bounded store.
#[derive(Debug)]
pub(super) struct Store<V> {
    slots: HashMap<Arc<str>, Slot<V>>,
    order: VecDeque<(u64, Arc<str>)>,
    next_seq: u64,
    capacity: usize,
}

impl<V> Store<V> {
    /// An empty store for at most `capacity` keys (at least one).
    pub(super) fn with_capacity(capacity: usize) -> Self {
        Self {
            slots: HashMap::new(),
            order: VecDeque::new(),
            next_seq: 0,
            capacity: capacity.max(1),
        }
    }

    /// The value of `key`, inserted with `make` if missing (evicting the oldest key when
    /// the store is full).
    pub(super) fn entry(&mut self, key: &str, make: impl FnOnce() -> V) -> &mut V {
        let key: Arc<str> = Arc::from(key);
        let seq = self.next_seq;
        if !self.slots.contains_key(&key) {
            while self.slots.len() >= self.capacity {
                self.evict_oldest();
            }
            // Before the push: the new pair has no slot yet and would be compacted away.
            if self.order.len() >= self.capacity.saturating_mul(2) {
                self.compact();
            }
            self.next_seq += 1;
            self.order.push_back((seq, key.clone()));
        }
        let slot = self
            .slots
            .entry(key)
            .or_insert_with(|| Slot { seq, value: make() });
        &mut slot.value
    }

    /// The value of `key`, if stored.
    pub(super) fn get_mut(&mut self, key: &str) -> Option<&mut V> {
        self.slots.get_mut(key).map(|slot| &mut slot.value)
    }

    /// Forgets `key`.
    pub(super) fn remove(&mut self, key: &str) {
        self.slots.remove(key);
    }

    /// Keeps only values for which `keep` holds.
    pub(super) fn retain(&mut self, mut keep: impl FnMut(&V) -> bool) {
        self.slots.retain(|_, slot| keep(&slot.value));
        self.compact();
    }

    /// Number of stored keys (tests).
    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.slots.len()
    }

    /// Removes the oldest live key (skips queue pairs of removed or re-inserted keys).
    fn evict_oldest(&mut self) {
        while let Some((seq, key)) = self.order.pop_front() {
            if self.is_live(seq, &key) {
                self.slots.remove(&key);
                return;
            }
        }
        // Every live key has a queue pair, so an empty queue means an empty store.
        self.slots.clear();
    }

    /// Drops queue pairs whose key is gone or was re-inserted.
    fn compact(&mut self) {
        let order = std::mem::take(&mut self.order);
        self.order = order
            .into_iter()
            .filter(|(seq, key)| self.is_live(*seq, key))
            .collect();
    }

    /// Whether the queue pair `(seq, key)` belongs to the stored value of `key`.
    fn is_live(&self, seq: u64, key: &str) -> bool {
        self.slots.get(key).is_some_and(|slot| slot.seq == seq)
    }
}

#[cfg(test)]
mod tests {
    use super::Store;

    #[test]
    fn full_store_evicts_the_oldest_key() {
        let mut store = Store::with_capacity(3);
        for key in ["a", "b", "c", "d"] {
            *store.entry(key, || 0) += 1;
        }
        assert_eq!(store.len(), 3);
        assert!(store.get_mut("a").is_none());
        assert!(store.get_mut("d").is_some());
    }

    #[test]
    fn removed_and_reinserted_keys_keep_the_order_right() {
        let mut store = Store::with_capacity(2);
        store.entry("a", || 1);
        store.entry("b", || 2);
        store.remove("a");
        store.entry("a", || 3);
        store.entry("c", || 4);
        // "b" is now the oldest live key.
        assert!(store.get_mut("b").is_none());
        assert_eq!(store.get_mut("a").copied(), Some(3));
        assert_eq!(store.get_mut("c").copied(), Some(4));
    }

    #[test]
    fn queue_stays_bounded_under_churn() {
        let mut store = Store::with_capacity(4);
        for i in 0..1_000 {
            let key = format!("k{}", i % 3);
            store.entry(&key, || i);
            store.remove(&key);
        }
        assert!(store.order.len() <= 8);
        store.entry("x", || 0);
        store.retain(|v| *v != 0);
        assert_eq!(store.len(), 0);
        assert!(store.order.is_empty());
    }
}
