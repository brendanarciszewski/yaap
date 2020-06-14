# Changelog

## Latest

## v0.0.2
-   added changelog
-   introduced an aliased `RefCell` into the allocator API so that unique
    references can be obtained as late as possible (enabling allocator sharing
    if the allocator is stored as `Rc` or `Arc`)
-   reordered modules (added prelude for Traits)

## v0.0.1
-   initial version based roughly on C++17's `std::pmr`
-   in early sketches AllocatorAwareContainer would enable allocators to be
    passed into interior containers
