//! A Least Recently Used (LRU) Cache.
//!
//! `LRUCache<K, V, MAX_SIZE>` stores up to `MAX_SIZE` key-value pairs,
//! automatically evicting the least recently used entry when the capacity
//! is exceeded. This makes it useful for caching expensive computations,
//! I/O results, or any data where temporal locality is expected.
//!
//! # Example
//! ```
//! let mut cache: LRUCache<i32, String, 2> = LRUCache::new();
//!
//! cache.put(1, "one".to_string());
//! cache.put(2, "two".to_string());
//!
//! // Access key 1, making it most recently used
//! assert_eq!(cache.get(1).map(|v| v.as_str()), Some("one"));
//!
//! // Insert new key, evicting key 2 (least recently used)
//! cache.put(3, "three".to_string());
//!
//! assert!(cache.get(2).is_none()); // evicted
//! assert!(cache.get(1).is_some());
//! assert!(cache.get(3).is_some());
//! ```
use alloc::{collections::BTreeMap, vec::Vec};

struct Node<K: Clone, V> {
    v: V,
    prev: Option<K>,
    next: Option<K>,
}

/// An Least Recently Used Cache with capacity `MAX_SIZE`.
pub struct LRUCache<K: Ord + Clone, V, const MAX_SIZE: usize> {
    inner: BTreeMap<K, Node<K, V>>,

    // Access information
    head: Option<K>,
    tail: Option<K>,
}

impl<K: Ord + Clone, V, const MAX_SIZE: usize> Default for LRUCache<K, V, MAX_SIZE> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Ord + Clone, V, const MAX_SIZE: usize> LRUCache<K, V, MAX_SIZE> {
    // Attach the access information about key.
    fn attach(&mut self, k: K) -> &mut Node<K, V> {
        if let Some(tail) = self.tail.take() {
            let last = self.inner.get_mut(&tail).unwrap();
            last.next = Some(k.clone());
        } else {
            self.head = Some(k.clone());
        }
        let ptail = self.tail.clone();
        self.tail = Some(k.clone());

        let node = self.inner.get_mut(&k).unwrap();
        node.prev = ptail;
        node
    }

    // Detach the access information about key.
    fn detach(&mut self, prev: Option<K>, next: Option<K>) {
        if let Some(next) = next.as_ref() {
            self.inner.get_mut(next).unwrap().prev = prev.clone();
        } else {
            self.tail = prev.clone();
        }

        if let Some(prev) = prev {
            self.inner.get_mut(&prev).unwrap().next = next;
        } else {
            self.head = next;
        }
    }

    /// Makes a new, empty `LRUCache`.
    ///
    /// Does not allocate anything on its own.
    pub const fn new() -> Self {
        Self {
            inner: BTreeMap::new(),
            head: None,
            tail: None,
        }
    }

    /// Returns a mutable reference to the value corresponding to the key and
    /// update the last access time.
    pub fn get(&mut self, k: K) -> Option<&mut V> {
        let node = self.inner.get_mut(&k)?;
        let (prev, next) = (node.prev.take(), node.next.take());
        self.detach(prev, next);
        Some(&mut self.attach(k).v)
    }

    /// Inserts the value computed with `f` into the `LRUCache` if it is not
    /// present, then returns a reference to the value in the `LRUCache`.
    pub fn get_or_insert_with<E>(
        &mut self,
        k: K,
        f: impl FnOnce() -> Result<V, E>,
    ) -> Result<&mut V, E> {
        Ok(if let Some(node) = self.inner.get_mut(&k) {
            let (prev, next) = (node.prev.take(), node.next.take());
            self.detach(prev, next);
            &mut self.attach(k).v
        } else {
            &mut self.__put(k, f()?).v
        })
    }

    fn __put(&mut self, k: K, v: V) -> &mut Node<K, V> {
        if let Some(node) = self.inner.get_mut(&k) {
            node.v = v;
            let (prev, next) = (node.prev.take(), node.next.take());
            self.detach(prev, next);
        } else {
            if MAX_SIZE <= self.inner.len() {
                self.remove(&self.head.clone().unwrap());
            }
            let node = Node {
                v,
                prev: self.tail.clone(),
                next: None,
            };
            self.inner.insert(k.clone(), node);
        }
        self.attach(k)
    }

    /// Inserts a key-value pair into the `LRUCache`.
    ///
    /// If the map did have this key present, the value is updated.
    /// The key is not updated, though; this matters for types that can be ==
    /// without being identical.
    ///
    /// If the cache size is overflowed after insertion, evict the oldest
    /// accessed entry.
    pub fn put(&mut self, k: K, v: V) {
        self.__put(k, v);
    }

    /// Removes a key from the LRUCache, returning the stored value if the
    /// key was previously in the LRUCache.
    ///
    /// The key may be any borrowed form of the LRUCache’s key type, but the
    /// ordering on the borrowed form must match the ordering on the key
    /// type.
    pub fn remove(&mut self, k: &K) -> Option<V> {
        let mut node = self.inner.remove(k)?;
        self.detach(node.prev.take(), node.next.take());
        Some(node.v)
    }

    /// Retains only the elements specified by the predicate.
    /// In other words, remove all pairs (k, v) for which f(&k, &mut v) returns
    /// false.
    pub fn retain(&mut self, mut f: impl FnMut(&K, &mut V) -> bool) {
        let retain_targets = self
            .inner
            .iter_mut()
            .filter_map(|(k, v)| {
                if !f(k, &mut v.v) {
                    Some(k.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        for target in retain_targets.into_iter() {
            self.remove(&target);
        }
    }

    /// Iterates over the key-value pairs in the LRUCache.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut V)> {
        self.inner.iter_mut().map(|(k, v)| (k, &mut v.v))
    }
}
