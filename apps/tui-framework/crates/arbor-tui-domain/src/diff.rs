// Diff algorithm — row-by-row comparison of two VirtualScreens.
// Outputs a list of DirtyRegion for the backend to emit.

use crate::screen::VirtualScreen;

/// A dirty (changed) region within a single row.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct DirtyRegion {
    pub row: u16,
    pub start_col: u16,
    pub end_col: u16, // exclusive
}

/// Compare two VirtualScreens row by row.
///
/// Returns a list of dirty regions. A single row may produce multiple
/// discontinuous DirtyRegions (e.g., only col 2 and col 7 changed).
/// Region merging is deferred to the backend's `emit()`.
pub fn diff(old: &VirtualScreen, new: &VirtualScreen) -> Vec<DirtyRegion> {
    let mut regions = Vec::new();
    let rows = old.rows().min(new.rows());
    let cols = old.cols().min(new.cols());
    let cols_usize = cols as usize;

    for row in 0..rows {
        let old_row = old
            .row_cells(row)
            .expect("row is bounded by old screen size");
        let new_row = new
            .row_cells(row)
            .expect("row is bounded by new screen size");
        let old_row = &old_row[..cols_usize];
        let new_row = &new_row[..cols_usize];

        if old_row == new_row {
            continue;
        }

        let mut in_dirty = false;
        let mut dirty_start: u16 = 0;

        for (col, (a, b)) in old_row.iter().zip(new_row).enumerate() {
            if a != b && !in_dirty {
                in_dirty = true;
                dirty_start = col as u16;
            } else if a == b && in_dirty {
                in_dirty = false;
                regions.push(DirtyRegion {
                    row,
                    start_col: dirty_start,
                    end_col: col as u16,
                });
            }
        }

        if in_dirty {
            regions.push(DirtyRegion {
                row,
                start_col: dirty_start,
                end_col: cols,
            });
        }
    }

    // Old screen has rows the new screen doesn't — mark as full-row dirty (cleared)
    for row in new.rows()..old.rows() {
        regions.push(DirtyRegion {
            row,
            start_col: 0,
            end_col: old.cols().min(new.cols()),
        });
    }

    // New screen has rows the old screen doesn't — mark as full-row dirty (new)
    for row in old.rows()..new.rows() {
        regions.push(DirtyRegion {
            row,
            start_col: 0,
            end_col: new.cols(),
        });
    }

    regions
}

/// Merge adjacent dirty regions (same row, touching or overlapping).
/// Used by the backend before emitting ANSI sequences.
pub fn merge_regions(regions: &mut Vec<DirtyRegion>) {
    if regions.is_empty() {
        return;
    }

    // Sort by (row, start_col)
    regions.sort_by(|a, b| a.row.cmp(&b.row).then(a.start_col.cmp(&b.start_col)));

    let mut merged = Vec::with_capacity(regions.len());
    let mut current = regions[0];

    for next in &regions[1..] {
        if next.row == current.row && next.start_col <= current.end_col {
            // Same row, touching or overlapping — extend
            current.end_col = current.end_col.max(next.end_col);
        } else {
            merged.push(current);
            current = *next;
        }
    }
    merged.push(current);
    *regions = merged;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_screens_no_diff() {
        let old = VirtualScreen::new(10, 5);
        let new = VirtualScreen::new(10, 5);
        let r = diff(&old, &new);
        assert!(r.is_empty());
    }

    #[test]
    fn single_cell_change() {
        let old = VirtualScreen::new(10, 5);
        let mut new = VirtualScreen::new(10, 5);
        new.cell_at_mut(3, 1).unwrap().ch = 'X';

        let r = diff(&old, &new);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].row, 1);
        assert_eq!(r[0].start_col, 3);
        assert_eq!(r[0].end_col, 4);
    }

    #[test]
    fn new_extra_rows() {
        let old = VirtualScreen::new(10, 3);
        let new = VirtualScreen::new(10, 5);
        let r = diff(&old, &new);
        // rows 3,4 are new — two full-row dirty regions
        let extra: Vec<_> = r.iter().filter(|d| d.row >= 3).collect();
        assert_eq!(extra.len(), 2);
    }

    #[test]
    fn old_extra_rows() {
        let old = VirtualScreen::new(10, 5);
        let new = VirtualScreen::new(10, 3);
        let r = diff(&old, &new);
        // rows 3,4 were removed — two clear dirty regions
        let removed: Vec<_> = r.iter().filter(|d| d.row >= 3).collect();
        assert_eq!(removed.len(), 2);
    }

    #[test]
    fn merge_adjacent_regions() {
        let mut regions = vec![
            DirtyRegion {
                row: 0,
                start_col: 0,
                end_col: 2,
            },
            DirtyRegion {
                row: 0,
                start_col: 2,
                end_col: 5,
            },
            DirtyRegion {
                row: 1,
                start_col: 3,
                end_col: 4,
            },
        ];
        merge_regions(&mut regions);
        assert_eq!(regions.len(), 2);
        assert_eq!(
            regions[0],
            DirtyRegion {
                row: 0,
                start_col: 0,
                end_col: 5
            }
        );
    }
}
