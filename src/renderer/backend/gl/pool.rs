use super::buffer::{BufferDescription, RawBuffer};
use super::image::{ImageDescription, RawImage};
use crate::renderer::AliasScope;
use std::marker::PhantomData;

//--------------------------------------------------------------------------------------------------
pub struct AliasedObject<D: Eq + Clone, T> {
    live_scopes: Vec<AliasScope>,
    description: D,
    object: T,
}

impl<D: Eq + Clone, T> AliasedObject<D, T> {
    fn scopes_overlap(&self, scope: &AliasScope) -> bool {
        self.live_scopes.iter().any(|s| s.overlaps(&scope))
    }
}

//--------------------------------------------------------------------------------------------------
/*#[derive(Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[repr(transparent)]
struct AliasKey<T>(slotmap::KeyData, PhantomData<*const T>);

impl<T> slotmap::Key for AliasKey<T> {
}

impl<T> Default for AliasKey<T> {
    fn default() -> Self {
        AliasKey(slotmap::KeyData::default(), PhantomData)
    }
}

impl<T> From<slotmap::KeyData> for AliasKey<T> {
    fn from(k: slotmap::KeyData) -> Self {
        AliasKey(k, PhantomData)
    }
}

impl<T> From<AliasKey<T>> for slotmap::KeyData {
    fn from(k: AliasKey<T>) -> Self {
        k.0
    }
}*/

//--------------------------------------------------------------------------------------------------
pub struct Pool<D: Eq + Clone, K: slotmap::Key + Copy, T> {
    entries: slotmap::SlotMap<K, AliasedObject<D, T>>,
}

impl<D: Eq + Clone, K: slotmap::Key + Copy, T> Pool<D, K, T> {
    pub fn new() -> Pool<D, K, T> {
        Pool {
            entries: slotmap::SlotMap::with_key(),
        }
    }

    pub fn alloc(
        &mut self,
        scope: AliasScope,
        description: D,
        alloc: impl FnOnce(&D) -> T,
    ) -> (K, &T) {
        // scan table to find compatible resource
        // Note: two-step find-return because of a borrow checker limitation
        // (https://github.com/rust-lang/rust/issues/54663)
        let mut found = None;
        for (ck, tr) in self.entries.iter_mut() {
            if tr.description == description && !tr.scopes_overlap(&scope) {
                tr.live_scopes.push(scope.clone());
                found = Some(ck);
                break;
            }
        }

        if let Some(ck) = found {
            (ck, &self.entries.get(ck.clone()).unwrap().object)
        } else {
            // no compatible resource was found: allocate a new one (SLOW PATH)
            let object = alloc(&description);
            let key = self.entries.insert(AliasedObject {
                description: description.clone(),
                live_scopes: vec![scope],
                object,
            });
            (key, &self.entries.get(key.clone()).unwrap().object)
        }
    }

    pub fn destroy(&mut self, key: K, scope: AliasScope, callback: impl FnOnce(T)) {
        let should_remove = if let Some(mut v) = self.entries.get_mut(key.clone()) {
            let pos = v.live_scopes.iter().position(|s| *s == scope);
            if let Some(pos) = pos {
                v.live_scopes.swap_remove(pos);
                v.live_scopes.is_empty()
            } else {
                panic!("invalid scoped resource")
            }
        } else {
            panic!("invalid scoped resource")
        };

        /*if should_remove {
            let v = self.store.remove(key).unwrap();
            callback(v.value)
        }*/
    }

    fn evict<F: FnMut(T)>(&mut self, until_frame: u64, mut deleter: F) {
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

    pub fn get(&self, key: K) -> Option<&T> {
        self.entries.get(key.clone()).map(|e| &e.object)
    }

    pub fn get_mut(&mut self, key: K) -> Option<&mut T> {
        self.entries.get_mut(key.clone()).map(|e| &mut e.object)
    }
}

new_key_type! {
pub struct ImageAliasKey;
pub struct BufferAliasKey;
}

pub type ImagePool = Pool<ImageDescription, ImageAliasKey, RawImage>;
pub type BufferPool = Pool<BufferDescription, BufferAliasKey, RawBuffer>;
