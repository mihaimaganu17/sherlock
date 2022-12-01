#![no_std]

use core::cmp;

/// An inclusive range, we do not use `RangeInclusive` as it does not implement `Copy`
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Range {
    pub start: u64,
    pub end: u64,
}

/// A set of non-overlapping inclusive `u64` ranges
#[derive(Clone, Copy)]
#[repr(C)]
pub struct RangeSet {
    /// Fixed array of ranges in the set
    ranges: [Range; 32],
    
    /// Number of used entries
    in_use: u32,
}

impl RangeSet {
    pub const fn new() -> RangeSet {
        RangeSet {
            ranges: [Range{ start: 0, end: 0}; 32],
            in_use: 0,
        }
    }

    pub fn entries(&self) -> &[Range] {
        &self.ranges[..self.in_use as usize]
    }

    fn delete(&mut self, idx: usize) {
        assert!(idx < self.in_use as usize, "Index out of bounds");

        // Copy the deleted range to the end of the list
        for ii in idx..self.in_use as usize - 1 {
            self.ranges.swap(ii, ii+1);
        }
        
        // Decrement the number of valid ranges
        self.in_use -= 1;
    }

    pub fn insert(&mut self, mut range: Range) {
        assert!(range.start <= range.end, "Invalid range shape");

        'try_merges: loop {
            for ii in 0..self.in_use as usize {
                let ent = self.ranges[ii];

                // This is done so that two ranges that are 'touching' but not overlapping will
                // be combined
                if !overlaps(range.start, range.end.saturating_add(1), ent.start,
                    ent.end.saturating_add(1)
                ) {
                    continue;
                }
                
                // There was overlap, Make this combination of the existing ranges.
                range.start = cmp::min(range.start, ent.start);
                range.end = cmp::min(range.end, ent.end);

                // Delete the old range, as the new one is now all inclusive
                self.delete(ii);

                continue 'try_merges;
            }

            break;
        }

        assert!((self.in_use as usize) < self.ranges.len(), "Too many entries in RangeSet on insert");

        // Add the new range to the end
        self.ranges[self.in_use as usize] = range;
        self.in_use += 1;
    }

    pub fn remove(&mut self, range: Range) {
        assert!(range.start <= range.end, "Invalid range shape");

        'try_subtractions: loop {
            for ii in 0..self.in_use as usize {
                let ent = self.ranges[ii];

                // If there is not overlap, there is nothing to do with this range.
                if !overlaps(range.start, range.end, ent.start, ent.end) {
                    continue;
                }

                // If this entry is entirely contained by the range to remove, then we can just
                // delete it
                if contains(ent.start, ent.end, range.start, range.end) {
                    self.delete(ii);
                    continue 'try_subtractions;
                }

                if range.start <= ent.start {
                    self.ranges[ii].start = range.end.saturating_add(1);
                } else if range.end >= ent.end {
                    self.ranges[ii].end = range.start.saturating_add(1);
                } else {
                    self.ranges[ii].start = range.end.saturating_add(1);

                    assert!((self.in_use as usize) < self.ranges.len(),
                        "Too many entries in RangeSet on split");

                    self.ranges[self.in_use as usize] = Range {
                        start: ent.start,
                        end: range.start.saturating_sub(1),
                    };

                    self.in_use += 1;
                    continue 'try_subtractions;
                }
            }

            break;
        }
    }

    pub fn subtract(&mut self, rs: &RangeSet) {
        for &ent in rs.entries() {
            self.remove(ent);
        }
    }

    pub fn sum(&self) -> Option<u64> {
        self.entries().iter()
            .try_fold(0u64, |acc, x| (Some(acc + (x.end - x.start).checked_add(1)?)))
    }

    pub fn allocate(&mut self, size: u64, align: u64) -> Option<usize> {
        if size == 0 { return None; }

        // Validate alignment is non-zero and a power of 2
        if align.count_ones() != 1 {
            return None;
        }

        // Generate a mask for the specified alignment
        let alignmask = align - 1;

        let mut allocation = None;

        // Go trough each memory range in the `RangeSet`
        for ent in self.entries() {
            // Determine number of bytes required for front padding to satisfy alignment reqs
            let align_fix = (align - (ent.start & alignmask)) & alignmask;
            
            // Compute base and end of allocation as an inclusive range
            let base = ent.start;
            let end = base.checked_add(size - 1)?.checked_add(align_fix)?;

            // Validate that this allocation is addressable in the current processor state.
            if base > core::usize::MAX as u64 || end > core::usize::MAX as u64 {
                continue;
            }

            // Check that this entry has enough room to satisfy allocation
            if end > ent.end {
                continue;
            }

            let prev_size = allocation.map(|(base, end, _)| end - base);

            if allocation.is_none() || prev_size.unwrap() > end -base {
            // Allocation successful
                allocation = Some((base, end, (base + align_fix) as usize));
            }
        }

        allocation.map(|(base, end, ptr)| {
            // Remove this range from the available set
            self.remove(Range {start: base, end: end});

            // Return out the pointer
            ptr
        })
    }
}

fn overlaps(mut x1: u64, mut x2: u64, mut y1: u64, mut y2: u64) -> bool {
    if x1 > x2 {
        core::mem::swap(&mut x1, &mut x2);
    }

    if y1 > y2 {
        core::mem::swap(&mut y1, &mut y2);
    }

    if x1 <= y2 && y1 <= x2 {
        true
    } else {
        false
    }
}

fn contains(mut x1: u64, mut x2: u64, mut y1: u64, mut y2: u64) -> bool {
    if x1 > x2 {
        core::mem::swap(&mut x1, &mut x2);
    }

    if y1 > y2 {
        core::mem::swap(&mut y1, &mut y2);
    }

    if x1 >= y1 && x2 <= y2 {
        true
    } else {
        false
    }
}
