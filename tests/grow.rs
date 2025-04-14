#[test]
fn test_grow() {
    let mut atlas = AtlasAllocator::new(size2(1000, 1000));

    atlas.grow(size2(2000, 2000));

    let full = atlas.allocate(size2(2000, 2000)).unwrap().id;
    assert!(atlas.allocate(size2(1, 1)).is_none());
    atlas.deallocate(full);

    let a = atlas.allocate(size2(100, 100)).unwrap().id;

    atlas.grow(size2(3000, 3000));

    let b = atlas.allocate(size2(1000, 2900)).unwrap().id;

    atlas.grow(size2(4000, 4000));

    atlas.deallocate(b);
    atlas.deallocate(a);

    let full = atlas.allocate(size2(4000, 4000)).unwrap().id;
    assert!(atlas.allocate(size2(1, 1)).is_none());
    atlas.deallocate(full);
}
