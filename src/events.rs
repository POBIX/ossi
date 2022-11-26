use alloc::collections::BTreeMap;

pub trait EventHandler<T : Copy> {
    /// Calls every subscribed handler with the specified args.
    fn invoke(&self, args: T);
    /// func will be invoked when calling invoke().
    /// returns the ID of the function, used to unsubscribe from the event.
    fn subscribe(&mut self, func: fn(T)) -> usize;
    /// Passing the ID returned by a subscribe() call will undo it.
    /// Returns the function on success, None if ID doesn't exist.
    fn unsubscribe(&mut self, id: usize) -> Option<fn(T)>;
}

static mut ID_COUNT: usize = 0;
pub struct Event<T> {
    handlers: BTreeMap<usize, fn(T)>
}

impl<T> Event<T> {
    pub const fn new() -> Event<T> {
        Event::<T> { handlers: BTreeMap::<usize, fn(T)>::new()}
    }
}

impl<T : Copy> EventHandler<T> for Event<T> {
    fn invoke(&self, args: T) {
        for (_, event) in &self.handlers {
            event(args);
        }
    }

    fn subscribe(&mut self, func: fn(T)) -> usize {
        unsafe {
            self.handlers.insert(ID_COUNT, func);
            ID_COUNT += 1;
            ID_COUNT - 1
        }
    }

    fn unsubscribe(&mut self, id: usize) -> Option<fn(T)> {
        self.handlers.remove(&id)
    }
}
