use std::ops::{Index, IndexMut};

mod base {
    use std::ops::{Index, IndexMut};

    pub struct SimpleMap<V> {
        inner: Vec<Option<V>>,
    }

    impl<V> SimpleMap<V> {
        pub fn with_capacity(capacity: usize) -> Self {
            Self {
                inner: Vec::with_capacity(capacity),
            }
        }

        pub const fn new() -> Self {
            Self { inner: Vec::new() }
        }

        pub fn contains_key(&self, key: usize) -> bool {
            self.inner.get(key).is_some()
        }

        pub fn insert(&mut self, k: usize, v: V) -> Option<V> {
            self.inner.resize_with(k + 1, || None);
            self.inner[k].replace(v)
        }

        pub fn remove(&mut self, k: usize) -> Option<V> {
            let space = self.inner.get_mut(k);
            space.and_then(|space| space.take())
        }

        pub fn get(&self, key: usize) -> Option<&V> {
            self.inner.get(key).and_then(|v| v.as_ref())
        }

        pub fn get_mut(&mut self, key: usize) -> Option<&mut V> {
            self.inner.get_mut(key).and_then(|v| v.as_mut())
        }

        pub fn clear(&mut self) {
            self.inner.clear();
        }

        pub fn iter_mut(&mut self) -> impl Iterator<Item = (usize, &mut V)> {
            self.inner.iter_mut().enumerate().filter_map(into_option)
        }

        pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
            self.inner.iter_mut().filter_map(Option::as_mut)
        }

        pub fn capacity(&self) -> usize {
            self.inner.capacity()
        }
    }

    fn into_option<V>(kv: (usize, &mut Option<V>)) -> Option<(usize, &mut V)> {
        let (k, v) = kv;
        v.as_mut().map(|v| (k, v))
    }

    impl<V> Index<usize> for SimpleMap<V> {
        type Output = V;

        fn index(&self, index: usize) -> &Self::Output {
            self.get(index).expect("no entry found for key")
        }
    }

    impl<V> IndexMut<usize> for SimpleMap<V> {
        fn index_mut(&mut self, index: usize) -> &mut Self::Output {
            self.get_mut(index).expect("no entry found for key")
        }
    }
}

pub struct SimpleMap<V> {
    base: base::SimpleMap<V>,
}

impl<V> SimpleMap<V> {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            base: base::SimpleMap::with_capacity(capacity),
        }
    }

    pub const fn new() -> Self {
        Self {
            base: base::SimpleMap::new(),
        }
    }

    pub fn entry(&mut self, key: usize) -> Entry<'_, V> {
        if self.base.contains_key(key) {
            Entry::Occupied(OccupiedEntry {
                map: &mut self.base,
                key,
            })
        } else {
            Entry::Vacant(VacantEntry {
                map: &mut self.base,
                key,
            })
        }
    }

    pub fn get(&self, key: usize) -> Option<&V> {
        self.base.get(key)
    }

    pub fn get_mut(&mut self, key: usize) -> Option<&mut V> {
        self.base.get_mut(key)
    }

    pub fn contains_key(&self, key: usize) -> bool {
        self.base.contains_key(key)
    }

    pub fn insert(&mut self, k: usize, v: V) -> Option<V> {
        self.base.insert(k, v)
    }

    pub fn clear(&mut self) {
        self.base.clear();
    }

    pub fn remove(&mut self, k: usize) -> Option<V> {
        self.base.remove(k)
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (usize, &mut V)> {
        self.base.iter_mut()
    }

    pub fn capacity(&self) -> usize {
        self.base.capacity()
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.base.values_mut()
    }
}

impl<V> Index<usize> for SimpleMap<V> {
    type Output = V;

    fn index(&self, index: usize) -> &Self::Output {
        self.base.index(index)
    }
}

impl<V> IndexMut<usize> for SimpleMap<V> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.base.index_mut(index)
    }
}

pub enum Entry<'a, V: 'a> {
    Occupied(OccupiedEntry<'a, V>),
    Vacant(VacantEntry<'a, V>),
}

pub struct OccupiedEntry<'a, V: 'a> {
    map: &'a mut base::SimpleMap<V>,
    key: usize,
}

pub struct VacantEntry<'a, V: 'a> {
    map: &'a mut base::SimpleMap<V>,
    key: usize,
}

impl<'a, V> Entry<'a, V> {
    pub fn or_insert(self, default: V) -> &'a mut V {
        match self {
            Self::Occupied(entry) => entry.into_mut(),
            Self::Vacant(entry) => entry.insert(default),
        }
    }

    pub fn or_insert_with<F: FnOnce() -> V>(self, default: F) -> &'a mut V {
        match self {
            Self::Occupied(entry) => entry.into_mut(),
            Self::Vacant(entry) => entry.insert(default()),
        }
    }

    pub fn and_modify<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut V),
    {
        match self {
            Self::Occupied(mut entry) => {
                f(entry.get_mut());
                Self::Occupied(entry)
            }
            Self::Vacant(entry) => Self::Vacant(entry),
        }
    }
}

impl<'a, V: Default> Entry<'a, V> {
    pub fn or_default(self) -> &'a mut V {
        match self {
            Self::Occupied(entry) => entry.into_mut(),
            Self::Vacant(entry) => entry.insert(Default::default()),
        }
    }
}

impl<'a, V> OccupiedEntry<'a, V> {
    pub fn into_mut(self) -> &'a mut V {
        &mut self.map[self.key]
    }

    pub fn get_mut(&mut self) -> &mut V {
        &mut self.map[self.key]
    }

    pub fn get(&self) -> &V {
        &self.map[self.key]
    }

    pub fn insert(self, value: V) -> &'a mut V {
        let _ = self.map.insert(self.key, value);
        &mut self.map[self.key]
    }

    pub fn remove(self) -> V {
        self.map.remove(self.key).expect("should be occupied")
    }
}

impl<'a, V> VacantEntry<'a, V> {
    pub fn insert(self, value: V) -> &'a mut V {
        let _ = self.map.insert(self.key, value);
        &mut self.map[self.key]
    }
}
