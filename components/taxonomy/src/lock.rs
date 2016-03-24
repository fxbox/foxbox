use std::cell::{ RefCell, Ref, RefMut };
use std::marker::PhantomData;
use std::ops::{ Deref, DerefMut };
use std::sync::{ LockResult, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard };
use std::sync::atomic::{ AtomicUsize, Ordering };


pub struct ProofCell<T, U> {
    cell: RefCell<T>,

    // The owner has type BigLock<_> and has a unique key equal to `owner_key`.
    owner_key: usize,

    phantom: PhantomData<U>
}

impl<T, U> ProofCell<T, U> {
    fn new(owner: &Lock<U>, value: T) -> Self {
        ProofCell {
            cell: RefCell::new(value),
            owner_key: owner.ownership,
            phantom: PhantomData
        }
    }
    fn borrow<'a>(&'a self, proof: &Proof) -> Ref<'a, T> {
        assert_eq!(self.owner_key, proof.0);
        self.cell.borrow()
    }
    fn borrow_mut<'a>(&'a self, proof: &ProofMut) -> RefMut<'a, T> {
        assert_eq!(self.owner_key, proof.0);
        self.cell.borrow_mut()
    }
}

/// With respect to Send and Sync, ProofCell behaves as a RwLock.
unsafe impl<T, U> Send for ProofCell<T, U> where T: Send + Sync {
}
unsafe impl<T, U> Sync for ProofCell<T, U> where T: Send + Sync {
}

/// A counter, used to generate unique identifiers for BigCell.
//static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A proof that the BigLock is currently opened.
/// Its lifetime is limited by that of the ReadGuard that provided it.
pub struct Proof(usize);

/// A proof that the BigLock is currently opened mutably.
/// Its lifetime is limited by that of the WriteGuard that provided it.
pub struct ProofMut(usize);

pub struct ReadGuard<'a, T> where T: 'a {
    proof: Proof,
    guarded: &'a T
}
impl<'a, T> ReadGuard<'a, T> {
    pub fn get(&'a self) -> (&'a Proof, &'a T) {
        (&self.proof, self.guarded)
    }
}

pub struct WriteGuard<'a, T> where T: 'a {
    proof: ProofMut,
    guarded: &'a mut T
}
impl<'a, T> WriteGuard<'a, T> {
    pub fn get(&'a self) -> (&'a ProofMut, &'a mut T) {
        (&self.proof, self.guarded)
    }
}

pub struct Lock<T> {
    lock: RwLock<T>,
    ownership: usize,
}
impl<T> Lock<T> {
    fn new(value: T) -> Self {
        use std::mem;
        let ownership : usize = unsafe { mem::transmute(&value as *const T) };
        Lock {
            lock: RwLock::new(value),
            ownership: ownership
        }
    }

    fn read(&self) -> LockResult<ReadGuard<T>> {
        match self.lock.read() {
            Ok(ok) => Ok(ReadGuard {
                proof: Proof(self.ownership),
                guarded: ok.deref()
            }),
            _ => unimplemented!()
        }
    }

    fn write(&self) -> LockResult<WriteGuard<T>> {
        match self.lock.write() {
            Ok(ok) => Ok(WriteGuard {
                proof: ProofMut(self.ownership),
                guarded: ok.deref_mut()
            }),
            _ => unimplemented!()
        }
    }

}