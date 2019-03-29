use crate::{DeviceIntRect, DeviceIntPoint, DeviceIntSize};
use euclid::{size2, vec2};

use std::num::Wrapping;

const LARGE_BUCKET: usize = 2;
const MEDIUM_BUCKET: usize = 1;
const SMALL_BUCKET: usize = 0;
const NUM_BUCKETS: usize = 3;

fn free_list_for_size(small_threshold: i32, large_threshold: i32, size: &DeviceIntSize) -> usize {
    if size.width >= large_threshold || size.height >= large_threshold {
        LARGE_BUCKET
    } else if size.width >= small_threshold || size.height >= small_threshold {
        MEDIUM_BUCKET
    } else {
        SMALL_BUCKET
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
struct AllocIndex(u32);
impl AllocIndex {
    const NONE: AllocIndex = AllocIndex(std::u32::MAX);

    fn index(self) -> usize { self.0 as usize }

    fn is_none(self) -> bool { self == AllocIndex::NONE }

    fn is_some(self) -> bool { self != AllocIndex::NONE }
}

/// ID referring to an allocated rectangle.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct AllocId(u32);

const GEN_MASK: u32 = 0xFF000000;
const IDX_MASK: u32 = 0x00FFFFFF;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum Orientation {
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


#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum NodeKind {
    Container,
    Alloc,
    Free,
    Unused,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
struct Node {
    parent: AllocIndex,
    next_sibbling: AllocIndex,
    prev_sibbling: AllocIndex,
    kind: NodeKind,
    orientation: Orientation,
    rect: DeviceIntRect,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
/// Options to tweak the behavior of the atlas allocator.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct AllocatorOptions {
    /// Round the rectangle sizes up to a multiple of this value.
    ///
    /// This value must be superior to zero.
    ///
    /// Default value: 1,
    pub snap_size: i32,

    /// Value below which a size is considered small.
    ///
    /// This is value is used to speed up the storage and lookup of free rectangles.
    /// This value must be inferior or equal to `large_size_threshold`
    ///
    /// Default value: 32,
    pub small_size_threshold: i32,

    /// Value above which a size is considered large.
    ///
    /// This is value is used to speed up the storage and lookup of free rectangles.
    /// This value must be inferior or equal to `large_size_threshold`
    ///
    /// Default value: 256,
    pub large_size_threshold: i32,
}

pub const DEFAULT_OPTIONS: AllocatorOptions = AllocatorOptions {
    snap_size: 1,
    large_size_threshold: 256,
    small_size_threshold: 32,
};

impl Default for AllocatorOptions {
    fn default() -> Self { DEFAULT_OPTIONS }
}

/// A dynamic texture atlas allocator using the guillotine algorithm.
///
/// The guillotine algorithm is assisted by a data structure that keeps track of
/// nighboring rectangles to provide fast deallocation and coalescing.
///
/// ## Goals
///
/// Coalescing free rectangles, in the context of dynamic atlas allocation can be
/// prohibitively expensive under real-time constraints if the algorithm needs to
/// visit a large amount of free rectangles to find merge candidates.
///
/// This implementation proposes a compromise with fast (constant time) search
/// for merge candidates at the expense of some (constant time) bookeeping overhead
/// when allocating and removing rectangles and imperfect defragmentation (see the
/// "Limitations" section below.
///
/// The subdivision scheme uses the worst fit varriant of the guillotine algorithm
/// for its simplicity and CPU efficiency.
///
/// ## The data structure
///
/// We maintain a tree with allocated and free rectangles as leaf nodes and
/// containers as non-leaf nodes.
///
/// The direct children of a Containers's form an ordered horizontal or vertical
/// sequence of rectangles that cover exactly their parent container's area.
///
/// For example, a subdivision such as this one:
///
/// ```ascii
/// +-----------+----------+---+---+--+---------+---+
/// |           |          | C | D |E | F       | G |
/// |           |          +---+---+--+---------+---+
/// |     A     |    B     |                        |
/// |           |          |           H            |
/// |           |          |                        |
/// +------+----+----------+-+----------------------+
/// |      |        J        |                      |
/// |  I   +-----------------+          L           |
/// |      |        K        |                      |
/// +------+-----------------+----------------------+
/// ```
///
/// Would have a tree of the form:
///
/// ```ascii
///
///  Tree                | Layout
/// ---------------------+------------
///                      |
///           #          |
///           |          |
///      +----+----+. . .|. vertical
///      |         |     |
///      #         #     |
///      |         |     |
///    +-+-+ . . +-+-+. .|. horizontal
///    | | |     | | |   |
///    A B #     I # L   |
///        |       |     |
///      +-+-+ . +-+-+. .|. vertical
///      |   |   |   |   |
///      #   h   J   K   |
///      |               |
///  +-+-+-+-+. . . . . .|. horizontal
///  | | | | |           |
///  c D E F G           |
/// ```
///
/// Where container nodes are represented with "#".
///
/// Note that if a horizontal container is the direct child of another
/// horizontal container, we can merge the two into a single horizontal
/// sequence.
/// We use this property to always keep the tree in its simplest form.
/// In practice this means that the orientation of a container is always
/// the opposite of the orientation of its parent, if any.
///
/// The goal of this data structure is to quickly find neighboring free
/// rectangles that can be coalesced into fewer rectangles.
/// This structure guarantees that two consecutive children of the same
/// container, if both rectangles are free, can be coalesed into a single
/// one.
///
/// An important thing to note about this tree structure is that we only
/// use it to visit niieghbor and parent nodes. As a result we don't care
/// about whether the tree is balanced, although flat sequences of children
/// tend to offer more opportunity for coalescing than deeply nested structures
/// Either way, the cost of finding potential merges is the same because
/// each node stores the indices of their sibblings, and we never have to
/// traverse any global list of free rectangle nodes.
///
/// ### Merging sibblings
///
/// As soon as two consecutive sibbling nodes are marked as "free", they are coalesced
/// into a single node.
///
/// In the example below, we juct deallocated the rectangle `B`, which is a sibblig of
/// `A` which is free and `C` which is still allocated. `A` and `B` are merged and this
/// change is reflected on the tree as shown below:
///
/// ```ascii
/// +---+---+---+         #               +-------+---+         #
/// |   |   |///|         |               |       |///|         |
/// | A | B |/C/|     +---+---+           | AB    |/C/|     +---+---+
/// |   |   |///|     |       |           |       |///|     |       |
/// +---+---+---+     #       D           +-------+---+     #       D
/// | D         |     |            ->     | D         |     |
/// |           |   +-+-+                 |           |   +-+-+
/// |           |   | | |                 |           |   |   |
/// +-----------+   A B C                 +-----------+   AB  C
/// ```
///
/// ### Merging unique children with their parents
///
/// In the previous example `C` was an allocated slot. Let's now deallocate it:
///
/// ```ascii
/// +-------+---+         #               +-----------+         #                 #
/// |       |   |         |               |           |         |                 |
/// | AB    | C |     +---+---+           | ABC       |     +---+---+         +---+---+
/// |       |   |     |       |           |           |     |       |         |       |
/// +-------+---+     #       D           +-----------+     #       D        ABC      D
/// | D         |     |            ->     | D         |     |           ->
/// |           |   +-+-+                 |           |     +
/// |           |   |   |                 |           |     |
/// +-----------+   AB  C                 +-----------+    ABC
/// ```
///
/// Deallocating `C` allowed it to merge with the free rectangle `AB`, making the
/// resulting node `ABC` the only child of its parent container. As a result the
/// node `ABC` was lifted up the tree to replace its parent.
///
/// In this example, assuming `D` to also be a free rectangle, `ABC` and `D` would
/// be immediately merged and the resulting node `ABCD`, also being only child of
/// its parent container, would replace its parent, turning the tree into a single
/// node `ABCD`.
///
/// ### Limitations
///
/// This strategy can miss some opportunities for coalescing free rectangles
/// when the two sibbling containers are split exactly the same way.
///
/// For example:
///
/// ```ascii
/// +---------+------+
/// |    A    |  B   |
/// |         |      |
/// +---------+------+
/// |    C    |  D   |
/// |         |      |
/// +---------+------+
/// ```
///
/// Could be the result of either a vertical followed with two horizontal splits,
/// or an horizontal then two vertical splits.
///
/// ```ascii
///  Tree            | Layout             Tree            | Layout
/// -----------------+------------       -----------------+------------
///         #        |                           #        |
///         |        |                           |        |
///     +---+---+ . .|. Vertical             +---+---+ . .|. Horizontal
///     |       |    |                       |       |    |
///     #       #    |               or      #       #    |
///     |       |    |                       |       |    |
///   +-+-+ . +-+-+ .|. Horizontal         +-+-+ . +-+-+ .|. Vertical
///   |   |   |   |  |                     |   |   |   |  |
///   A   B   C   D  |                     A   C   B   D  |
/// ```
///
/// In the former case A can't be merged with C nor B with D because they are not sibblings.
///
/// For a lot of workloads it is rather rare for two consecutive sibbling containers to be
/// subdivided exactly the same way. In this situation losing the ability to merge rectangles
/// that aren't under the same container is good compromise between the CPU cost of coalescing
/// and the fragmentation of the atlas.
///
/// This algorithm is, however, not the best solution for very "structured" grid-like
/// subdivision patterns where the ability to merge across containers would have provided
/// frequent defragmentation opportunities.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone)]
pub struct AtlasAllocator {
    nodes: Vec<Node>,
    /// Free lists are split into a small a medium and a large bucket for faster lookups.
    free_lists: [Vec<AllocIndex>; NUM_BUCKETS],

    /// Index of the first element of an intrusive linked list of unused nodes.
    /// The `next_sibbling` member of unused node serves as the linked list link.
    unused_nodes: AllocIndex,

    /// We keep a per-node generation counter to reduce the lekelihood of ID reuse bugs
    /// going unnoticed.
    generations: Vec<Wrapping<u8>>,

    /// See `AllocatorOptions`.
    snap_size: i32,

    /// See `AllocatorOptions`.
    small_size_threshold: i32,

    /// See `AllocatorOptions`.
    large_size_threshold: i32,

    /// Total size of the atlas.
    size: DeviceIntSize,
}

impl AtlasAllocator {

    /// Create an atlas allocator.
    pub fn new(size: DeviceIntSize) -> Self {
        AtlasAllocator::with_options(size, &DEFAULT_OPTIONS)
    }

    /// Create an atlas allocator that rounds out the allocated rectangles to multiples
    /// of the provided value.
    pub fn with_options(size: DeviceIntSize, options: &AllocatorOptions) -> Self {
        assert!(options.snap_size > 0);
        assert!(size.width > 0);
        assert!(size.height > 0);
        assert!(options.large_size_threshold >= options.small_size_threshold);

        let mut free_lists = [Vec::new(), Vec::new(), Vec::new()];
        let bucket = free_list_for_size(
            options.small_size_threshold,
            options.large_size_threshold,
            &size
        );
        free_lists[bucket].push(AllocIndex(0));

        AtlasAllocator {
            nodes: vec![Node {
                parent: AllocIndex::NONE,
                next_sibbling: AllocIndex::NONE,
                prev_sibbling: AllocIndex::NONE,
                rect: size.into(),
                kind: NodeKind::Free,
                orientation: Orientation::Vertical,
            }],
            free_lists,
            generations: vec![Wrapping(0)],
            unused_nodes: AllocIndex::NONE,
            snap_size: options.snap_size,
            small_size_threshold: options.small_size_threshold,
            large_size_threshold: options.large_size_threshold,
            size,
        }
    }

    /// The total size of the atlas.
    pub fn size(&self) -> DeviceIntSize {
        self.size
    }

    /// Allocate a rectangle in the atlas.
    pub fn allocate(&mut self, mut requested_size: DeviceIntSize) -> Option<(AllocId, DeviceIntPoint)> {

        self.adjust_size(&mut requested_size.width);
        self.adjust_size(&mut requested_size.height);

        // Find a suitable free rect.
        let chosen_id = self.find_suitable_rect(&requested_size);

        if chosen_id.is_none() {
            //println!("failed to allocate {:?}", requested_size);
            //self.print_free_rects();

            // No suitable free rect!
            return None;
        }

        let chosen_node = self.nodes[chosen_id.index()].clone();
        let current_orientation = chosen_node.orientation;
        assert_eq!(chosen_node.kind, NodeKind::Free);

        // Decide whether to split horizontally or vertically.
        //
        // If the chosen free rectangle is bigger than the requested size, we subdivide it
        // into an allocated rectangle, a split rectange and a leftover rectange:
        //
        // +-----------+-------------+
        // |///////////|             |
        // |/allocated/|             |
        // |///////////|             |
        // +-----------+             |
        // |                         |
        // |          chosen         |
        // |                         |
        // +-------------------------+
        //
        // Will be split into either:
        //
        // +-----------+-------------+
        // |///////////|             |
        // |/allocated/|  leftover   |
        // |///////////|             |
        // +-----------+-------------+
        // |                         |
        // |          split          |
        // |                         |
        // +-------------------------+
        //
        // or:
        //
        // +-----------+-------------+
        // |///////////|             |
        // |/allocated/|             |
        // |///////////|    split    |
        // +-----------+             |
        // |           |             |
        // | leftover  |             |
        // |           |             |
        // +-----------+-------------+

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

        // Update the tree.

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

        Some((self.alloc_id(allocated_id), allocated_rect.origin))
    }

    /// Deallocate a rectangle in the atlas.
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

    pub fn rearrange(&mut self) -> ChangeList {
        let size = self.size;
        self.resize_and_rearrange(size)
    }

    pub fn resize_and_rearrange(&mut self, new_size: DeviceIntSize) -> ChangeList {
        let mut allocs = Vec::with_capacity(self.nodes.len());
        for (i, node) in self.nodes.iter().enumerate() {
            if node.kind != NodeKind::Alloc {
                continue;
            }
            let id = self.alloc_id(AllocIndex(i as u32));
            allocs.push(Allocation { id, rect: node.rect });
        }

        allocs.sort_by_key(|alloc| alloc.rect.size.area());
        allocs.reverse();

        self.nodes.clear();
        self.generations.clear();
        self.unused_nodes = AllocIndex::NONE;
        for i in 0..NUM_BUCKETS {
            self.free_lists[i].clear();
        }

        let bucket = free_list_for_size(
            self.small_size_threshold,
            self.large_size_threshold,
            &new_size
        );
        self.free_lists[bucket].push(AllocIndex(0));

        self.nodes.push(Node {
            parent: AllocIndex::NONE,
            next_sibbling: AllocIndex::NONE,
            prev_sibbling: AllocIndex::NONE,
            rect: new_size.into(),
            kind: NodeKind::Free,
            orientation: Orientation::Vertical,
        });
        self.generations.push(Wrapping(0));

        let mut changes = Vec::new();
        let mut failures = Vec::new();

        for old in allocs {
            let size = old.rect.size;
            if let Some((id, origin)) = self.allocate(size) {
                changes.push(Change {
                    old,
                    new: Allocation {
                        id,
                        rect: DeviceIntRect { origin, size },
                    }
                });
            } else {
                failures.push(old);
            }
        }

        ChangeList {
            changes,
            failures,
        }
    }

    fn find_suitable_rect(&mut self, requested_size: &DeviceIntSize) -> AllocIndex {

        let mut candidate_score = 0;
        let mut candidate = None;
        let mut freelist_idx = 0;
        let smallest_bucket = free_list_for_size(
            self.small_size_threshold,
            self.large_size_threshold,
            requested_size,
        );
        for bucket in smallest_bucket..NUM_BUCKETS {
            while freelist_idx < self.free_lists[bucket].len() {
                let id = self.free_lists[bucket][freelist_idx];

                // During tree simplification we don't remove merged nodes from the free list, so we have
                // to handle it here.
                // This is a tad awkward, but lets us avoid having to maintain a doubly linked list for
                // the free list (which would be needed to remove nodes during tree simplification).
                if self.nodes[id.index()].kind != NodeKind::Free {
                    // remove the element from the free list
                    self.free_lists[bucket].swap_remove(freelist_idx);
                    continue;
                }

                let size = self.nodes[id.index()].rect.size;
                let dx = size.width - requested_size.width;
                let dy = size.height - requested_size.height;

                if dx >= 0 && dy >= 0 {
                    if dx == 0 || dy == 0 {
                        // Perfect fit!
                        candidate = Some((id, freelist_idx));
                        //println!("perfect fit!");
                        break;
                    }

                    // Favor the largest minimum dimmension.
                    let score = i32::min(dx, dy);
                    if score > candidate_score {
                        candidate_score = score;
                        candidate = Some((id, freelist_idx));
                    }
                }

                freelist_idx += 1;
            }

            if let Some((id, freelist_idx)) = candidate {
                self.free_lists[bucket].swap_remove(freelist_idx);
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
        println!("Large:");
        for &id in &self.free_lists[LARGE_BUCKET] {
            if self.nodes[id.index()].kind == NodeKind::Free {
                println!(" - {} #{:?}", self.nodes[id.index()].rect, id);
            }
        }
        println!("Medium:");
        for &id in &self.free_lists[MEDIUM_BUCKET] {
            if self.nodes[id.index()].kind == NodeKind::Free {
                println!(" - {} #{:?}", self.nodes[id.index()].rect, id);
            }
        }
        println!("Small:");
        for &id in &self.free_lists[SMALL_BUCKET] {
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

    fn add_free_rect(&mut self, id: AllocIndex, size: &DeviceIntSize) {
        //println!("add free rect #{:?}", id);
        debug_assert_eq!(self.nodes[id.index()].kind, NodeKind::Free);
        let bucket = free_list_for_size(
            self.small_size_threshold,
            self.large_size_threshold,
            size,
        );
        self.free_lists[bucket].push(id);
    }

    // Merge `next` into `node` and append `next` to a list of available `nodes`vector slots.
    fn merge_sibblings(&mut self, node: AllocIndex, next: AllocIndex, orientation: Orientation) {
        let r1 = self.nodes[node.index()].rect;
        let r2 = self.nodes[next.index()].rect;
        //println!("merge {} #{:?} and {} #{:?}       {:?}", r1, node, r2, next, orientation);
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

pub struct Allocation {
    pub id: AllocId,
    pub rect: DeviceIntRect,
}

pub struct Change {
    pub old: Allocation,
    pub new: Allocation,
}

pub struct ChangeList {
    pub changes: Vec<Change>,
    pub failures: Vec<Allocation>,
}

pub fn dump_svg(atlas: &AtlasAllocator, output: &mut dyn std::io::Write) -> std::io::Result<()> {

    write!(
        output,
r#"<?xml version="1.0" encoding="UTF-8" standalone="no"?>
<svg
   xmlns:dc="http://purl.org/dc/elements/1.1/"
   xmlns:cc="http://creativecommons.org/ns#"
   xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
   xmlns:svg="http://www.w3.org/2000/svg"
   xmlns="http://www.w3.org/2000/svg"
   id="svg8"
   version="1.1"
   viewBox="0 0 {width} {height}"
   width="{width}mm"
   height="{height}mm"
>
  <defs
     id="defs2" />
  <metadata
     id="metadata5">
    <rdf:RDF>
      <cc:Work
         rdf:about="">
        <dc:format>image/svg+xml</dc:format>
        <dc:type
           rdf:resource="http://purl.org/dc/dcmitype/StillImage" />
        <dc:title></dc:title>
      </cc:Work>
    </rdf:RDF>
  </metadata>
  <g>
"#,
        width = atlas.size.width,
        height = atlas.size.height,
    )?;

    for node in &atlas.nodes {
        let style = match node.kind {
            NodeKind::Free => {
                "fill:rgb(200,255,200);stroke-width:1;stroke:rgb(0,0,0)"
            }
            NodeKind::Alloc => {
                "fill:rgb(150,150,255);stroke-width:3;stroke:rgb(0,0,0)"
            }
            _ => { continue; }
        };

        let rect = node.rect;

        writeln!(
            output,
            r#"    <rect x="{}" y="{}" width="{}" height="{}" style="{}" />"#,
            rect.origin.x,
            rect.origin.y,
            rect.size.width,
            rect.size.height,
            style,
        )?;
    }

    writeln!(output, "</g></svg>" )
}

#[test]
fn atlas_simple() {
    let mut atlas = AtlasAllocator::new(size2(1000, 1000));

    let full = atlas.allocate(size2(1000,1000)).unwrap();
    assert!(atlas.allocate(size2(1, 1)).is_none());

    atlas.deallocate(full);

    let a = atlas.allocate(size2(100, 1000)).unwrap().0;
    let b = atlas.allocate(size2(900, 200)).unwrap().0;
    let c = atlas.allocate(size2(300, 200)).unwrap().0;
    let d = atlas.allocate(size2(200, 300)).unwrap().0;
    let e = atlas.allocate(size2(100, 300)).unwrap().0;
    let f = atlas.allocate(size2(100, 300)).unwrap().0;
    let g = atlas.allocate(size2(100, 300)).unwrap().0;

    atlas.deallocate(b);
    atlas.deallocate(f);
    atlas.deallocate(c);
    atlas.deallocate(e);
    let h = atlas.allocate(size2(500, 200)).unwrap().0;
    atlas.deallocate(a);
    let i = atlas.allocate(size2(500, 200)).unwrap().0;
    atlas.deallocate(g);
    atlas.deallocate(h);
    atlas.deallocate(d);
    atlas.deallocate(i);

    let full = atlas.allocate(size2(1000,1000)).unwrap().0;
    assert!(atlas.allocate(size2(1, 1)).is_none());
    atlas.deallocate(full);
}

#[test]
fn atlas_random_test() {
    let mut atlas = AtlasAllocator::with_options(
        size2(1000, 1000),
        &AllocatorOptions {
            snap_size: 5,
            ..DEFAULT_OPTIONS
        }
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

            if let Some((id, _)) = atlas.allocate(size) {
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
    println!(
        "nodes.cap: {}, free_list.cap: {}/{}/{}",
        atlas.nodes.capacity(),
        atlas.free_lists[LARGE_BUCKET].capacity(),
        atlas.free_lists[MEDIUM_BUCKET].capacity(),
        atlas.free_lists[SMALL_BUCKET].capacity(),
    );

    let full = atlas.allocate(size2(1000,1000)).unwrap().0;
    assert!(atlas.allocate(size2(1, 1)).is_none());
    atlas.deallocate(full);
}


