use std::cell::Cell;
use std::ops::Deref;

use oxc_allocator::Allocator;

thread_local! {
    static REUSE: Cell<Option<Allocator>> = Cell::new(Some(Allocator::new()));
}

struct ReusableAllocatorGuard<'a> {
    allocator: Allocator,
    cell: &'a Cell<Option<Allocator>>,
}

impl Drop for ReusableAllocatorGuard<'_> {
    fn drop(&mut self) {
        let mut allocator = std::mem::take(&mut self.allocator);
        allocator.reset();
        self.cell.set(Some(allocator));
    }
}

impl Deref for ReusableAllocatorGuard<'_> {
    type Target = Allocator;

    fn deref(&self) -> &Self::Target {
        &self.allocator
    }
}

pub(crate) fn with_reusable_allocator<R>(f: impl FnOnce(&Allocator) -> R) -> R {
    REUSE.with(|cell| {
        let guard = ReusableAllocatorGuard {
            allocator: cell.replace(None).unwrap_or_else(Allocator::new),
            cell,
        };
        let result = f(&guard);
        drop(guard);
        result
    })
}
