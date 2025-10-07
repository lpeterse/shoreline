use std::fmt::Debug;
use std::sync::Arc;
use super::super::{Id, Peer};

#[derive(Debug)]
pub struct Table<const K: usize = 8, const B: usize = 32> {
    pub id: Id,
    pub buckets: Box<[Bucket<K>; B]>,
}

impl<const K: usize, const B: usize> Table<K, B> {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            buckets: Box::new([(); B].map(|_| Bucket::new())),
        }
    }

    pub fn count_good(&self) -> usize {
        self.buckets.iter().map(|b| b.count_good()).sum()
    }

    pub fn insert(&mut self, peer: Arc<Peer>) {
        let i = self.id.similarity(peer.id()).min(B - 1);
        self.buckets[i].insert(peer);
    }

    pub fn remove(&mut self, id: &Id) {
        let i = self.id.similarity(id).min(B - 1);
        self.buckets[i].remove(id);
    }

    pub fn collect(&self) -> Vec<Arc<Peer>> {
        let mut v = Vec::with_capacity(K * B);
        for b in self.buckets.as_ref() {
            for e in b.as_ref() {
                if let Some(e) = e {
                    v.push(e.clone());
                }
            }
        }
        v
    }

    pub fn closest_n(&self, id: &Id, n: usize) -> Vec<Arc<Peer>> {
        let p = self.id.similarity(id).min(B);
        let mut v = Vec::with_capacity(n);
        for i in p..B {
            for x in self.buckets[i].as_ref() {
                if let Some(x) = x {
                    if x.status().is_good() {
                        v.push(x.clone());
                        if v.len() >= n {
                            return v;
                        }
                    }
                }
            }
        }
        for i in (0..p).rev() {
            for x in self.buckets[i].as_ref() {
                if let Some(x) = x {
                    if x.status().is_good() {
                        v.push(x.clone());
                        if v.len() >= n {
                            return v;
                        }
                    }
                }
            }
        }
        v
    }
}

#[derive(Debug)]
pub struct Bucket<const K: usize>([Option<Arc<Peer>>; K]);

impl<const K: usize> Bucket<K> {
    pub fn new() -> Self {
        Self([(); K].map(|_| None))
    }

    pub fn count_good(&self) -> usize {
        self.0.iter().filter(|e| e.iter().any(|x| x.status().is_good())).count()
    }

    pub fn insert(&mut self, peer: Arc<Peer>) {
        // Fill empty slot
        for slot in self.0.iter_mut() {
            if slot.is_none() {
                *slot = Some(peer);
                return;
            }
        }
        // Replace expendable slot
        for slot in self.0.iter_mut() {
            if let Some(old) = slot {
                if old.status().is_expendable() {
                    *slot = Some(peer);
                    return;
                }
            }
        }
        // Replace slot with higher rtt
        for slot in self.0.iter_mut() {
            if let Some(old) = slot {
                if old.rtt() > peer.rtt() {
                    *slot = Some(peer);
                    return;
                }
            }
        }
    }

    pub fn remove(&mut self, id: &Id) {
        for x in self.0.iter_mut() {
            if let Some(e) = x {
                if e.id() == id {
                    *x = None;
                    return;
                }
            }
        }
    }
}

impl <const K: usize> AsRef<[Option<Arc<Peer>>]> for Bucket<K> {
    fn as_ref(&self) -> &[Option<Arc<Peer>>] {
        &self.0
    }
}
