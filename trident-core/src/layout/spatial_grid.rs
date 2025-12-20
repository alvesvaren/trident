// Spatial hash grid for efficient overlap detection.
//
// Instead of O(n) checks against all placed rectangles, this provides O(1) average
// lookup by dividing the layout space into cells.

use super::RectI;
use std::collections::HashMap;

/// A spatial hash grid for efficient rectangle overlap queries.
#[derive(Debug, Clone)]
pub struct SpatialGrid {
    /// Size of each cell in the grid.
    cell_size: i32,
    /// Map from cell coordinates to list of rectangles overlapping that cell.
    cells: HashMap<(i32, i32), Vec<RectI>>,
}

impl SpatialGrid {
    /// Create a new spatial grid with the given cell size.
    /// Cell size should be roughly the size of the largest expected item.
    pub fn new(cell_size: i32) -> Self {
        Self {
            cell_size: cell_size.max(1), // Avoid division by zero
            cells: HashMap::new(),
        }
    }

    /// Compute which cells a rectangle overlaps.
    fn cell_range(&self, rect: &RectI) -> Vec<(i32, i32)> {
        let min_x = rect.x.div_euclid(self.cell_size);
        let max_x = (rect.right() - 1).div_euclid(self.cell_size);
        let min_y = rect.y.div_euclid(self.cell_size);
        let max_y = (rect.bottom() - 1).div_euclid(self.cell_size);

        let mut cells = Vec::new();
        for cx in min_x..=max_x {
            for cy in min_y..=max_y {
                cells.push((cx, cy));
            }
        }
        cells
    }

    /// Insert a rectangle into the grid.
    pub fn insert(&mut self, rect: RectI) {
        for cell in self.cell_range(&rect) {
            self.cells.entry(cell).or_default().push(rect);
        }
    }

    /// Query for rectangles that might overlap the given rectangle.
    /// Returns all rectangles in cells that the query overlaps.
    /// Note: This may include false positives; caller should do exact overlap check.
    pub fn query(&self, rect: &RectI) -> Vec<RectI> {
        let mut result = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for cell in self.cell_range(rect) {
            if let Some(rects) = self.cells.get(&cell) {
                for r in rects {
                    // Use position as key to dedupe (assumes no two rects at same position)
                    let key = (r.x, r.y, r.w, r.h);
                    if seen.insert(key) {
                        result.push(*r);
                    }
                }
            }
        }
        result
    }

    /// Check if the given rectangle overlaps any rectangle in the grid.
    pub fn overlaps_any(&self, rect: &RectI) -> bool {
        for candidate in self.query(rect) {
            if rect.overlaps(&candidate) {
                return true;
            }
        }
        false
    }

    /// Clear the grid.
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.cells.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_query() {
        let mut grid = SpatialGrid::new(100);
        let r1 = RectI { x: 0, y: 0, w: 50, h: 50 };
        let r2 = RectI { x: 200, y: 200, w: 50, h: 50 };

        grid.insert(r1);
        grid.insert(r2);

        // Query near r1 should find r1
        let nearby = grid.query(&RectI { x: 10, y: 10, w: 20, h: 20 });
        assert!(nearby.contains(&r1));
        assert!(!nearby.contains(&r2));
    }

    #[test]
    fn test_overlaps_any() {
        let mut grid = SpatialGrid::new(100);
        grid.insert(RectI { x: 0, y: 0, w: 50, h: 50 });

        assert!(grid.overlaps_any(&RectI { x: 25, y: 25, w: 50, h: 50 }));
        assert!(!grid.overlaps_any(&RectI { x: 100, y: 100, w: 50, h: 50 }));
    }
}
