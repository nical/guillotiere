#[test]
fn atlas_random_test() {
    let mut atlas = AtlasAllocator::with_options(
        size2(1000, 1000),
        &AllocatorOptions {
            alignment: size2(5, 2),
            ..DEFAULT_OPTIONS
        },
    );

    let a = 1103515245;
    let c = 12345;
    let m = usize::pow(2, 31);
    let mut seed: usize = 37;

    let mut rand = || {
        seed = (a * seed + c) % m;
        seed
    };

    let mut n: usize = 0;
    let mut misses: usize = 0;

    let mut allocated = Vec::new();
    for _ in 0..500000 {
        if rand() % 5 > 2 && !allocated.is_empty() {
            // deallocate something
            let nth = rand() % allocated.len();
            let id = allocated[nth];
            allocated.remove(nth);

            atlas.deallocate(id);
        } else {
            // allocate something
            let size = size2((rand() % 300) as i32 + 5, (rand() % 300) as i32 + 5);

            if let Some(alloc) = atlas.allocate(size) {
                allocated.push(alloc.id);
                n += 1;
            } else {
                misses += 1;
            }
        }
    }

    while let Some(id) = allocated.pop() {
        atlas.deallocate(id);
    }

    println!("added/removed {} rectangles, {} misses", n, misses);
    println!(
        "nodes.cap: {}, free_list.cap: {}/{}/{}",
        atlas.nodes.capacity(),
        atlas.free_lists[LARGE_BUCKET].capacity(),
        atlas.free_lists[MEDIUM_BUCKET].capacity(),
        atlas.free_lists[SMALL_BUCKET].capacity(),
    );

    let full = atlas.allocate(size2(1000, 1000)).unwrap().id;
    assert!(atlas.allocate(size2(1, 1)).is_none());
    atlas.deallocate(full);
}
