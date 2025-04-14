#[test]
fn allocate_zero() {
    let mut atlas = SimpleAtlasAllocator::new(size2(1000, 1000));

    assert!(atlas.allocate(size2(0, 0)).is_none());
}

#[test]
fn allocate_negative() {
    let mut atlas = SimpleAtlasAllocator::new(size2(1000, 1000));

    assert!(atlas.allocate(size2(-1, 1)).is_none());
    assert!(atlas.allocate(size2(1, -1)).is_none());
    assert!(atlas.allocate(size2(-1, -1)).is_none());

    assert!(atlas.allocate(size2(-167114179, -718142)).is_none());
}

#[test]
fn issue_25() {
    let mut allocator = AtlasAllocator::new(Size::new(65536, 65536));
    allocator.allocate(Size::new(2,2));
    allocator.allocate(Size::new(65500,2));
    allocator.allocate(Size::new(2, 65500));
}
