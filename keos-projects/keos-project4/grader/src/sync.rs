pub mod mutex {
    use alloc::{sync::Arc, vec::Vec};
    use keos::{
        sync::atomic::{AtomicBool, AtomicUsize},
        thread::{ThreadBuilder, ThreadState},
    };
    use keos_project4::sync::mutex::Mutex;

    pub fn smoke() {
        const LENGTH: usize = 64;
        let output = Arc::new(Mutex::new(Vec::new()));
        let counter = Arc::new(AtomicUsize::new(0));
        let counter2 = Arc::new(AtomicUsize::new(0));

        for i in 0..LENGTH {
            let counter = counter.clone();
            let counter2 = counter2.clone();
            let output = output.clone();
            ThreadBuilder::new("smoker").spawn(move || {
                counter.fetch_add(1);
                let mut d = output.lock();
                while counter.load() != LENGTH {}
                d.push(i);
                counter2.fetch_add(1);
                d.unlock();
            });
        }

        while counter2.load() != LENGTH {}

        let mut output = output.lock();
        output.sort();
        assert_eq!(&*output, &(0..LENGTH).collect::<Vec<_>>());
        output.unlock();
    }

    pub fn smoke_many() {
        for i in 0..100 {
            keos::println!("Iteration #{i:}");
            smoke()
        }
    }

    pub fn parking() {
        let mutex = Arc::new(Mutex::new(()));
        let guard = mutex.lock();
        let thread_spawned = Arc::new(AtomicBool::new(false));

        let be_parked = {
            let (thread_spawned, mutex) = (thread_spawned.clone(), mutex.clone());
            ThreadBuilder::new("blockee").spawn(move || {
                thread_spawned.store(true);
                let guard = mutex.lock();
                guard.unlock();
            })
        };

        while !thread_spawned.load() {
            core::hint::spin_loop();
        }
        for _ in 0..10000 {
            core::hint::spin_loop();
        }

        assert_eq!(
            keos::thread::get_state_by_tid(be_parked.tid),
            Ok(ThreadState::Parked),
            "Blocked thread by Mutex should be in Parked state"
        );

        guard.unlock();
        be_parked.join();
    }
}

pub mod condition_variable {
    use alloc::{sync::Arc, vec::Vec};
    use keos::{MAX_CPU, sync::atomic::AtomicUsize, thread::ThreadBuilder};
    use keos_project4::sync::{condition_variable::ConditionVariable, mutex::Mutex};

    const MAX: usize = 2;
    struct BufferInner {
        item: [usize; MAX],
        front: usize,
        tail: usize,
    }
    impl BufferInner {
        fn is_full(&self) -> bool {
            self.tail.overflowing_sub(self.front).0 % MAX == MAX - 1
        }
        fn is_empty(&self) -> bool {
            self.front == self.tail
        }
    }
    struct Buffer {
        inner: Mutex<BufferInner>,
        full: ConditionVariable,
        empty: ConditionVariable,
    }

    impl Buffer {
        fn put(&self, val: usize) {
            let mut guard = self.full.wait_while(&self.inner, |b| b.is_full());
            let tail = (guard.tail + 1) % MAX;
            guard.tail = tail;
            guard.item[tail] = val;
            self.empty.signal(guard);
        }

        fn put_many<const T: usize>(&self, val: [usize; T]) {
            let mut idx = 0;
            while idx < T {
                let mut guard = self.full.wait_while(&self.inner, |b| b.is_full());
                while !guard.is_full() && idx < T {
                    let tail = (guard.tail + 1) % MAX;
                    guard.tail = tail;
                    guard.item[tail] = val[idx];
                    idx += 1;
                }
                self.empty.broadcast(guard);
            }
        }

        fn get(&self) -> usize {
            let mut guard = self.empty.wait_while(&self.inner, |b| b.is_empty());
            let front = (guard.front + 1) % MAX;
            let item = guard.item[front];
            guard.front = front;
            self.full.signal(guard);
            item
        }
    }

    pub fn bounded_buffer_1() {
        let (buffer, waiters, output) = (
            Arc::new(Buffer {
                inner: Mutex::new(BufferInner {
                    item: [0; MAX],
                    front: 0,
                    tail: 0,
                }),
                full: ConditionVariable::new(),
                empty: ConditionVariable::new(),
            }),
            Arc::new(Arc::new(AtomicUsize::new(0))),
            Arc::new(Mutex::new(Vec::new())),
        );

        let consumers = [0; MAX_CPU * 2 + 2].map(|_| {
            let (buffer, waiters, output) = (buffer.clone(), waiters.clone(), output.clone());
            ThreadBuilder::new("consumer").spawn(move || {
                waiters.fetch_add(1);
                let d = buffer.get();
                let mut guard = output.lock();
                guard.push(d);
                guard.unlock();
            })
        });
        while waiters.load() != MAX_CPU * 2 + 2 {}

        let producer = {
            let buffer = buffer.clone();
            ThreadBuilder::new("producer").spawn(move || {
                for i in 0..MAX_CPU * 2 + 2 {
                    buffer.put(i);
                }
            })
        };

        for consumer in consumers {
            consumer.join();
        }
        producer.join();

        let mut output = output.lock();
        output.sort();
        assert_eq!(&*output, &(0..MAX_CPU * 2 + 2).collect::<Vec<_>>());
        output.unlock();
    }

    pub fn bounded_buffer_2() {
        let (buffer, waiters, output) = (
            Arc::new(Buffer {
                inner: Mutex::new(BufferInner {
                    item: [0; MAX],
                    front: 0,
                    tail: 0,
                }),
                full: ConditionVariable::new(),
                empty: ConditionVariable::new(),
            }),
            Arc::new(Arc::new(AtomicUsize::new(0))),
            Arc::new(Mutex::new(Vec::new())),
        );

        let consumers = [0; MAX_CPU * 2 + 2].map(|_| {
            let (buffer, waiters, output) = (buffer.clone(), waiters.clone(), output.clone());
            ThreadBuilder::new("consumer").spawn(move || {
                waiters.fetch_add(1);
                let d = buffer.get();
                let mut guard = output.lock();
                guard.push(d);
                guard.unlock();
            })
        });
        while waiters.load() != MAX_CPU * 2 + 2 {}
        for _ in 0..10000000 {
            core::hint::black_box(());
        }

        let producer = {
            let buffer = buffer.clone();
            ThreadBuilder::new("producer").spawn(move || {
                for i in (0..MAX_CPU * 2 + 2).array_chunks::<{ MAX_CPU / 2 }>() {
                    buffer.put_many(i);
                }
            })
        };

        for consumer in consumers {
            consumer.join();
        }
        producer.join();
        let mut output = output.lock();
        output.sort();
        assert_eq!(&*output, &(0..MAX_CPU * 2 + 2).collect::<Vec<_>>());
        output.unlock();
    }
}

pub mod semaphore {
    use alloc::sync::Arc;
    use core::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
    use keos::thread::ThreadBuilder;
    use keos_project4::sync::{Mutex, Semaphore};

    pub fn sema_0() {
        let sema = Arc::new(Semaphore::new(0, ()));
        let is_thread_active = Arc::new(AtomicBool::new(false));
        let is_woken_up = Arc::new(AtomicBool::new(false));

        let thread = {
            let (sema, is_thread_active, is_woken_up) =
                (sema.clone(), is_thread_active.clone(), is_woken_up.clone());

            ThreadBuilder::new("worker").spawn(move || {
                is_thread_active.store(true, Ordering::SeqCst);
                sema.wait();
                is_woken_up.store(true, Ordering::SeqCst);
            })
        };

        while !is_thread_active.load(Ordering::SeqCst) {
            core::hint::spin_loop();
        }

        assert!(!is_woken_up.load(Ordering::SeqCst));

        sema.signal();
        thread.join();

        assert!(is_woken_up.load(Ordering::SeqCst));
    }

    pub fn sema_1() {
        const COUNT: u32 = 16;
        let sema = Arc::new(Semaphore::new(1, ()));
        let ready_counter = Arc::new(AtomicU32::new(0));
        let counter = Arc::new(AtomicU32::new(0));

        for i in 0..COUNT {
            let (sema, ready_counter, counter) =
                (sema.clone(), ready_counter.clone(), counter.clone());

            ThreadBuilder::new(alloc::format!("t{i}")).spawn(move || {
                ready_counter.fetch_add(1, Ordering::SeqCst);

                let sema_permit = sema.wait();
                counter.fetch_add(1, Ordering::SeqCst);
                core::mem::forget(sema_permit);
            });
        }

        while ready_counter.load(Ordering::SeqCst) < COUNT {
            core::hint::spin_loop();
        }

        let mut expected_cnt = 1;

        while counter.load(Ordering::SeqCst) < COUNT {
            expected_cnt += 1;
            sema.signal();
            for _ in 0..10000 {
                core::hint::spin_loop();
            }

            assert_eq!(counter.load(Ordering::SeqCst), expected_cnt);
        }
    }

    pub fn sema_2() {
        const COUNT: u32 = 16;
        let sema = Arc::new(Semaphore::new(2, ()));
        let ready_counter = Arc::new(AtomicU32::new(0));
        let counter = Arc::new(AtomicU32::new(0));

        for i in 0..COUNT {
            let (sema, ready_counter, counter) =
                (sema.clone(), ready_counter.clone(), counter.clone());

            ThreadBuilder::new(alloc::format!("t{i}")).spawn(move || {
                ready_counter.fetch_add(1, Ordering::SeqCst);

                let sema_permit = sema.wait();
                counter.fetch_add(1, Ordering::SeqCst);
                core::mem::forget(sema_permit);
            });
        }

        while ready_counter.load(Ordering::SeqCst) < COUNT {
            core::hint::spin_loop();
        }

        let mut expected_cnt = 2;

        while counter.load(Ordering::SeqCst) < COUNT {
            expected_cnt += 2;
            sema.signal();
            sema.signal();
            for _ in 0..10000 {
                core::hint::spin_loop();
            }

            assert_eq!(counter.load(Ordering::SeqCst), expected_cnt);
        }
    }

    pub fn exec_order() {
        const COUNT: usize = 3;

        let counter = Arc::new(AtomicUsize::new(0));
        let counter2 = Arc::new(AtomicUsize::new(0));
        let sema = Arc::new(Semaphore::new(0, 0));

        for i in 0..COUNT {
            let counter = counter.clone();
            let counter2 = counter2.clone();
            let sema = sema.clone();
            ThreadBuilder::new(alloc::format!("support_{}", i)).spawn(move || {
                counter2.fetch_add(1, Ordering::SeqCst);
                let _guard = sema.wait();
                counter.fetch_add(1, Ordering::SeqCst);
            });
        }

        while counter2.load(Ordering::SeqCst) != COUNT {}
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    pub fn n_permits() {
        const COUNT: usize = 5;
        const PERMITS: usize = 3;

        let counter = Arc::new(AtomicUsize::new(0));
        let counter2 = Arc::new(AtomicUsize::new(0));
        let sema = Arc::new(Semaphore::new(PERMITS, 0));
        let lock = Arc::new(Mutex::new(0));

        let guard = lock.lock();

        for i in 0..COUNT {
            let counter = counter.clone();
            let counter2 = counter2.clone();
            let sema = sema.clone();
            let lock = lock.clone();
            ThreadBuilder::new(alloc::format!("support_{}", i)).spawn(move || {
                counter2.fetch_add(1, Ordering::SeqCst);
                let _guard = sema.wait();
                counter.fetch_add(1, Ordering::SeqCst);
                let _lock = lock.lock();
                _lock.unlock();
            });
        }

        while counter2.load(Ordering::SeqCst) != COUNT {}
        assert_eq!(counter.load(Ordering::SeqCst), PERMITS);
        guard.unlock();
    }
}
