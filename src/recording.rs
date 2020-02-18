use crate::*;
use std::collections::HashMap;

pub struct RecordingAllocator {
    allocator: AtlasAllocator,
    recorder: Recorder,
    // Assign unique ids to recorded events. This simplifies a few things, later on.
    id_map: HashMap<AllocId, AllocId>,
    next_id: u32,
}

impl RecordingAllocator {
    /// Create an atlas allocator.
    pub fn new(size: Size) -> Self {
        RecordingAllocator {
            allocator: AtlasAllocator::new(size),
            recorder: Recorder {
                events: Vec::new(),
                initial_size: size,
                options: DEFAULT_OPTIONS,
            },
            id_map: HashMap::new(),
            next_id: 0,
        }
    }

    /// Create an atlas allocator with the provided options.
    pub fn with_options(size: Size, options: &AllocatorOptions) -> Self {
        RecordingAllocator {
            allocator: AtlasAllocator::with_options(size, options),
            recorder: Recorder {
                events: Vec::new(),
                initial_size: size,
                options: *options,
            },
            id_map: HashMap::new(),
            next_id: 0,
        }
    }

    /// The total size of the atlas.
    pub fn size(&self) -> Size {
        self.allocator.size()
    }

    /// Allocate a rectangle in the atlas.
    pub fn allocate(&mut self, requested_size: Size) -> Option<Allocation> {
        let res = self.allocator.allocate(requested_size).map(|res| {
            let id = AllocId(self.next_id);
            self.next_id += 1;

            self.id_map.insert(id, res.id);

            println!(" alloc {:?} (was {:?})", id, res.id);

            Allocation { id, ..res }
        });

        self.recorder.record(Event::Allocate(requested_size, res.map(|r| r.id)));

        res
    }

    /// Deallocate a rectangle in the atlas.
    pub fn deallocate(&mut self, node_id: AllocId) {
        if let Some(actual_id) = self.id_map.get(&node_id) {
            println!(" dealloc {:?} (was {:?})", node_id, actual_id);
            self.allocator.deallocate(*actual_id);
            self.recorder.record(Event::Deallocate(node_id));
        }
    }

    /// Recompute the allocations in the atlas and returns a list of the changes.
    ///
    /// Previous ids and rectangles are not valid anymore after this operation as each id/rectangle
    /// pair is assigned to new values which are communicated in the returned change list.
    /// Rearranging the atlas can help reduce fragmentation.
    pub fn rearrange(&mut self) -> ChangeList {
        let changes = self.allocator.rearrange();
        let remapped = self.remap_changelist(&changes);

        self.recorder.record(Event::Rearrange(remapped.clone()));

        remapped
    }

    /// Identical to `AtlasAllocator::rearrange`, also allowing to change the size of the atlas.
    pub fn resize_and_rearrange(&mut self, new_size: Size) -> ChangeList {
        let changes = self.allocator.resize_and_rearrange(new_size);
        let remapped = self.remap_changelist(&changes);

        self.recorder.record(Event::ResizeAndRearrange(new_size, remapped.clone()));

        remapped
    }

    fn remap_changelist(&mut self, changes: &ChangeList) -> ChangeList {
        let mut remapped = ChangeList {
            changes: Vec::new(),
            failures: Vec::new(),
        };

        let prev_id_map = std::mem::replace(&mut self.id_map, HashMap::new());
        self.id_map.clear();

        for change in &changes.changes {
            let mut id = None;
            for (k, v) in &prev_id_map {
                if *v == change.old.id {
                    id = Some(*k);
                    break;
                }
            }


            let id = id.unwrap();

            self.id_map.insert(id, change.new.id);
            remapped.changes.push(Change {
                old: Allocation { id, ..change.old },
                new: Allocation { id, ..change.new },
            });
        }

        for failure in &changes.failures {
            let mut id = None;
            for (k, v) in &prev_id_map {
                if *v == failure.id {
                    id = Some(*k);
                    break;
                }
            }
            remapped.failures.push(Allocation {
                id : id.unwrap(),
                rectangle: failure.rectangle,
            });
        }

        remapped
    }

    pub fn grow(&mut self, new_size: Size) {
        self.recorder.record(Event::Grow(new_size));
        self.allocator.grow(new_size);
    }

    pub fn for_each_free_rectangle<F>(&self, callback: F)
    where
        F: FnMut(&Rectangle),
    {
        self.allocator.for_each_free_rectangle(callback);
    }

    pub fn for_each_allocated_rectangle<F>(&self, callback: F)
    where
        F: FnMut(AllocId, &Rectangle),
    {
        self.allocator.for_each_allocated_rectangle(callback);
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    Allocate(Size, Option<AllocId>),
    Deallocate(AllocId),
    Grow(Size),
    Rearrange(ChangeList),
    ResizeAndRearrange(Size, ChangeList),
}

pub struct Recorder {
    events: Vec<Event>,
    initial_size: Size,
    options: AllocatorOptions,
}

impl Recorder {
    pub fn record(&mut self, event: Event) {
        self.events.push(event);
    }

    pub fn finish(&mut self) -> Recording {
        Recording {
            events: std::mem::replace(&mut self.events, Vec::new()),
            options: self.options,
            initial_size: self.initial_size,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayStats {
    allocations: u32,
    deallocations: u32,
    failed_allocations: u32,
}

#[derive(Clone)]
pub struct Recording {
    initial_size: Size,
    events: Vec<Event>,
    options: AllocatorOptions,
}

impl Recording {
    pub fn replay(&self) -> std::thread::Result<ReplayStats> {
        //println!("--------------- replay");
        std::panic::catch_unwind(|| {
            let mut stats = ReplayStats {
                allocations: 0,
                deallocations: 0,
                failed_allocations: 0,
            };

            let mut allocator = AtlasAllocator::with_options(self.initial_size, &self.options);
            let mut id_remap: HashMap<AllocId, Option<AllocId>> = HashMap::default();
            for evt in &self.events {
                match *evt {
                    Event::Allocate(size, recorded_id) => {
                        let alloc = allocator.allocate(size);

                        match alloc {
                            Some(_) => {
                                stats.allocations += 1;
                            }
                            None => {
                                stats.failed_allocations += 1;
                            }
                        }

                        if let Some(recorded_id) = recorded_id {
                            id_remap.insert(recorded_id, alloc.map(|alloc| alloc.id));
                        }

                        //println!("+ alloc {:?} ({:?})", recorded_id, alloc.map(|alloc| alloc.id));
                    }
                    Event::Deallocate(recorded_id) => {
                        if let Some(Some(id)) = id_remap.remove(&recorded_id) {
                            //println!("- dealloc {:?} ({:?})", recorded_id, id);
                            allocator.deallocate(id);
                            stats.deallocations += 1;
                        }
                    }
                    Event::Grow(size) => {
                        allocator.grow(size);
                    }
                    Event::Rearrange(ref recorded_changes) => {
                        //println!(" *** rearrange");
                        let changes = allocator.rearrange();
                        Recording::apply_changelists(&mut id_remap, &recorded_changes, &changes);
                    }
                    Event::ResizeAndRearrange(new_size, ref recorded_changes) => {
                        let changes = allocator.resize_and_rearrange(new_size);
                        Recording::apply_changelists(&mut id_remap, recorded_changes, &changes);
                    }
                }
            }

            stats
        })
    }

    fn apply_changelists(
        id_remap: &mut HashMap<AllocId, Option<AllocId>>,
        recorded: &ChangeList,
        new: &ChangeList,
    ) {
        for change in &recorded.changes {
            if let Some(id) = id_remap.remove(&change.old.id) {
                //println!("   * {:?} -> {:?}", change.old.id, change.new.id);

                id_remap.insert(change.new.id, id);
            }
        }

        for remapped in id_remap.values_mut() {
            for change in &new.changes {
                if *remapped == Some(change.old.id) {
                    //println!("   ** {:?} -> {:?}", remapped.unwrap(), change.new.id);
                    *remapped = Some(change.new.id);
                    break;
                }
            }
            for failure in &new.failures {
                if *remapped == Some(failure.id) {
                    //println!("   ** failed {:?}", remapped.unwrap());
                    *remapped = None;
                    break;
                }
            }
        }
    }

    pub fn remove_event(&mut self, index: usize) {
        //println!("remove {:?}", self.events[index]);
        self.events.remove(index);
    }

    pub fn find_reduced_testcase(&self) -> Recording {
        let mut recording = self.clone();
        let mut i = 0;

        loop {
            if i >= recording.events.len() {
                recording.remap_ids();
                return recording;
            }

            let mut reduced = recording.clone();
            reduced.events.remove(i);

            if !reduced.replay().is_ok() {
                recording = reduced;
            } else {
                i += 1;
            }
        }
    }

    pub fn write_testcase(&self, output: &mut dyn std::io::Write) -> std::io::Result<()> {
        writeln!(output, "#[test]")?;
        writeln!(output, "fn reduced_testcase() {{")?;
        writeln!(output, "    let options = AllocatorOptions {{")?;
        writeln!(output, "         snap_size: {},", self.options.snap_size)?;
        writeln!(
            output,
            "         small_size_threshold: {},",
            self.options.small_size_threshold
        )?;
        writeln!(
            output,
            "         large_size_threshold: {},",
            self.options.large_size_threshold
        )?;
        writeln!(output, "    }};")?;
        writeln!(
            output,
            "    let size = size2({}, {});",
            self.initial_size.width, self.initial_size.height
        )?;
        writeln!(
            output,
            "    let mut allocator = AtlasAllocator::with_options(size, options);"
        )?;
        let mut next_identifier = self.events.len() as u32;
        for event in &self.events {
            match *event {
                Event::Allocate(size, res) => {
                    let identifier = res.map(|id| id.to_u32()).unwrap_or_else(|| {
                        next_identifier += 1;
                        next_identifier
                    });
                    writeln!(
                        output,
                        "    let r{} = allocator.allocate(size2({}, {}));",
                        identifier, size.width, size.height,
                    )?;
                }
                Event::Deallocate(id) => {
                    writeln!(
                        output,
                        "    allocator.deallocate(r{}.unwrap().id);",
                        id.to_u32()
                    )?;
                }
                Event::Grow(size) => {
                    writeln!(
                        output,
                        "    allocator.grow(size2({}, {}));",
                        size.width, size.height
                    )?;
                }
                Event::Rearrange(_) => {
                    writeln!(output, "    allocator.rearrange();")?;
                }
                Event::ResizeAndRearrange(size, _) => {
                    writeln!(
                        output,
                        "    allocator.resize_and_rearrange(size2({}, {}));",
                        size.width, size.height
                    )?;
                }
            }
        }
        writeln!(output, "}}")?;

        Ok(())
    }

    fn remap_ids(&mut self) {
        let mut allocator = AtlasAllocator::with_options(self.initial_size, &self.options);
        let mut id_remap: HashMap<AllocId, Option<AllocId>> = HashMap::default();
        let mut idx = 0;
        while idx < self.events.len() {
            match self.events[idx] {
                Event::Allocate(size, ref mut recorded_id) => {
                    let key = *recorded_id;
                    let alloc = allocator.allocate(size);

                    let actual_id = alloc.map(|alloc| alloc.id);
                    *recorded_id = actual_id;

                    if let Some(key) = key {
                        id_remap.insert(key, actual_id);
                    }
                }
                Event::Deallocate(ref mut recorded_id) => match id_remap.remove(recorded_id) {
                    Some(Some(id)) => {
                        allocator.deallocate(id);
                        *recorded_id = id;
                    }
                    _ => {
                        self.events.remove(idx);
                        continue;
                    }
                },
                Event::Grow(size) => {
                    allocator.grow(size);
                }
                Event::Rearrange(ref mut recorded_changes) => {
                    let changes = allocator.rearrange();
                    Recording::apply_changelists(&mut id_remap, recorded_changes, &changes);

                    *recorded_changes = changes;
                }
                Event::ResizeAndRearrange(new_size, ref mut recorded_changes) => {
                    let changes = allocator.resize_and_rearrange(new_size);
                    Recording::apply_changelists(&mut id_remap, recorded_changes, &changes);

                    *recorded_changes = changes;
                }
            }

            idx += 1
        }
    }
}

#[test]
fn recording_random_test() {
    let mut atlas = RecordingAllocator::with_options(
        size2(1000, 1000),
        &AllocatorOptions {
            snap_size: 5,
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

    let mut allocated = Vec::new();
    for i in 0..50000 {
        if i % 10000 == 10 {
            let cl = atlas.rearrange();
            assert!(cl.failures.is_empty());
            for changes in &cl.changes {
                for id in &mut allocated {
                    if *id == changes.old.id {
                        *id = changes.new.id;
                        break;
                    }
                }
            }
        }

        if rand() % 5 > 2 && !allocated.is_empty() {
            let nth = rand() % allocated.len();
            let id = allocated[nth];
            allocated.remove(nth);

            atlas.deallocate(id);
        } else {
            let size = size2((rand() % 300) as i32 + 5, (rand() % 300) as i32 + 5);

            if let Some(alloc) = atlas.allocate(size) {
                allocated.push(alloc.id);
            }
        }
    }

    while let Some(id) = allocated.pop() {
        atlas.deallocate(id);
    }

    let mut recording = atlas.recorder.finish();

    recording.replay().unwrap();

    recording.remap_ids();

    recording.replay().unwrap();

    for i in 0..100 {
        recording.remove_event(i * 27);
    }

    recording.replay().unwrap();

    recording.remap_ids();

    recording.replay().unwrap();
}
