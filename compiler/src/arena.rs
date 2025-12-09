use std::{collections::HashMap, ops};

#[derive(Clone, Copy, Debug)]
pub struct Key(usize);

// simple arena type (using a hashmap probably) that allows me to reuse the same index even when the type changes
// an improved version can use a vec instead, replacing removed elements by tombstones that point to the next free element,
// or the start of the tail
#[derive(Clone, Debug)]
pub struct Arena<T> {
    arena: HashMap<usize, T>,
    uid: usize,
}

#[derive(Clone, Debug)]
pub struct SecondaryArena<T> {
    arena: HashMap<usize, T>,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self { arena: HashMap::new(), uid: 0 }
    }

    pub fn get(&self, key: Key) -> Option<&T> {
        self.arena.get(&key.0)
    }

    pub fn iter(&self) -> impl Iterator::<Item = (Key, &T)> {
        self.arena.iter().map(|(k, v)| (Key(*k), v))
    }

    pub fn insert(&mut self, v: T) -> Key {
        self.uid += 1;
        self.arena.insert(self.uid, v);
        Key(self.uid)
    }

    pub fn insert_with(&mut self, f: impl Fn(Key) -> T) -> Key {
        self.uid += 1;
        self.arena.insert(self.uid, f(Key(self.uid)));
        Key(self.uid)
    }

    pub fn map<U>(self, f: impl Fn(T) -> U) -> Arena<U> {
        Arena {
            arena: self.arena.into_iter().map(|(k, v)| (k, f(v))).collect(),
            uid: self.uid,
        }
    }
}

impl<T> SecondaryArena<T> {
    pub fn new() -> Self {
        Self { arena: HashMap::new() }
    }

    pub fn get(&self, key: Key) -> Option<&T> {
        self.arena.get(&key.0)
    }

    pub fn iter(&self) -> impl Iterator::<Item = (Key, &T)> {
        self.arena.iter().map(|(k, v)| (Key(*k), v))
    }

    pub fn insert(&mut self, k: Key, v: T) {
        self.arena.insert(k.0, v);
    }

    pub fn map<U>(self, f: impl Fn(T) -> U) -> SecondaryArena<U> {
        SecondaryArena {
            arena: self.arena.into_iter().map(|(k, v)| (k, f(v))).collect(),
        }
    }
}

impl<T> ops::Index<Key> for Arena<T> {
    type Output = T;

    fn index(&self, key: Key) -> &Self::Output {
        &self.arena[&key.0]
    }
}

impl<T> ops::IndexMut<Key> for Arena<T> {
    fn index_mut(&mut self, key: Key) -> &mut Self::Output {
        self.arena.get_mut(&key.0).unwrap()
    }
}

impl<T> ops::Index<Key> for SecondaryArena<T> {
    type Output = T;

    fn index(&self, key: Key) -> &Self::Output {
        &self.arena[&key.0]
    }
}

impl<T> ops::IndexMut<Key> for SecondaryArena<T> {
    fn index_mut(&mut self, key: Key) -> &mut Self::Output {
        self.arena.get_mut(&key.0).unwrap()
    }
}
