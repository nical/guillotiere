#[test]
fn simple_atlas() {
    let mut atlas = SimpleAtlasAllocator::new(size2(1000, 1000));

    assert!(atlas.allocate(size2(1, 1001)).is_none());
    assert!(atlas.allocate(size2(1001, 1)).is_none());

    let mut rectangles = Vec::new();
    rectangles.push(atlas.allocate(size2(100, 1000)).unwrap());
    rectangles.push(atlas.allocate(size2(900, 200)).unwrap());
    rectangles.push(atlas.allocate(size2(300, 200)).unwrap());
    rectangles.push(atlas.allocate(size2(200, 300)).unwrap());
    rectangles.push(atlas.allocate(size2(100, 300)).unwrap());
    rectangles.push(atlas.allocate(size2(100, 300)).unwrap());
    rectangles.push(atlas.allocate(size2(100, 300)).unwrap());
    assert!(atlas.allocate(size2(800, 800)).is_none());

    for i in 0..rectangles.len() {
        for j in 0..rectangles.len() {
            if i == j {
                continue;
            }

            assert!(!rectangles[i].intersects(&rectangles[j]));
        }
    }
}
