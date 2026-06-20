use std::cell::Cell;

use oxc_allocator::Allocator;

thread_local! {
    static REUSE: Cell<Option<Allocator>> = Cell::new(Some(Allocator::new()));
}

pub(crate) fn with_reusable_allocator<R>(f: impl FnOnce(&Allocator) -> R) -> R {
    REUSE.with(|cell| {
        let mut allocator = cell.replace(None).unwrap_or_else(Allocator::new);
        // If `f` panics, the allocator is dropped during unwinding instead of being reused.
        let result = f(&allocator);
        allocator.reset();
        cell.set(Some(allocator));
        result
    })
}
