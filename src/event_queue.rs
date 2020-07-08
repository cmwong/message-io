use crossbeam_channel::{self, Sender, Receiver};

use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::time::Duration;
use std::thread::{self, JoinHandle};
use std::collections::{HashMap};

const TIMER_SAMPLING_CHECK: u64 = 50; //ms

pub struct EventQueue<E> {
    receiver: Receiver<E>,
    event_sender: EventSender<E>,
}

impl<E> EventQueue<E>
where E: Send + 'static {
    /// Creates a new event queue for generic incoming events.
    pub fn new() -> EventQueue<E>
    {
        let (sender, receiver) = crossbeam_channel::unbounded();
        EventQueue {
            receiver,
            event_sender: EventSender::new(sender),
        }
    }

    /// Returns the internal sender reference to this queue.
    /// This reference can be safety cloned and shared to other threads in order to make several senders to the same queue.
    pub fn sender(&mut self) -> &mut EventSender<E> {
        &mut self.event_sender
    }

    /// Blocks the current thread until an event is received by this queue.
    pub fn receive(&mut self) -> E {
        self.receiver.recv().unwrap()
    }

    /// Blocks the current thread until an event is received by this queue or timeout is exceeded.
    /// If timeout is reached a None is returned, otherwise the event is returned.
    pub fn receive_event_timeout(&mut self, timeout: Duration) -> Option<E> {
        self.receiver.recv_timeout(timeout).ok()
    }
}


pub struct EventSender<E> {
    sender: Sender<E>,
    timer_registry: HashMap<usize, JoinHandle<()>>,
    timers_running: Arc<AtomicBool>,
    last_timer_id: usize,
}

impl<E> EventSender<E>
where E: Send + 'static {
    fn new(sender: Sender<E>) -> EventSender<E> {
        EventSender {
            sender,
            timer_registry: HashMap::new(),
            timers_running: Arc::new(AtomicBool::new(true)),
            last_timer_id: 0,
        }
    }

    /// Send instantly an event to the event queue.
    pub fn send(&self, event: E) {
        self.sender.send(event).unwrap();
    }

    /// Send a timed event to the [EventQueue].
    /// The event only will be sent after the specific duration,
    /// never before, even it the [EventSender] is dropped.
    pub fn send_with_timer(&mut self, event: E, duration: Duration) {
        let sender = self.sender.clone();
        let timer_id = self.last_timer_id;
        let running = self.timers_running.clone();
        let mut time_acc = Duration::from_secs(0);
        let duration_step = Duration::from_millis(TIMER_SAMPLING_CHECK);
        let timer_handle = thread::spawn(move || {
            while time_acc < duration {
                thread::sleep(duration_step);
                time_acc += duration_step;
                if !running.load(Ordering::Relaxed) {
                    return;
                }
            }
            sender.send(event).unwrap();
        });
        self.timer_registry.insert(timer_id, timer_handle);
        self.last_timer_id += 1;
    }
}

impl<E> Drop for EventSender<E> {
    fn drop(&mut self) {
        self.timers_running.store(false, Ordering::Relaxed);
        for (_, timer) in self.timer_registry.drain() {
            timer.join().unwrap();
        }
    }
}

impl<E> Clone for EventSender<E> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            timer_registry: HashMap::new(),
            timers_running: Arc::new(AtomicBool::new(true)),
            last_timer_id: 0,
        }
    }
}
