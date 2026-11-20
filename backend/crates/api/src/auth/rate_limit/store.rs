//! Bounded key-value store of the rate limiter: at most `capacity` keys. Inserting a new
//! key into a full store evicts the oldest key the caller allows to evict (by queue
//! order); keys it protects (locked or with running attempts) move to the back of the
//! queue instead. If no evictable key turns up within [`EVICTION_PROBES`], the new key is
//! **not** stored – a flood of new keys can never push protected keys out.
//!
//! Every insertion gets a sequence number; it identifies this incarnation of the key
//! (a key removed and inserted again gets a new one). Eviction is amortised O(1): the
//! queue holds `(sequence, key)` pairs, pairs whose key was removed or re-inserted since
//! are skipped (and compacted away in bulk).

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

/// Most live keys inspected per insertion into a full store; bounds the work per request
/// when many keys are protected.
const EVICTION_PROBES: usize = 32;

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

    /// The value of `key` with its sequence number, inserted with `make` if missing. A
    /// full store first evicts a key for which `evictable` holds; `None` if there is none
    /// (the new key is not stored).
    pub(super) fn entry(
        &mut self,
        key: &str,
        make: impl FnOnce() -> V,
        evictable: impl Fn(&V) -> bool,
    ) -> Option<(u64, &mut V)> {
        let key: Arc<str> = Arc::from(key);
        let seq = self.next_seq;
        if !self.slots.contains_key(&key) {
            if self.slots.len() >= self.capacity && !self.evict_one(&evictable) {
                return None;
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
        Some((slot.seq, &mut slot.value))
    }

    /// The value of `key`, if stored with sequence number `seq` (not a later incarnation).
    pub(super) fn get_mut_seq(&mut self, key: &str, seq: u64) -> Option<&mut V> {
        self.slots
            .get_mut(key)
            .filter(|slot| slot.seq == seq)
            .map(|slot| &mut slot.value)
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

    /// Removes the oldest live key for which `evictable` holds; protected keys move to the
    /// back of the queue. `false` if none was found within [`EVICTION_PROBES`] live keys.
    fn evict_one(&mut self, evictable: &impl Fn(&V) -> bool) -> bool {
        let mut probes = 0;
        while probes < EVICTION_PROBES {
            let Some((seq, key)) = self.order.pop_front() else {
                return false;
            };
            let Some(slot) = self.slots.get(&key).filter(|slot| slot.seq == seq) else {
                continue; // Pair of a removed or re-inserted key.
            };
            probes += 1;
            if evictable(&slot.value) {
                self.slots.remove(&key);
                return true;
            }
            self.order.push_back((seq, key));
        }
        false
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
    use super::{EVICTION_PROBES, Store};

    /// Inserts `key` with `value`; every key is evictable.
    fn put(store: &mut Store<i32>, key: &str, value: i32) -> Option<i32> {
        store.entry(key, || value, |_| true).map(|(_, v)| *v)
    }

    /// Current value of `key` (any incarnation).
    fn get(store: &mut Store<i32>, key: &str) -> Option<i32> {
        let seq = store.slots.get(key)?.seq;
        store.get_mut_seq(key, seq).copied()
    }

    #[test]
    fn full_store_evicts_the_oldest_key() {
        let mut store = Store::with_capacity(3);
        for key in ["a", "b", "c", "d"] {
            put(&mut store, key, 0);
        }
        assert_eq!(store.len(), 3);
        assert!(get(&mut store, "a").is_none());
        assert!(get(&mut store, "d").is_some());
    }

    #[test]
    fn removed_and_reinserted_keys_keep_the_order_right() {
        let mut store = Store::with_capacity(2);
        put(&mut store, "a", 1);
        put(&mut store, "b", 2);
        store.remove("a");
        put(&mut store, "a", 3);
        put(&mut store, "c", 4);
        // "b" is now the oldest live key.
        assert!(get(&mut store, "b").is_none());
        assert_eq!(get(&mut store, "a"), Some(3));
        assert_eq!(get(&mut store, "c"), Some(4));
    }

    #[test]
    fn queue_stays_bounded_under_churn() {
        let mut store = Store::with_capacity(4);
        for i in 0..1_000 {
            let key = format!("k{}", i % 3);
            put(&mut store, &key, i);
            store.remove(&key);
        }
        assert!(store.order.len() <= 8);
        put(&mut store, "x", 0);
        store.retain(|v| *v != 0);
        assert_eq!(store.len(), 0);
        assert!(store.order.is_empty());
    }

    #[test]
    fn protected_keys_are_never_evicted() {
        let mut store = Store::with_capacity(3);
        let protect = |v: &i32| *v != 1;
        store.entry("locked", || 1, protect);
        for i in 0..100 {
            store.entry(&format!("k{i}"), || 0, protect);
        }
        assert_eq!(get(&mut store, "locked"), Some(1));
        assert_eq!(get(&mut store, "k99"), Some(0));
        assert_eq!(store.len(), 3);
    }

    #[test]
    fn full_store_of_protected_keys_refuses_new_keys() {
        let mut store = Store::with_capacity(EVICTION_PROBES + 5);
        for i in 0..EVICTION_PROBES + 5 {
            store.entry(&format!("k{i}"), || 1, |_| false);
        }
        assert!(store.entry("new", || 0, |_| false).is_none());
        assert!(get(&mut store, "new").is_none());
        // Existing keys are still served.
        assert!(store.entry("k0", || 0, |_| false).is_some());
        assert_eq!(store.len(), EVICTION_PROBES + 5);
    }

    #[test]
    fn reinserted_key_gets_a_new_sequence() {
        let mut store = Store::with_capacity(2);
        let (old, _) = store.entry("a", || 1, |_| true).unwrap();
        store.remove("a");
        let (new, _) = store.entry("a", || 2, |_| true).unwrap();
        assert_ne!(old, new);
        assert!(store.get_mut_seq("a", old).is_none());
        assert_eq!(store.get_mut_seq("a", new).copied(), Some(2));
    }
}
