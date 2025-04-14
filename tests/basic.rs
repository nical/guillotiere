#[test]
fn atlas_basic() {
    let mut atlas = AtlasAllocator::new(size2(1000, 1000));

    let full = atlas.allocate(size2(1000, 1000)).unwrap().id;
    assert!(atlas.allocate(size2(1, 1)).is_none());

    atlas.deallocate(full);

    let a = atlas.allocate(size2(100, 1000)).unwrap().id;
    let b = atlas.allocate(size2(900, 200)).unwrap().id;
    let c = atlas.allocate(size2(300, 200)).unwrap().id;
    let d = atlas.allocate(size2(200, 300)).unwrap().id;
    let e = atlas.allocate(size2(100, 300)).unwrap().id;
    let f = atlas.allocate(size2(100, 300)).unwrap().id;
    let g = atlas.allocate(size2(100, 300)).unwrap().id;

    atlas.deallocate(b);
    atlas.deallocate(f);
    atlas.deallocate(c);
    atlas.deallocate(e);
    let h = atlas.allocate(size2(500, 200)).unwrap().id;
    atlas.deallocate(a);
    let i = atlas.allocate(size2(500, 200)).unwrap().id;
    atlas.deallocate(g);
    atlas.deallocate(h);
    atlas.deallocate(d);
    atlas.deallocate(i);

    let full = atlas.allocate(size2(1000, 1000)).unwrap().id;
    assert!(atlas.allocate(size2(1, 1)).is_none());
    atlas.deallocate(full);
}
