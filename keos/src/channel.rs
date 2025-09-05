//! Multi-producer, multi-consumer FIFO queue communication primitives.
//!
//! This module provides message-based communication over channels, concretely
//! defined among two types:
//!
//! * [`Sender`]
//! * [`Receiver`]
//!
//! A [`Sender`] is used to send data to a [`Receiver`]. Both
//! sender and receiver are clone-able (multi-producer) such that many threads
//! can send simultaneously to multiple receiver (multi-consumer).
//!
//!
//! [`send`]: Sender::send
//!
//! ## Disconnection
//!
//! The send and receive operations on channels will all return a `Result`
//! indicating whether the operation succeeded or not. An unsuccessful operation
//! is normally indicative of the other half of a channel having "hung up" by
//! being dropped in its corresponding thread.
//!
//! Once half of a channel has been deallocated, most operations can no longer
//! continue to make progress, so `Err` will be returned. Many applications
//! will continue to `unwrap` the results returned from this module,
//! instigating a propagation of failure among threads if one unexpectedly dies.

use crate::{
    spinlock::SpinLock,
    thread::{Current, ParkHandle},
};
use alloc::{boxed::Box, vec::Vec};
use core::{
    fmt,
    sync::atomic::{AtomicUsize, Ordering},
};
use crossbeam_queue::ArrayQueue;

pub(crate) struct ChannelInner<T> {
    pub q: ArrayQueue<T>,
    pub tx_cnt: AtomicUsize,
    pub rx_cnt: AtomicUsize,
    tx_waiter: SpinLock<Vec<ParkHandle>>,
    rx_waiter: SpinLock<Vec<ParkHandle>>,
}

impl<T> ChannelInner<T> {
    #[inline]
    pub fn has_receiver(&self) -> bool {
        self.rx_cnt.load(Ordering::Acquire) != 0
    }

    #[inline]
    pub fn has_sender(&self) -> bool {
        self.tx_cnt.load(Ordering::Acquire) != 0
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.q.capacity()
    }

    pub fn push(
        &self,
        value: T,
        do_unpark: impl Fn(ParkHandle) -> Result<(), ()>,
    ) -> Result<(), T> {
        match self.q.push(value) {
            Ok(_) => {
                let mut guard = self.rx_waiter.lock();
                if let Some(th) = guard.pop() {
                    do_unpark(th).expect("Failed to unpark channel tx waiter.")
                }
                guard.unlock();
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
    pub fn pop(&self, do_unpark: impl Fn(ParkHandle) -> Result<(), ()>) -> Option<T> {
        match self.q.pop() {
            Some(v) => {
                let mut guard = self.tx_waiter.lock();

                if let Some(th) = guard.pop() {
                    do_unpark(th).expect("Failed to unpark channel rx waiter.")
                }
                guard.unlock();
                Some(v)
            }
            None => None,
        }
    }
}

/// The receiving half of [`channel`] type.
///
/// This half can only be owned by one thread, but it can be cloned to receive
/// to other threads.
///
/// Messages sent to the channel can be retrieved using [`recv`].
///
/// [`recv`]: Receiver::recv
pub struct Receiver<T: core::marker::Send + 'static> {
    inner: *mut ChannelInner<T>,
}

// The receiver port can be sent from place to place, so long as it
// is not used to receive non-sendable things.
unsafe impl<T: Send> Send for Receiver<T> {}
unsafe impl<T: Send> Sync for Receiver<T> {}

/// An iterator over messages on a [`Receiver`], created by [`iter`].
///
/// This iterator will block whenever `next` is called,
/// waiting for a new message, and `None` will be returned
/// when the corresponding channel has hung up.
///
/// [`iter`]: Receiver::iter
#[derive(Debug)]
pub struct Iter<'a, T: core::marker::Send + 'static> {
    rx: &'a Receiver<T>,
}

/// An iterator that attempts to yield all pending values for a [`Receiver`],
/// created by [`try_iter`].
///
/// `None` will be returned when there are no pending values remaining or
/// if the corresponding channel has hung up.
///
/// This iterator will never block the caller in order to wait for data to
/// become available. Instead, it will return `None`.
///
/// [`try_iter`]: Receiver::try_iter
#[derive(Debug)]
pub struct TryIter<'a, T: core::marker::Send + 'static> {
    rx: &'a Receiver<T>,
}

/// An owning iterator over messages on a [`Receiver`],
/// created by [`Receiver::into_iter`].
///
/// This iterator will block whenever [`next`]
/// is called, waiting for a new message, and `None` will be
/// returned if the corresponding channel has hung up.
///
/// [`next`]: Iterator::next
#[derive(Debug)]
pub struct IntoIter<T: core::marker::Send + 'static> {
    rx: Receiver<T>,
}

/// The sending-half of [`channel`] type.
///
/// This half can only be owned by one thread, but it can be cloned to send to
/// other threads.
///
/// Messages can be sent through this channel with [`send`].
///
/// [`send`]: Sender::send
pub struct Sender<T: core::marker::Send + 'static> {
    inner: *mut ChannelInner<T>,
}

// The send port can be sent from place to place, so long as it
// is not used to send non-sendable things.
unsafe impl<T: Send> Send for Sender<T> {}
unsafe impl<T: Send> Sync for Sender<T> {}

/// An error returned from the [`Sender::send`] function on **channel**s.
///
/// A **send** operation can only fail if the receiving end of a channel is
/// disconnected, implying that the data could never be received. The error
/// contains the data being sent as a payload so it can be recovered.
///
/// [`Sender::send`]: Sender::send
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct SendError<T>(pub T);

/// An error returned from the [`recv`] function on a [`Receiver`].
///
/// The [`recv`] operation can only fail if the sending half of a
/// [`channel`] is disconnected, implying that no further
/// messages will ever be received.
///
/// [`recv`]: Receiver::recv
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct RecvError;

/// The list of the possible error outcomes for the [`try_send`] method.
///
/// [`try_send`]: Sender::try_send
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum TrySendError<T> {
    /// The data could not be sent on the [`channel`] because it would
    /// require that the callee block to send the data.
    ///
    /// If this is a buffered channel, then the buffer is full at this time. If
    /// this is not a buffered channel, then there is no [`Receiver`] available
    /// to acquire the data.
    Full(T),

    /// This [`channel`]'s receiving half has disconnected, so the data
    /// could not be sent. The data is returned back to the callee in this
    /// case.
    Disconnected(T),
}

/// The list of the possible reasons that [`try_recv`] could not return data
/// when called.
///
/// This can occur with a [`channel`].
///
/// [`try_recv`]: Receiver::try_recv
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum TryRecvError {
    /// This **channel** is currently empty, but the **Sender**(s) have not yet
    /// disconnected, so data may yet become available.
    Empty,

    /// The **channel**'s sending half has become disconnected, and there will
    /// never be any more data received on it.
    Disconnected,
}

/// Creates a new bounded channel.
///
/// All data sent on the [`Sender`] will become available on the [`Receiver`]
/// in the same order as it was sent. Like asynchronous [`channel`]s, the
/// [`Receiver`] will block until a message becomes available.
///
/// This channel has an internal buffer on which messages will be queued.
/// `bound` specifies the buffer size. When the internal buffer becomes full,
/// future sends will *block* waiting for the buffer to open up. Note that a
/// buffer size of 0 is valid, in which case this becomes "rendezvous channel"
/// where each [`send`] will not return until a [`recv`] is paired with it.
///
/// Both [`Sender`] and [`Receiver`] can be cloned to [`send`] or [`recv`] to
/// the same channel multiple times.
///
/// If the [`Receiver`] is disconnected while trying to [`send`] with the
/// [`Sender`], the [`send`] method will return a [`SendError`]. Similarly, If
/// the [`Sender`] is disconnected while trying to [`recv`], the [`recv`] method
/// will return a [`RecvError`].
///
/// [`send`]: Sender::send
/// [`recv`]: Receiver::recv
pub fn channel<T: core::marker::Send + 'static>(bound: usize) -> (Sender<T>, Receiver<T>) {
    let chan = Box::into_raw(Box::new(ChannelInner {
        q: ArrayQueue::new(bound),
        tx_cnt: AtomicUsize::new(1),
        rx_cnt: AtomicUsize::new(1),
        tx_waiter: SpinLock::new(Vec::new()),
        rx_waiter: SpinLock::new(Vec::new()),
    }));
    (Sender { inner: chan }, Receiver { inner: chan })
}

////////////////////////////////////////////////////////////////////////////////
// Sender
////////////////////////////////////////////////////////////////////////////////
impl<T: core::marker::Send + 'static> Sender<T> {
    #[inline]
    fn inner<'a>(&self) -> &'a ChannelInner<T> {
        unsafe { &*self.inner }
    }

    /// Can send a value through this channel.
    pub fn can_send(&self) -> bool {
        let inner = self.inner();
        inner.q.is_empty() && inner.has_receiver()
    }

    /// Does anyone can receive a value through this channel.
    pub fn has_receiver(&self) -> bool {
        let inner = self.inner();
        inner.has_receiver()
    }

    /// Sends a value on this channel.
    ///
    /// This function will *block* until space in the internal buffer becomes
    /// available or a receiver is available to hand off the message to.
    ///
    /// Note that a successful send does *not* guarantee that the receiver will
    /// ever see the data if there is a buffer on this channel. Items may be
    /// enqueued in the internal buffer for the receiver to receive at a later
    /// time. If the buffer size is 0, however, the channel becomes a rendezvous
    /// channel and it guarantees that the receiver has indeed received
    /// the data if this function returns success.
    ///
    /// This function will never panic, but it may return `Err` if the
    /// [`Receiver`] has disconnected and is no longer able to receive
    /// information.
    pub fn send(&self, t: T) -> Result<(), SendError<T>> {
        let inner = self.inner();
        let mut t_ = t;
        loop {
            if !inner.has_receiver() {
                break Err(SendError(t_));
            } else {
                match inner.push(t_, |th| {
                    th.unpark();
                    Ok(())
                }) {
                    Err(e) => {
                        t_ = e;
                        if inner.q.is_full() {
                            let mut guard = inner.tx_waiter.lock();
                            if inner.q.is_full() {
                                Current::park_with(move |th| {
                                    guard.push(th);
                                    drop(guard)
                                });
                            }
                        }
                    }
                    _ => {
                        break Ok(());
                    }
                }
            }
        }
    }

    /// Attempts to send a value on this channel without blocking.
    ///
    /// This method differs from [`send`] by returning immediately if the
    /// channel's buffer is full or no receiver is waiting to acquire some
    /// data. Compared with [`send`], this function has two failure cases
    /// instead of one (one for disconnection, one for a full buffer).
    ///
    /// See [`send`] for notes about guarantees of whether the
    /// receiver has received the data or not if this function is successful.
    ///
    /// [`send`]: Self::send
    pub fn try_send(&self, t: T) -> Result<(), TrySendError<T>> {
        let inner = self.inner();
        if !inner.has_receiver() {
            Err(TrySendError::Disconnected(t))
        } else {
            match inner.push(t, |th| {
                th.unpark();
                Ok(())
            }) {
                Err(t) => Err(TrySendError::Full(t)),
                _ => Ok(()),
            }
        }
    }

    /// Get capacity of the channel.
    pub fn capacity(&self) -> usize {
        self.inner().capacity()
    }
}

impl<T: core::marker::Send + 'static> Clone for Sender<T> {
    fn clone(&self) -> Sender<T> {
        if self.inner().tx_cnt.fetch_add(1, Ordering::Relaxed) > isize::MAX as usize {
            panic!("sender count overflowed.");
        }
        Sender { inner: self.inner }
    }
}

impl<T: core::marker::Send + 'static> Drop for Sender<T> {
    fn drop(&mut self) {
        let inner = self.inner();
        if inner.tx_cnt.fetch_sub(1, Ordering::AcqRel) == 1
            && inner.rx_cnt.load(Ordering::Relaxed) == 0
        {
            unsafe { drop(Box::from_raw(self.inner)) }
        }
    }
}

impl<T: core::marker::Send + 'static> fmt::Debug for Sender<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sender").finish()
    }
}

////////////////////////////////////////////////////////////////////////////////
// Receiver
////////////////////////////////////////////////////////////////////////////////

impl<T: core::marker::Send + 'static> Receiver<T> {
    #[inline]
    fn inner<'a>(&self) -> &'a ChannelInner<T> {
        unsafe { &*self.inner }
    }

    /// Can receive a value through this channel.
    pub fn can_recv(&self) -> bool {
        let inner = self.inner();
        !inner.q.is_empty() && inner.has_sender()
    }

    /// Does anyone can send a value through this channel.
    pub fn has_sender(&self) -> bool {
        let inner = self.inner();
        inner.has_sender()
    }

    /// Attempts to wait for a value on this receiver, returning an error if the
    /// corresponding channel has hung up.
    ///
    /// This function will always block the current thread if there is no data
    /// available and it's possible for more data to be sent. Once a message is
    /// sent to the corresponding [`Sender`] (or [`Sender`]), then this
    /// receiver will wake up and return that message.
    ///
    /// If the corresponding [`Sender`] has disconnected, or it disconnects
    /// while this call is blocking, this call will wake up and return
    /// `Err` to indicate that no more messages can ever be received on
    /// this channel. However, since channels are buffered, messages sent
    /// before the disconnect will still be properly received.
    pub fn recv(&self) -> Result<T, RecvError> {
        let inner = self.inner();
        loop {
            match inner.pop(|th| {
                th.unpark();
                Ok(())
            }) {
                Some(n) => break Ok(n),
                None if !inner.has_sender() => {
                    break inner
                        .pop(|th| {
                            th.unpark();
                            Ok(())
                        })
                        .ok_or(RecvError);
                }
                None => {
                    let mut guard = inner.rx_waiter.lock();
                    match inner.pop(|th| {
                        th.unpark();
                        Ok(())
                    }) {
                        Some(n) => {
                            guard.unlock();
                            break Ok(n);
                        }
                        _ => {
                            Current::park_with(|handle| {
                                guard.push(handle);
                                guard.unlock();
                            });
                        }
                    }
                }
            }
        }
    }

    /// Attempts to return a pending value on this receiver without blocking.
    ///
    /// This method will never block the caller in order to wait for data to
    /// become available. Instead, this will always return immediately with a
    /// possible option of pending data on the channel.
    ///
    /// This is useful for a flavor of "optimistic check" before deciding to
    /// block on a receiver.
    ///
    /// Compared with [`recv`], this function has two failure cases instead of
    /// one (one for disconnection, one for an empty buffer).
    ///
    /// [`recv`]: Self::recv
    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        let inner = self.inner();
        match inner.pop(|th| {
            th.unpark();
            Ok(())
        }) {
            Some(n) => Ok(n),
            None if !inner.has_sender() => inner
                .pop(|th| {
                    th.unpark();
                    Ok(())
                })
                .ok_or(TryRecvError::Disconnected),
            None => Err(TryRecvError::Empty),
        }
    }

    /// Returns an iterator that will block waiting for messages, but never
    /// panicked. It will return `None` when the channel has hung up.
    pub fn iter(&self) -> Iter<'_, T> {
        Iter { rx: self }
    }

    /// Returns an iterator that will attempt to yield all pending values.
    /// It will return `None` if there are no more pending values or if the
    /// channel has hung up. The iterator will never panicked or block the
    /// user by waiting for values.
    pub fn try_iter(&self) -> TryIter<'_, T> {
        TryIter { rx: self }
    }

    /// Get capacity of the channel.
    pub fn capacity(&self) -> usize {
        self.inner().capacity()
    }
}

impl<T: core::marker::Send + 'static> Iterator for Iter<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.rx.recv().ok()
    }
}

impl<T: core::marker::Send + 'static> Iterator for TryIter<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.rx.try_recv().ok()
    }
}

impl<'a, T: core::marker::Send + 'static> IntoIterator for &'a Receiver<T> {
    type Item = T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}

impl<T: core::marker::Send + 'static> Iterator for IntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.rx.recv().ok()
    }
}

impl<T: core::marker::Send + 'static> IntoIterator for Receiver<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> IntoIter<T> {
        IntoIter { rx: self }
    }
}

impl<T: core::marker::Send + 'static> Clone for Receiver<T> {
    fn clone(&self) -> Receiver<T> {
        if self.inner().rx_cnt.fetch_add(1, Ordering::Relaxed) > isize::MAX as usize {
            panic!("receiver count overflowed.");
        }
        Receiver { inner: self.inner }
    }
}

impl<T: core::marker::Send + 'static> Drop for Receiver<T> {
    fn drop(&mut self) {
        let inner = self.inner();
        if inner.rx_cnt.fetch_sub(1, Ordering::AcqRel) == 1
            && inner.tx_cnt.load(Ordering::Relaxed) == 0
        {
            unsafe { drop(Box::from_raw(self.inner)) }
        }
    }
}

impl<T: core::marker::Send + 'static> fmt::Debug for Receiver<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Receiver").finish()
    }
}
