#[test]
fn clear_empty() {
    let mut atlas = AtlasAllocator::new(size2(1000, 1000));

    assert!(atlas.is_empty());

    assert!(atlas.allocate(size2(10, 10)).is_some());
    assert!(!atlas.is_empty());

    atlas.clear();
    assert!(atlas.is_empty());

    let a = atlas.allocate(size2(10, 10)).unwrap().id;
    let b = atlas.allocate(size2(20, 20)).unwrap().id;
    assert!(!atlas.is_empty());

    atlas.deallocate(b);
    atlas.deallocate(a);
    assert!(atlas.is_empty());

    atlas.clear();
    assert!(atlas.is_empty());

    atlas.clear();
    assert!(atlas.is_empty());
}
