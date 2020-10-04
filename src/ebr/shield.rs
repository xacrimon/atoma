use super::global::Global;
use super::local::LocalState;
use crate::deferred::Deferred;
use std::marker::PhantomData;
use std::sync::Arc;

/// Universal methods for any shield implementation.
pub trait Shield<'a>: Clone {
    /// Attempt to synchronize the current thread to allow advancing the global epoch.
    /// This might be useful to call every once in a while if you plan on holding a `Shield`
    /// for an extended amount of time as to not stop garbage collection.
    ///
    /// This is only effective if this is the only active shield created by this thread.
    /// Has no effect when called from an [`unprotected`] shield.
    ///
    /// [`unprotected`]: fn.unprotected.html
    fn repin(&mut self);

    /// Attempt to synchronize the current thread like `Shield::repin` but executing a closure
    /// during the time the `Shield` is temporarily deactivated.
    ///
    /// If this method is called from an [`unprotected`] shield, the closure will be executed
    /// immediately without unpinning the thread.
    ///
    /// [`unprotected`]: fn.unprotected.html
    fn repin_after<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce() -> R;

    /// Schedule a closure for execution once no shield may hold a reference
    /// to an object unlinked with the current shield.
    ///
    /// If this method is called from an [`unprotected`] shield, the closure will be executed
    /// immediately.
    ///
    /// [`unprotected`]: fn.unprotected.html
    fn retire<F>(&self, f: F)
    where
        F: FnOnce() + 'a;
}

/// A `FullShield` is largely equivalent to `ThinShield` in terms of functionality.
/// They're both shields with the same guarantees and can be user interchangeably.
/// The major difference is that `FullShield` implements `Send` and `Sync` while
/// `Shield` does not. `FullShield` is provided for scenarios like asynchronous iteration
/// over a datastructure which is a big pain if the iterator isn't `Send`.
///
/// The downside to this functionality is that they are much more expensive to create and destroy
/// and even more so when multiple threads are creating and destroying them at the same time.
/// This is due to the fact that full shields require more bookeeping to handle the fact
/// that they may suddently change locals/threads.
///
/// While the latency of creation and destruction of a `FullShield` is for the most part
/// relatively constant it does involve accessing state protected by a `Mutex`.
/// This means that in the unfortunate event that a thread gets preempted in this critical section
/// creation and destruction may block. This is in constrast to the wait-free creation and destruction
/// of a `ThinShield`.
///
/// For documentation on functionality please check the documentation of the `Shield` trait.
pub struct FullShield<'a> {
    global: &'a Arc<Global>,
}

impl<'a> FullShield<'a> {
    pub(crate) fn new(global: &'a Arc<Global>) -> Self {
        Self { global }
    }
}

impl<'a> Shield<'a> for FullShield<'a> {
    fn repin(&mut self) {
        // repinning is fine here since we are taking a mutable reference and
        // therefore this shield is not used for anything else
        unsafe {
            self.global.ct.exit(self.global);
            self.global.ct.enter(self.global);
        }
    }

    fn repin_after<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        // see comment on FullShield::repin
        unsafe {
            self.global.ct.exit(self.global);
            let value = f();
            self.global.ct.enter(self.global);
            value
        }
    }

    fn retire<F>(&self, f: F)
    where
        F: FnOnce() + 'a,
    {
        let deferred = Deferred::new(f);
        self.global.retire(deferred, self);
    }
}

impl<'a> Clone for FullShield<'a> {
    fn clone(&self) -> Self {
        Global::full_shield(self.global)
    }
}

impl<'a> Drop for FullShield<'a> {
    fn drop(&mut self) {
        // this is okay since we shall have called enter upon construction of this shield object
        unsafe {
            self.global.ct.exit(self.global);
        }
    }
}

unsafe impl<'a> Send for FullShield<'a> {}
unsafe impl<'a> Sync for FullShield<'a> {}

/// A `ThinShield` locks an epoch and is needed to manipulate protected atomic pointers.
/// It is a type level contract so that you are forces to acquire one before manipulating pointers.
/// This reduces common mistakes drastically since incorrect code will now fail at compile time.
///
/// For documentation on functionality please check the documentation of the `Shield` trait.
pub struct ThinShield<'a> {
    local_state: &'a LocalState,
    _m0: PhantomData<*mut ()>,
}

impl<'a> ThinShield<'a> {
    pub(crate) fn new(local_state: &'a LocalState) -> Self {
        Self {
            local_state,
            _m0: PhantomData,
        }
    }
}

impl<'a> Shield<'a> for ThinShield<'a> {
    // see comment on FullShield::repin
    fn repin(&mut self) {
        unsafe {
            self.local_state.exit();
            self.local_state.enter();
        }
    }

    fn repin_after<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        // see comment on FullShield::repin
        unsafe {
            self.local_state.exit();
            let value = f();
            self.local_state.enter();
            value
        }
    }

    fn retire<F>(&self, f: F)
    where
        F: FnOnce() + 'a,
    {
        let deferred = Deferred::new(f);
        self.local_state.retire(deferred, self);
    }
}

impl<'a> Clone for ThinShield<'a> {
    fn clone(&self) -> Self {
        // since we're creating a new shield we need to also record the creation of it
        unsafe {
            self.local_state.enter();
        }

        Self::new(self.local_state)
    }
}

impl<'a> Drop for ThinShield<'a> {
    fn drop(&mut self) {
        // this is okay since we shall have called enter upon construction of this shield object
        unsafe {
            self.local_state.exit();
        }
    }
}

#[derive(Copy, Clone)]
struct UnprotectedShield;

impl<'a> Shield<'a> for UnprotectedShield {
    fn repin(&mut self) {}

    fn repin_after<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        f()
    }

    fn retire<F>(&self, f: F)
    where
        F: FnOnce() + 'a,
    {
        f();
    }
}

/// Returns a reference to a dummy shield that allows unprotected access to [`Atomic`]s.
///
/// This shield will not keep any thread pinned, it just allows interacting with [`Atomic`]s
/// unsafely.
/// Thus, neither calling [`repin`] nor [`repin_after`] on a shield returned from this function will
/// actually re-pin the current thread. Calling [`repin_after`] or [`retire`] will execute the
/// supplied function immediately.
///
/// # Safety
/// Loading and dereferencing data from an [`Atomic`] using this guard is safe only if the [`Atomic`]
/// is not being concurrently modified by other threads.
///
/// # Examples
/// ```
/// use flize::{self, Atomic, Shared, Shield};
/// use std::sync::atomic::Ordering::Relaxed;
/// use std::mem;
///
/// let a = {
///     let s: Shared<'_, i32> = unsafe { Shared::from_ptr(Box::into_raw(Box::new(7))) };
///     Atomic::new(s)
/// };
///
/// unsafe {
///     // Load `a` without pinning the current thread.
///     let s = a.load(Relaxed, flize::unprotected());
///     assert_eq!(s.as_ref_unchecked(), &7);
///
///     // It is possible to create more unprotected shields with `clone()`.
///     let unprotected = &flize::unprotected().clone();
///     
///     // Swap `a` with a new value (9) without pinning the current thread.
///     let s = Shared::from_ptr(Box::into_raw(Box::new(9)));
///     let s = a.swap(s, Relaxed, unprotected);
///     assert_eq!(a.load(Relaxed, unprotected).as_ref_unchecked(), &9);
///     assert_eq!(s.as_ref_unchecked(), &7);
///
///     let ptr = a.load(Relaxed, unprotected).as_ptr();
///     unprotected.retire(move || {
///         // This is executed immediately, thus `a` now holds an invalid pointer.
///         drop(Box::from_raw(ptr));    
///     });
///     
///     // Dropping `unprotected` doesn't affect the current thread since it did not pin it.
/// }
/// ```
///
/// [`Atomic`]: struct.Atomic.html
/// [`repin`]: trait.Shield.html#method.repin
/// [`repin_after`]: trait.Shield.html#method.repin_after
/// [`retire`]: trait.Shield.html#method.retire
pub unsafe fn unprotected() -> &'static impl Shield<'static> {
    static UNPROTECTED: UnprotectedShield = UnprotectedShield;
    &UNPROTECTED
}

/// This is a utility type that allows you to either take a reference to a shield
/// and be bound by the lifetime of it or take an owned shield use `'static`.
#[derive(Clone)]
pub enum CowShield<'collector, 'shield, S>
where
    S: Shield<'collector>,
{
    Owned(S, PhantomData<&'collector ()>),
    Borrowed(&'shield S),
}

impl<'collector, 'shield, S> CowShield<'collector, 'shield, S>
where
    S: Shield<'collector>,
{
    pub fn new_owned(shield: S) -> Self {
        CowShield::Owned(shield, PhantomData)
    }

    pub fn new_borrowed(shield: &'shield S) -> Self {
        CowShield::Borrowed(shield)
    }

    pub fn into_owned(self) -> S {
        match self {
            CowShield::Owned(shield, _) => shield,
            CowShield::Borrowed(shield) => shield.clone(),
        }
    }

    pub fn get(&self) -> &S {
        match self {
            CowShield::Owned(shield, _) => shield,
            CowShield::Borrowed(shield) => shield,
        }
    }
}
