use futures::{task::AtomicWaker, Future};
use ic_kit::prelude::*;
use std::{
    cell::RefCell,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::{Context, Poll},
};

struct Inner {
    waker: AtomicWaker,
    set: AtomicBool,
}

#[derive(Clone)]
pub struct Flag(Arc<Inner>);

impl Flag {
    pub fn new() -> Self {
        Self(Arc::new(Inner {
            waker: AtomicWaker::new(),
            set: AtomicBool::new(false),
        }))
    }

    pub fn signal(&self) {
        self.0.set.store(true, Ordering::Relaxed);
        self.0.waker.wake();
    }
}

impl Future for Flag {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        // quick check to avoid registration if already done.
        if self.0.set.load(Ordering::Relaxed) {
            return Poll::Ready(());
        }

        self.0.waker.register(cx.waker());

        // Need to check condition **after** `register` to avoid a race
        // condition that would result in lost notifications.
        if self.0.set.load(Ordering::Relaxed) {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

pub struct Flags {
    flags: RefCell<Vec<Flag>>,
}

impl Default for Flags {
    fn default() -> Self {
        Flags {
            flags: RefCell::new(vec![]),
        }
    }
}

impl Flags {
    pub fn take(&mut self) -> Vec<Flag> {
        self.flags.take()
    }

    pub fn insert(&mut self) -> Flag {
        let flag = Flag::new();
        self.flags.borrow_mut().push(flag.clone());
        flag
    }
}

#[update]
pub async fn wait() {
    let flag = ic::with_mut(Flags::insert);
    flag.await

    // Expected: method awaits for signal call and then returns
    // Actual: immediate rejection
    //    Call was rejected
    //      - Reject code: 5
    //      - Reject text: Canister rrkah-fqaaa-aaaaa-aaaaq-cai did not reply to the call
    //
    // Whats going on here?
    //    The wasm runtime does not properly await, so the signal triggers our future, but
    //    the calls context has already been "replied" to, and we violate the wasm contract.
}

#[update]
pub async fn signal() {
    let flags = ic::with_mut(Flags::take);
    for flag in flags {
        flag.signal();
    }
}

#[derive(KitCanister)]
#[candid_path("playground.did")]
pub struct PlaygroundCanister;
