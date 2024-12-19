use crate::core::orderbook::OrderBook;
use std::sync::atomic::{AtomicPtr, Ordering};

pub struct Manager {
    primary: AtomicPtr<OrderBook>,
    secondary: AtomicPtr<OrderBook>,
}

impl Default for Manager {
    fn default() -> Self {
        Self::new()
    }
}

impl Manager {
    pub fn new() -> Manager {
        let primary = Box::into_raw(Box::new(OrderBook::default()));
        let secondary = Box::into_raw(Box::new(OrderBook::default()));
        Manager {
            primary: AtomicPtr::new(primary),
            secondary: AtomicPtr::new(secondary),
        }
    }

    pub fn get_primary(&self) -> *mut OrderBook {
        self.primary.load(Ordering::SeqCst)
    }

    pub fn get_secondary(&self) -> *mut OrderBook {
        self.secondary.load(Ordering::SeqCst)
    }

    // WARNING: always take fresh secondary reference after snapshot
    // in case the reference is stored in a variable outside
    pub fn snapshot(&self) {
        let primary = self.primary.load(Ordering::SeqCst);
        let old_secondary = self.secondary.load(Ordering::SeqCst);
        unsafe {
            let latest = Box::into_raw(Box::new((*primary).clone()));
            self.secondary.store(latest, Ordering::SeqCst);
            drop(Box::from_raw(old_secondary));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::core::models::{LimitOrder, Operation, Side};
    use crate::engine::services::manager::Manager;

    #[test]
    fn it_tests_successful_snapshot() {
        let manager = Manager::new();
        let operation = Operation::Limit(LimitOrder::new(1, 100, 100, Side::Bid));
        let primary = manager.get_primary();
        unsafe {
            (*primary).execute(operation);
        }
        unsafe {
            (*primary).execute(operation);
        }
        manager.snapshot();
        let secondary = manager.get_secondary();
        unsafe {
            println!("{:?}", (*secondary).depth(5));
        }
    }
}
