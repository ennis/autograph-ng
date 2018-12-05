use slotmap::{Key, SecondaryMap, SlotMap};
use std::collections::{hash_map::DefaultHasher, HashMap};
use std::hash::{Hash, Hasher};

use crate::renderer::Scope;

enum ResourceEntry<V, C: Eq + Clone> {
    Persistent {
        resource: V,
    },
    Scoped {
        scope: Scope,
        create_info: C,
        key: CacheKey,
    },
}

struct CacheEntry<V, C: Eq + Clone> {
    /// Not yet used, must be zero.
    queue: u32,
    create_info: C,
    /// TODO Replace this with just V once SlotMap gets drain_filter
    value: Option<V>,
    last_used_frame: u64,
    scopes: Vec<Scope>,
}

impl<V, C: Eq + Clone> CacheEntry<V, C> {
    fn scopes_overlap(&self, scope: &Scope) -> bool {
        self.scopes.iter().any(|s| s.overlaps(scope))
    }
}

new_key_type! { pub struct CacheKey; }

pub struct ResourceCache<K: Key + Clone, V, C: Eq + Clone> {
    live: SlotMap<K, ResourceEntry<V, C>>,
    /// store of resources available for transients
    store: SlotMap<CacheKey, CacheEntry<V, C>>,
}

impl<K: Key + Clone, V, C: Eq + Clone> ResourceCache<K, V, C> {
    pub fn new() -> ResourceCache<K, V, C> {
        ResourceCache {
            live: SlotMap::with_key(),
            store: SlotMap::with_key(),
        }
    }

    pub fn insert(&mut self, resource: V) -> K {
        self.live.insert(ResourceEntry::Persistent { resource })
    }

    pub fn create_scoped(&mut self, scope: Scope, create_info: C) -> K {
        self.live.insert(ResourceEntry::Scoped {
            key: CacheKey::null(),
            create_info,
            scope,
        })
    }

    pub fn destroy(&mut self, key: K, callback: impl FnOnce(V)) {
        if let Some(v) = self.live.remove(key) {
            match v {
                ResourceEntry::Persistent { resource } => {
                    callback(resource);
                }
                ResourceEntry::Scoped { key, .. } => {
                    self.store.get_mut(key).unwrap().scopes.clear();
                }
            }
        }
    }

    fn evict<F: FnMut(V)>(&mut self, until_frame: u64, mut deleter: F) {
        /*self.store.retain(|k, e| {
            if e.last_used_frame > until_frame {
                let v = mem::replace(&mut e.value, None).unwrap();
                deleter(v);
                true
            } else {
                false
            }
        });*/
    }

    /// Call once per frame
    pub fn allocate_scoped(&mut self, key: K, frame: u64, alloc: impl FnOnce(&C) -> V) {
        let (ci, scope, ckey) = match self.live.get_mut(key).unwrap() {
            ResourceEntry::Persistent { .. } => return, // no need to allocate
            ResourceEntry::Scoped {
                ref create_info,
                ref scope,
                ref mut key,
            } => (create_info, scope, key),
        };

        // entry has a transient entry
        if let Some(tr) = self.store.get_mut(ckey.clone()) {
            // transient entry still valid
            if tr.create_info == *ci && !tr.scopes_overlap(scope) {
                // not in use, set used (FAST PATH)
                tr.scopes.push(scope.clone());
            }
        }
        // scan table to find compatible resource
        for (ck, tr) in self.store.iter_mut() {
            if tr.create_info == *ci && !tr.scopes_overlap(scope) {
                // same descriptor, not in use: set in use and update transient key
                tr.scopes.push(scope.clone());
                *ckey = ck;
                return;
            }
        }

        // no compatible resource was found: allocate a new one (SLOW PATH)
        let v = alloc(ci);
        let new_ck = self.store.insert(CacheEntry {
            create_info: ci.clone(),
            queue: 0, // TODO
            last_used_frame: frame,
            value: Some(v),
            scopes: vec![*scope],
        });
        *ckey = new_ck;
    }

    pub fn clear_scoped_resources(&mut self) {
        for (_, tr) in self.store.iter_mut() {
            tr.scopes.clear();
        }
    }

    /*fn free_transient(&mut self, key: K, frame: u64) {
        let (ci, ckey) = match self.live.get_mut(key).unwrap() {
            ResourceEntry::Persistent { .. } => return,
            ResourceEntry::Transient {
                ref create_info,
                ref mut key,
            } => (create_info, key),
        };

        if let Some(tr) = self.store.get_mut(ckey.clone()) {
            if tr.in_use {
                tr.in_use = false;
            } else {
                // double free?
                panic!("double free of transient resource");
            }
        } else {
            // TODO panic or whatever
            panic!("non-existent transient entry")
        }
    }*/

    /*fn swap_transients(&mut self, a: K, b: K) {
        assert_ne!(a.clone().into(), b.clone().into());
        let pa = match self.live.get_mut(a).unwrap() {
            ResourceEntry::Persistent { .. } => panic!("cannot swap non-transient resources"),
            ResourceEntry::Transient { ref mut key, .. } => key as *mut _,
        };

        let pb = match self.live.get_mut(b).unwrap() {
            ResourceEntry::Persistent { .. } => panic!("cannot swap non-transient resources"),
            ResourceEntry::Transient { key, .. } => key as *mut _,
        };

        unsafe {
            ptr::swap(pa, pb);
        }
    }*/

    pub fn get(&self, key: K) -> Option<&V> {
        match self.live.get(key).unwrap() {
            ResourceEntry::Persistent { ref resource } => Some(resource),
            ResourceEntry::Scoped { ref key, .. } => self
                .store
                .get(key.clone())
                .map(|e| e.value.as_ref().unwrap()),
        }
    }

    pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
        match self.live.get_mut(key).unwrap() {
            ResourceEntry::Persistent { ref mut resource } => Some(resource),
            ResourceEntry::Scoped { ref key, .. } => self
                .store
                .get_mut(key.clone())
                .map(|e| e.value.as_mut().unwrap()),
        }
    }
}

#[cfg(test)]
mod tests {
    /*use super::*;

    #[derive(Debug)]
    struct Resource(u32);
    #[derive(Copy,Clone,Eq,PartialEq,Hash)]
    struct ResourceDesc(u32);
    new_key_type!{ struct CacheKey; }

    #[test]
    fn test_alloc_free() {
        let mut store = TransientCache::<CacheKey,_,_>::new();
        let key = CacheKey::null();
        let key = store.acquire(0, key, 0, |&d| Resource(d));
        store.release(key);
        // shouldn't reallocate
        let new_key = store.acquire(0, key, 0, |&d| Resource(d));
        assert_eq!(key, new_key);
        store.release(key);
        // should reallocate (different descs)
        let new_key = store.acquire(0, key, 1, |&d| Resource(d));
        assert_ne!(key, new_key);
        // should reallocate (in use, exclusive)
        let newer_key = store.acquire(0, key, 1, |&d| Resource(d));
        assert_ne!(new_key, newer_key);
    }

    #[test]
    fn test_invalid_keys() {
        let store = TransientCache::<CacheKey,Resource,ResourceDesc>::new();
        let key = CacheKey::null();
        assert!(store.get(key).is_none());
    }

    #[test]
    fn test_exclusive() {
        let mut store = TransientCache::<CacheKey,_,_>::new();
        let key = CacheKey::null();
        let key = store.acquire(0, key, 0, |&d| Resource(d));
        // should reallocate
        let new_key = store.acquire(0, key, 0, |&d| Resource(d));
        assert_ne!(key, new_key);
    }*/
}
