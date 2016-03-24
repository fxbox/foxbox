//! Implementation of reversible insertions on maps.
//!
//! These utility data structures are useful when several hashmaps/hashsets need to be kept
//! synchronized. For instance, maps a data structure needs to be added to maps `a`, `b`, `c`
//! but the entire operation needs to be cancelled if there is a collision in map `b`.

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::hash::Hash;

/// Insert a (key, value) pair in a map. However, if the object is dropped before method `commit()`
/// is called, the insertion is cancelled.
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
/// use foxbox_taxonomy::transact::InsertInMap;
///
/// let mut map = HashMap::new();
///
/// {
///   let transaction = InsertInMap::start(&mut map, vec![(1, 1)]).unwrap();
///
/// # let some_condition = true;
///   if some_condition {
///     transaction.commit();
///   }
/// }
///
/// // At this stage, if we have not called `transaction.commit()`, the
/// // insertion is cancelled.
/// ```
pub struct InsertInMap<'a, K, V> where K: 'a + Clone + Hash + Eq, V: 'a {
    map: &'a mut HashMap<K, V>,
    committed: bool,
    keys: Vec<K>,
}

impl<'a, K, V> InsertInMap<'a, K, V> where K:'a + Clone + Hash + Eq, V: 'a {
    /// Insert (key, value) pairs in a map, reversibly, and without overwriting.
    ///
    /// If one of the keys `k` is already present in the map, this is a noop, and the
    /// result is `Err(k)`. Otherwise, the result is `Ok(transaction)`. In the latter
    /// case, if `transaction` is dropped before `transaction.commit()` is called,
    /// the insertion is cancelled.
    pub fn start(map: &'a mut HashMap<K, V>, data: Vec<(K, V)>) -> Result<Self, K> {
        // `Some(k)` if we have encountered at least one key `k` that was already
        // present in the map.
        let mut conflict = None;

        // The keys we have successfully inserted so far.
        let mut keys = Vec::with_capacity(data.len());

        // Attempt to insert all the keys. In case of conflict, bailout and
        // rollback the transaction.
        for (k, v) in data {
            match map.entry(k.clone()) {
                Entry::Occupied(_) => {
                    conflict = Some(k);
                    break;
                },
                Entry::Vacant(entry) => {
                    entry.insert(v);
                    keys.push(k)
                }
            }
        }
        match conflict {
            None =>
                Ok(InsertInMap {
                    map: map,
                    keys: keys,
                    committed: false,
                }),
            Some(k) => {
                // We need to rollback everything we have inserted so far.
                for k in keys {
                    map.remove(&k);
                }
                Err(k)
            }
        }
    }

    /// Commit the transaction. Once this is done, the value may be dropped without cancelling
    /// the insertion.
    pub fn commit(mut self) {
        self.committed = true
    }
}

impl<'a, K, V> Drop for InsertInMap<'a, K, V> where K:'a + Clone + Hash + Eq, V: 'a {
    /// If this object is dropped before being committed, cancel the transaction.
    fn drop(&mut self) {
        if self.committed {
            // Transaction has been committed, nothing to do.
            return;
        }
        for k in self.keys.drain(..) {
            // Otherwise, cancel all insertions.
            self.map.remove(&k);
        }
    }
}

#[test]
fn test_transact_map() {
    println!("Initializing a map");
    let mut map = HashMap::new();
    for i in 1..6 {
        map.insert(i, i);
    }
    let reference_map = map.clone();

    println!("Failing to insert due to collision");
    {
        if let Err(i) = InsertInMap::start(&mut map, vec![(6, 6), (7, 7), (8, 8), (4, 10)]) {
            assert_eq!(i, 4);
        } else {
            panic!("We should have detected the collision");
        }
        // Transaction is not committed.
        assert_eq!(map, reference_map);
    }

    println!("Inserting and dropping");
    {
        InsertInMap::start(&mut map, vec![(6, 6), (7, 7), (8, 8)]).unwrap();
        // Transaction is not committed.
        assert_eq!(map, reference_map);
    }

    println!("Inserting and committing");
    {
        {
            let transaction = InsertInMap::start(&mut map, vec![(6, 6), (7, 7), (8, 8)]).unwrap();
            transaction.commit();
        }
        println!("Map: {:?}", map);
        for ((k, v), _) in map.iter().zip(1..9) {
            assert_eq!(k, v);
        }
    }
}
