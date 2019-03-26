use crate::{DeviceIntRect, DeviceIntSize};
use euclid::{size2, vec2};

use std::num::Wrapping;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
struct AllocIndex(u32);
impl AllocIndex {
    const NONE: AllocIndex = AllocIndex(std::u32::MAX);

    fn index(self) -> usize { self.0 as usize }

    fn is_none(self) -> bool { self == AllocIndex::NONE }

    fn is_some(self) -> bool { self != AllocIndex::NONE }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct AllocId(u32);

const GEN_MASK: u32 = 0xFF000000;
const IDX_MASK: u32 = 0x00FFFFFF;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Orientation {
    Vertical,
    Horizontal,
}

impl Orientation {
    fn flipped(self) -> Self {
        match self {
            Orientation::Vertical => Orientation::Horizontal,
            Orientation::Horizontal => Orientation::Vertical,
        }
    }
}


#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum NodeKind {
    Container,
    Alloc,
    Free,
    Unused,
}

#[derive(Clone, Debug)]
pub struct Node {
    parent: AllocIndex,
    next_sibbling: AllocIndex,
    prev_sibbling: AllocIndex,
    kind: NodeKind,
    orientation: Orientation,
    rect: DeviceIntRect,
}

pub struct AtlasAllocator {
    nodes: Vec<Node>,
    free_list: Vec<AllocIndex>,
    unused_nodes: AllocIndex,
    generations: Vec<Wrapping<u8>>,
    snap_size: i32,
}

fn fit(slot: &DeviceIntRect, size: &DeviceIntSize) -> bool {
    slot.size.width >= size.width && slot.size.height >= size.height
}

impl AtlasAllocator {
    pub fn new(size: DeviceIntSize) -> Self {
        AtlasAllocator::with_snapping(size, 1)
    }

    pub fn with_snapping(size: DeviceIntSize, snap_size: i32) -> Self {
        AtlasAllocator {
            nodes: vec![Node {
                parent: AllocIndex::NONE,
                next_sibbling: AllocIndex::NONE,
                prev_sibbling: AllocIndex::NONE,
                rect: size.into(),
                kind: NodeKind::Free,
                orientation: Orientation::Vertical,
            }],
            free_list: vec![AllocIndex(0)],
            generations: vec![Wrapping(0)],
            unused_nodes: AllocIndex::NONE,
            snap_size,
        }
    }

    pub fn allocate(&mut self, mut requested_size: DeviceIntSize) -> Option<AllocId> {

        // Find a suitable free rect.
        let chosen_id = self.find_suitable_rect(&requested_size);

        if chosen_id.is_none() {
            //println!("failed to allocate {:?}", requested_size);
            //self.print_free_rects();

            // No suitable free rect!
            return None;
        }

        self.adjust_size(&mut requested_size.width);
        self.adjust_size(&mut requested_size.height);

        let chosen_node = self.nodes[chosen_id.index()].clone();
        let current_orientation = chosen_node.orientation;
        assert_eq!(chosen_node.kind, NodeKind::Free);

        // Decide whether to split horizontally or vertically.

        let candidate_free_rect_to_right = DeviceIntRect {
            origin: chosen_node.rect.origin + vec2(requested_size.width, 0),
            size: size2(
                chosen_node.rect.size.width - requested_size.width,
                requested_size.height,
            ),
        };
        let candidate_free_rect_to_bottom = DeviceIntRect {
            origin: chosen_node.rect.origin + vec2(0, requested_size.height),
            size: size2(
                requested_size.width,
                chosen_node.rect.size.height - requested_size.height,
            ),
        };

        let allocated_rect = DeviceIntRect {
            origin: chosen_node.rect.origin,
            size: requested_size,
        };

        let split_rect;
        let leftover_rect;
        let orientation;
        if requested_size == chosen_node.rect.size {
            // Perfect fit.
            orientation = current_orientation;
            split_rect = DeviceIntRect::zero();
            leftover_rect = DeviceIntRect::zero();
        } else if candidate_free_rect_to_right.size.area() > candidate_free_rect_to_bottom.size.area() {
            leftover_rect = candidate_free_rect_to_bottom;
            split_rect = DeviceIntRect {
                origin: candidate_free_rect_to_right.origin,
                size: size2(candidate_free_rect_to_right.size.width, chosen_node.rect.size.height),
            };
            orientation = Orientation::Horizontal;
        } else {
            leftover_rect = candidate_free_rect_to_right;
            split_rect = DeviceIntRect {
                origin: candidate_free_rect_to_bottom.origin,
                size: size2(chosen_node.rect.size.width, candidate_free_rect_to_bottom.size.height),
            };
            orientation = Orientation::Vertical;
        }

        // Update the tree

        let allocated_id;
        let split_id;
        let leftover_id;
        //println!("{:?} -> {:?}", current_orientation, orientation);
        if orientation == current_orientation {
            if split_rect.size.area() > 0 {
                let next_sibbling = chosen_node.next_sibbling;

                split_id = self.new_node();
                self.nodes[split_id.index()] = Node {
                    parent: chosen_node.parent,
                    next_sibbling,
                    prev_sibbling: chosen_id,
                    rect: split_rect,
                    kind: NodeKind::Free,
                    orientation: current_orientation,
                };

                self.nodes[chosen_id.index()].next_sibbling = split_id;
                if next_sibbling.is_some() {
                    self.nodes[next_sibbling.index()].prev_sibbling = split_id;
                }
            } else {
                split_id = AllocIndex::NONE;
            }

            if leftover_rect.size.area() > 0 {
                self.nodes[chosen_id.index()].kind = NodeKind::Container;

                allocated_id = self.new_node();
                leftover_id = self.new_node();

                self.nodes[allocated_id.index()] = Node {
                    parent: chosen_id,
                    next_sibbling: leftover_id,
                    prev_sibbling: AllocIndex::NONE,
                    rect: allocated_rect,
                    kind: NodeKind::Alloc,
                    orientation: current_orientation.flipped(),
                };

                self.nodes[leftover_id.index()] = Node {
                    parent: chosen_id,
                    next_sibbling: AllocIndex::NONE,
                    prev_sibbling: allocated_id,
                    rect: leftover_rect,
                    kind: NodeKind::Free,
                    orientation: current_orientation.flipped(),
                };
            } else {
                // No need to split for the leftover area, we can allocate directly in the chosen node.
                allocated_id = chosen_id;
                let node = &mut self.nodes[chosen_id.index()];
                node.kind = NodeKind::Alloc;
                node.rect = allocated_rect;

                leftover_id = AllocIndex::NONE
            }
        } else {
            self.nodes[chosen_id.index()].kind = NodeKind::Container;

            if split_rect.size.area() > 0 {
                split_id = self.new_node();
                self.nodes[split_id.index()] = Node {
                    parent: chosen_id,
                    next_sibbling: AllocIndex::NONE,
                    prev_sibbling: AllocIndex::NONE,
                    rect: split_rect,
                    kind: NodeKind::Free,
                    orientation: current_orientation.flipped(),
                };
            } else {
                split_id = AllocIndex::NONE;
            }

            if leftover_rect.size.area() > 0 {
                let container_id = self.new_node();
                self.nodes[container_id.index()] = Node {
                    parent: chosen_id,
                    next_sibbling: split_id,
                    prev_sibbling: AllocIndex::NONE,
                    rect: DeviceIntRect::zero(),
                    kind: NodeKind::Container,
                    orientation: current_orientation.flipped(),
                };

                self.nodes[split_id.index()].prev_sibbling = container_id;

                allocated_id = self.new_node();
                leftover_id = self.new_node();

                self.nodes[allocated_id.index()] = Node {
                    parent: container_id,
                    next_sibbling: leftover_id,
                    prev_sibbling: AllocIndex::NONE,
                    rect: allocated_rect,
                    kind: NodeKind::Alloc,
                    orientation: current_orientation,
                };

                self.nodes[leftover_id.index()] = Node {
                    parent: container_id,
                    next_sibbling: AllocIndex::NONE,
                    prev_sibbling: allocated_id,
                    rect: leftover_rect,
                    kind: NodeKind::Free,
                    orientation: current_orientation,
                };
            } else {
                allocated_id = self.new_node();
                self.nodes[allocated_id.index()] = Node {
                    parent: chosen_id,
                    next_sibbling: split_id,
                    prev_sibbling: AllocIndex::NONE,
                    rect: allocated_rect,
                    kind: NodeKind::Alloc,
                    orientation: current_orientation.flipped(),
                };

                self.nodes[split_id.index()].prev_sibbling = allocated_id;

                leftover_id = AllocIndex::NONE;
            }
        }

        if split_id.is_some() {
            self.add_free_rect(split_id, &split_rect.size);
        }

        if leftover_id.is_some() {
            self.add_free_rect(leftover_id, &leftover_rect.size);
        }

        //println!("allocated {:?}     split: {:?} leftover: {:?}", allocated_rect, split_rect, leftover_rect);
        //self.print_free_rects();

        #[cfg(feature = "checks")]
        self.check_tree();

        Some(self.alloc_id(allocated_id))
    }

    pub fn deallocate(&mut self, node_id: AllocId) {
        let mut node_id = self.get_index(node_id);

        assert!(node_id.index() < self.nodes.len());
        assert_eq!(self.nodes[node_id.index()].kind, NodeKind::Alloc);

        //println!("deallocate rect {} #{:?}", self.nodes[node_id.index()].rect, node_id);
        self.nodes[node_id.index()].kind = NodeKind::Free;

        loop {
            let orientation = self.nodes[node_id.index()].orientation;

            let next = self.nodes[node_id.index()].next_sibbling;
            let prev = self.nodes[node_id.index()].prev_sibbling;

            // Try to merge with the next node.
            if next.is_some() && self.nodes[next.index()].kind == NodeKind::Free {
                self.merge_sibblings(node_id, next, orientation);
            }

            // Try to merge with the previous node.
            if prev.is_some() && self.nodes[prev.index()].kind == NodeKind::Free {
                self.merge_sibblings(prev, node_id, orientation);
                node_id = prev;
            }

            // If this node is now a unique child. We collapse it into its parent and try to merge
            // again at the parent level.
            let parent = self.nodes[node_id.index()].parent;
            if self.nodes[node_id.index()].prev_sibbling.is_none()
                && self.nodes[node_id.index()].next_sibbling.is_none()
                && parent.is_some() {
                //println!("collapse #{:?} into parent #{:?}", node_id, parent);

                self.mark_node_unused(node_id);

                // Replace the parent container with a free node.
                self.nodes[parent.index()].rect = self.nodes[node_id.index()].rect;
                self.nodes[parent.index()].kind = NodeKind::Free;

                // Start again at the parent level.
                node_id = parent;
            } else {

                let size = self.nodes[node_id.index()].rect.size;
                self.add_free_rect(node_id, &size);
                break;
            }
        }

        #[cfg(feature = "checks")]
        self.check_tree();
    }

    fn find_suitable_rect(&mut self, requested_size: &DeviceIntSize) -> AllocIndex {

        let mut i = 0;
        while i < self.free_list.len() {
            let id = self.free_list[i];

            // During tree simplification we don't remove merged nodes from the free list, so we have
            // to handle it here.
            // This is a tad awkward, but lets us avoid having to maintain a doubly linked list for
            // the free list (which would be needed to remove nodes during tree simplification).
            let should_remove = self.nodes[id.index()].kind != NodeKind::Free;
            let suitable = !should_remove && fit(&self.nodes[id.index()].rect, requested_size);

            if should_remove || suitable {
                // remove the element from the free list
                self.free_list.swap_remove(i);
            } else {
                i += 1;
            }

            if suitable {
                return id;
            }
        }

        AllocIndex::NONE
    }

    fn new_node(&mut self) -> AllocIndex {
        let idx = self.unused_nodes;
        if idx.index() < self.nodes.len() {
            self.unused_nodes = self.nodes[idx.index()].next_sibbling;
            self.generations[idx.index()] += Wrapping(1);
            return idx;
        }

        self.nodes.push(Node {
            parent: AllocIndex::NONE,
            next_sibbling: AllocIndex::NONE,
            prev_sibbling: AllocIndex::NONE,
            rect: DeviceIntRect::zero(),
            kind: NodeKind::Unused,
            orientation: Orientation::Horizontal,
        });

        self.generations.push(Wrapping(0));

        AllocIndex(self.nodes.len() as u32 - 1)
    }

    fn mark_node_unused(&mut self, id: AllocIndex) {
        debug_assert!(self.nodes[id.index()].kind != NodeKind::Unused);
        self.nodes[id.index()].kind = NodeKind::Unused;
        self.nodes[id.index()].next_sibbling = self.unused_nodes;
        self.unused_nodes = id;
    }

    fn adjust_size(&self, size: &mut i32) {
        let rem = *size % self.snap_size;
        if rem > 0 {
            *size += self.snap_size - rem;
        }
    }

    #[allow(dead_code)]
    fn print_free_rects(&self) {
        for &id in &self.free_list {
            if self.nodes[id.index()].kind == NodeKind::Free {
                println!(" - {} #{:?}", self.nodes[id.index()].rect, id);
            }
        }
    }

    #[cfg(feature = "checks")]
    fn check_sibblings(&self, id: AllocIndex, next: AllocIndex, orientation: Orientation) {
        if next.is_none() {
            return;
        }

        if self.nodes[next.index()].prev_sibbling != id {
            //println!("error: #{}'s next sibbling #{} has prev sibbling #{}", id, next, self.nodes[next.index()].prev_sibbling);
        }
        assert_eq!(self.nodes[next.index()].prev_sibbling, id);

        match self.nodes[id.index()].kind {
            NodeKind::Container | NodeKind::Unused => {
                return;
            }
            _ => {}
        }
        match self.nodes[next.index()].kind {
            NodeKind::Container | NodeKind::Unused => {
                return;
            }
            _ => {}
        }

        let r1 = self.nodes[id.index()].rect;
        let r2 = self.nodes[next.index()].rect;
        match orientation {
            Orientation::Horizontal => {
                assert_eq!(r1.min_y(), r2.min_y());
                assert_eq!(r1.max_y(), r2.max_y());
            }
            Orientation::Vertical => {
                assert_eq!(r1.min_x(), r2.min_x());
                assert_eq!(r1.max_x(), r2.max_x());
            }
        }
    }

    #[cfg(feature = "checks")]
    fn check_tree(&self) {
        for node_idx in 0..self.nodes.len() {
            let node = &self.nodes[node_idx];

            if node.kind == NodeKind::Unused {
                continue;
            }

            let mut iter = node.next_sibbling;
            while iter.is_some() {
                assert_eq!(self.nodes[iter.index()].orientation, node.orientation);
                assert_eq!(self.nodes[iter.index()].parent, node.parent);
                let next = self.nodes[iter.index()].next_sibbling;

                #[cfg(feature = "checks")]
                self.check_sibblings(iter, next, node.orientation);

                iter = next;

            }

            if node.parent.is_some() {
                if self.nodes[node.parent.index()].kind != NodeKind::Container {
                    //println!("error: child: {:?} parent: {:?}", node_idx, node.parent);
                }
                assert_eq!(self.nodes[node.parent.index()].orientation, node.orientation.flipped());
                assert_eq!(self.nodes[node.parent.index()].kind, NodeKind::Container);
            }
        }
    }

    fn add_free_rect(&mut self, id: AllocIndex, _size: &DeviceIntSize) {
        //println!("add free rect #{}", id);
        // TODO: Separate small/medium /large free rect lists.
        debug_assert_eq!(self.nodes[id.index()].kind, NodeKind::Free);
        self.free_list.push(id);
    }

    // Merge `next` into `node` and append `next` to a list of available `nodes`vector slots.
    fn merge_sibblings(&mut self, node: AllocIndex, next: AllocIndex, orientation: Orientation) {
        let r1 = self.nodes[node.index()].rect;
        let r2 = self.nodes[next.index()].rect;
        //println!("merge {} #{} and {} #{}       {:?}", r1, node, r2, next, orientation);
        let merge_size = self.nodes[next.index()].rect.size;
        match orientation {
            Orientation::Horizontal => {
                assert_eq!(r1.min_y(), r2.min_y());
                assert_eq!(r1.max_y(), r2.max_y());
                self.nodes[node.index()].rect.size.width += merge_size.width;
            }
            Orientation::Vertical => {
                assert_eq!(r1.min_x(), r2.min_x());
                assert_eq!(r1.max_x(), r2.max_x());
                self.nodes[node.index()].rect.size.height += merge_size.height;
            }
        }

        // Remove the merged node from the sibbling list.
        let next_next = self.nodes[next.index()].next_sibbling;
        self.nodes[node.index()].next_sibbling = next_next;
        if next_next.is_some() {
            self.nodes[next_next.index()].prev_sibbling = node;
        }

        // Add the merged node to the list of available slots in the nodes vector.
        self.mark_node_unused(next);
    }

    fn alloc_id(&self, index: AllocIndex) -> AllocId {
        let generation = self.generations[index.index()].0 as u32;
        debug_assert!(index.0 & IDX_MASK == index.0);
        AllocId(index.0 + (generation << 24))
    }

    fn get_index(&self, id: AllocId) -> AllocIndex {
        let idx = id.0 & IDX_MASK;
        let expected_generation = (self.generations[idx as usize].0 as u32) << 24;
        assert_eq!(id.0 & GEN_MASK, expected_generation);
        AllocIndex(idx)
    }
}


#[test]
fn atlas_simple() {
    let mut atlas = AtlasAllocator::new(size2(1000, 1000));

    let full = atlas.allocate(size2(1000,1000)).unwrap();
    assert!(atlas.allocate(size2(1, 1)).is_none());

    atlas.deallocate(full);

    let a = atlas.allocate(size2(100, 1000)).unwrap();
    let b = atlas.allocate(size2(900, 200)).unwrap();
    let c = atlas.allocate(size2(300, 200)).unwrap();
    let d = atlas.allocate(size2(200, 300)).unwrap();
    let e = atlas.allocate(size2(100, 300)).unwrap();
    let f = atlas.allocate(size2(100, 300)).unwrap();
    let g = atlas.allocate(size2(100, 300)).unwrap();

    atlas.deallocate(b);
    atlas.deallocate(f);
    atlas.deallocate(c);
    atlas.deallocate(e);
    let h = atlas.allocate(size2(500, 200)).unwrap();
    atlas.deallocate(a);
    let i = atlas.allocate(size2(500, 200)).unwrap();
    atlas.deallocate(g);
    atlas.deallocate(h);
    atlas.deallocate(d);
    atlas.deallocate(i);

    let full = atlas.allocate(size2(1000,1000)).unwrap();
    assert!(atlas.allocate(size2(1, 1)).is_none());
    atlas.deallocate(full);
}

#[test]
fn atlas_random_test() {
    let mut atlas = AtlasAllocator::with_snapping(size2(1000, 1000), 5);

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
    for _ in 0..1000000 {
        if rand() % 5 > 2 && !allocated.is_empty() {
            // deallocate something
            let nth = rand() % allocated.len();
            let id = allocated[nth];
            allocated.remove(nth);

            atlas.deallocate(id);
        } else {
            // allocate something
            let size = size2(
                (rand() % 300) as i32 + 5,
                (rand() % 300) as i32 + 5,
            );

            if let Some(id) = atlas.allocate(size) {
                allocated.push(id);
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
    println!("nodes.cap: {}, free_list.cap: {}", atlas.nodes.capacity(), atlas.free_list.capacity());

    let full = atlas.allocate(size2(1000,1000)).unwrap();
    assert!(atlas.allocate(size2(1, 1)).is_none());
    atlas.deallocate(full);
}


