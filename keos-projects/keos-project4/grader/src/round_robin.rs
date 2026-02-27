use alloc::{collections::VecDeque, format, string::ToString, sync::Arc, vec::Vec};
use keos::{
    MAX_CPU,
    intrinsics::cpuid,
    sync::atomic::{AtomicBool, AtomicUsize},
    thread::{Thread, ThreadBuilder, scheduler::Scheduler},
};
use keos_project4::round_robin::RoundRobin;

/// Tests the scheduler's ability to execute multiple threads in order.
///
/// This test ensures that:
/// - Multiple threads are created and scheduled correctly.
/// - Each thread gets its turn to execute sequentially.
/// - The total number of executed threads matches the expected count.
pub fn functionality() {
    const JOB_CNT: usize = 50;
    let cnt = Arc::new(AtomicUsize::new(0));

    // Spawn `JOB_CNT` threads that execute in order.
    let handles = (0..JOB_CNT)
        .map(|i| {
            let c = cnt.clone();
            ThreadBuilder::new("Waiter").spawn(move || {
                // Wait for the counter to reach `i`, ensuring sequential execution.
                loop {
                    let v = c.load();
                    if v == i {
                        c.fetch_add(1);
                        break;
                    }
                }
                // Spin until all threads complete execution.
                while c.load() != JOB_CNT {
                    core::hint::spin_loop();
                }
            })
        })
        .collect::<Vec<_>>();

    // Ensure all threads complete execution.
    for handle in handles {
        assert_eq!(handle.join(), 0);
    }

    // Verify that the total count matches the expected job count.
    assert_eq!(cnt.load(), JOB_CNT);
}

/// Tests the load balancing of the scheduler by ensuring tasks are distributed
/// across CPUs.
///
/// This test verifies that the scheduler properly distributes workloads among
/// available CPUs. It ensures that:
/// - No single CPU should handle all tasks alone.
/// - All CPU should handle at least one task.
/// - All tasks should be completed successfully.
pub fn balance() {
    const TASK_CNT: usize = MAX_CPU * 10;
    // Atomic counter to track total executed tasks and per-CPU execution counts.
    let test_control = Arc::new((
        AtomicBool::new(true),
        AtomicUsize::new(0),                       // Total executed tasks
        [0; MAX_CPU].map(|_| AtomicUsize::new(0)), // Per-CPU execution counts
    ));
    let mut handles = VecDeque::new();

    // Spawn `TASK_CNT` threads to test load balancing.
    for i in 0..TASK_CNT {
        let test_control = test_control.clone();
        let handle = ThreadBuilder::new(format!("t{i}")).spawn(move || {
            let (barrier, executed, counts) = &*test_control;
            // Wait until all task are pushed.
            while barrier.load() {
                core::hint::spin_loop();
            }

            // Increment the count for the CPU that executes this thread.
            counts[cpuid()].fetch_add(1);
            executed.fetch_add(1);

            // Spin until all tasks are completed.
            while executed.load() != TASK_CNT {
                core::hint::spin_loop();
            }
        });
        handles.push_back(handle);
    }
    test_control.0.store(false);
    keos::thread::scheduler::scheduler().reschedule();

    // Ensure all threads complete execution.
    while let Some(handle) = handles.pop_front() {
        assert_eq!(handle.join(), 0);
    }

    // Verify that no single CPU processed all tasks.
    for count in &test_control.2 {
        let cnt = count.load();
        assert_ne!(
            cnt, TASK_CNT,
            "One CPU handled all tasks, indicating imbalance."
        );
        assert_ne!(
            cnt, 0,
            "One CPU does not handle any task, indicating starvation."
        );
    }
}

/// Tests the workload balancing across multiple CPUs using the Round Robin
/// scheduler.
///
/// This test ensures that:
/// - Each CPU can consume tasks from other CPUs.
pub fn balance2() {
    let task_control = Arc::new((
        AtomicUsize::new(0), // Pinned CPUs
        AtomicUsize::new(0), // Consumed tasks
        AtomicUsize::new(0), // Consumer turn
    ));
    let scheduler = Arc::new(RoundRobin::new());
    let mut handles = VecDeque::new();

    for i in 0..MAX_CPU {
        let task_control = task_control.clone();
        let scheduler = scheduler.clone();
        let handle = ThreadBuilder::new(format!("t{i}")).spawn(move || {
            let (pinned, consumed, turn) = &*task_control;
            // Pin all cores not to be scheduled.
            let _p = Thread::pin();
            let core_id = cpuid();
            pinned.fetch_add(1);

            // Wait until all cores are pinned.
            while pinned.load() != MAX_CPU {
                core::hint::spin_loop();
            }

            // Generate N-1 tasks and assign them to a single core.
            for i in 0..MAX_CPU {
                if core_id == i {
                    // Consuming turn.
                    while turn.load() != core_id {
                        core::hint::spin_loop();
                    }

                    for _ in 0..MAX_CPU - 1 {
                        while scheduler.next_to_run().is_none() {}
                        consumed.fetch_add(1);
                    }

                    turn.fetch_add(1);
                } else {
                    // Spawning turn.
                    scheduler.push_to_queue(Thread::new("task"));
                    let s = turn.load() * (MAX_CPU - 1) + (MAX_CPU - 1);

                    // Ensure tasks are consumed before proceeding.
                    loop {
                        if consumed.load() >= s {
                            break;
                        }
                        core::hint::spin_loop();
                    }
                }
            }
        });
        handles.push_back(handle);
    }

    // Ensure all threads complete execution.
    while let Some(handle) = handles.pop_front() {
        handle.join();
    }
}
/// Tests CPU affinity enforcement in the Round Robin scheduler.
///
/// This test ensures that:
/// - Each CPU only executes tasks that were originally assigned to it if every
///   CPU consumes a task with same ratio.
/// - The scheduler maintains a strict CPU affinity policy.
pub fn affinity() {
    let task_control = Arc::new((
        AtomicUsize::new(0), // Pinned CPUs
        AtomicUsize::new(0), // Spawned tasks
        AtomicUsize::new(0), // Consumed tasks
    ));
    let scheduler = Arc::new(RoundRobin::new());
    let mut handles = VecDeque::new();

    for i in 0..MAX_CPU {
        let task_control = task_control.clone();
        let scheduler = scheduler.clone();
        let handle = ThreadBuilder::new(format!("t{i}")).spawn(move || {
            let (pinned, spawned, consumed) = &*task_control;
            // Pin all cores not to be scheduled.
            let _p = Thread::pin();
            let core_id = cpuid();
            pinned.fetch_add(1);
            // Wait until all cores are pinned.
            while pinned.load() != MAX_CPU {
                core::hint::spin_loop();
            }

            // Task Spawning: Each core sequentially adds tasks to the scheduler.
            loop {
                let spawned_cnt = spawned.load();
                // Stop once enough tasks are created.
                if spawned_cnt >= 10 * MAX_CPU {
                    break;
                }
                // Each core pushes a task in turn.
                if spawned_cnt.checked_rem(MAX_CPU) == Some(core_id) {
                    scheduler.push_to_queue(Thread::new(core_id.to_string()));
                    spawned.fetch_add(1);
                }
            }

            // Task Consumption: Each core executes only its assigned tasks.
            loop {
                let consumed_cnt = consumed.load();
                // Stop when all tasks have been executed.
                if consumed_cnt >= 10 * MAX_CPU {
                    break;
                }
                // Each core takes turns consuming tasks.
                if consumed_cnt.checked_rem(MAX_CPU) == Some(core_id) {
                    // Ensure that the next task belongs to the correct CPU.
                    assert_eq!(
                        scheduler
                            .next_to_run()
                            .and_then(|th| th.name.parse::<usize>().ok())
                            .unwrap(),
                        core_id,
                    );
                    consumed.fetch_add(1);
                }
            }

            // Final validation: Ensure no remaining tasks.
            assert!(scheduler.next_to_run().is_none());
        });
        handles.push_back(handle);
    }

    // Ensure all threads complete execution.
    while let Some(handle) = handles.pop_front() {
        assert_eq!(handle.join(), 0);
    }
}
