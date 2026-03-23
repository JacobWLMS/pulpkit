//! Damage rectangle tracking and merging.

/// A damage rectangle in surface-local pixel coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl DamageRect {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self { x, y, width, height }
    }

    /// Compute the union (bounding box) of two rects.
    pub fn union(self, other: DamageRect) -> DamageRect {
        let x1 = self.x.min(other.x);
        let y1 = self.y.min(other.y);
        let x2 = (self.x + self.width).max(other.x + other.width);
        let y2 = (self.y + self.height).max(other.y + other.height);
        DamageRect {
            x: x1,
            y: y1,
            width: x2 - x1,
            height: y2 - y1,
        }
    }

    /// Check if two rects overlap.
    pub fn overlaps(&self, other: &DamageRect) -> bool {
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }

    /// Expand the rect by `margin` pixels on all sides.
    pub fn expand(&self, margin: i32) -> DamageRect {
        DamageRect {
            x: self.x - margin,
            y: self.y - margin,
            width: self.width + margin * 2,
            height: self.height + margin * 2,
        }
    }

    /// Return the area in pixels.
    pub fn area(&self) -> i32 {
        self.width * self.height
    }
}

/// Merge overlapping or nearby damage rects to reduce clip/restore cycles.
///
/// Rects within `merge_distance` pixels of each other are merged. The algorithm
/// is greedy: expand each rect, check for overlaps, merge unions until stable.
pub fn merge_damage(rects: Vec<DamageRect>, merge_distance: i32) -> Vec<DamageRect> {
    if rects.len() <= 1 {
        return rects;
    }

    let mut merged = rects;
    let mut changed = true;

    while changed {
        changed = false;
        let mut i = 0;
        while i < merged.len() {
            let mut j = i + 1;
            while j < merged.len() {
                let expanded_i = merged[i].expand(merge_distance);
                if expanded_i.overlaps(&merged[j]) {
                    merged[i] = merged[i].union(merged[j]);
                    merged.remove(j);
                    changed = true;
                } else {
                    j += 1;
                }
            }
            i += 1;
        }
    }

    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn union_bounding_box() {
        let a = DamageRect::new(10, 10, 20, 20);
        let b = DamageRect::new(25, 15, 20, 20);
        let u = a.union(b);
        assert_eq!(u, DamageRect::new(10, 10, 35, 25));
    }

    #[test]
    fn overlapping_rects() {
        let a = DamageRect::new(0, 0, 20, 20);
        let b = DamageRect::new(10, 10, 20, 20);
        assert!(a.overlaps(&b));
        assert!(b.overlaps(&a));
    }

    #[test]
    fn non_overlapping_rects() {
        let a = DamageRect::new(0, 0, 10, 10);
        let b = DamageRect::new(50, 50, 10, 10);
        assert!(!a.overlaps(&b));
    }

    #[test]
    fn merge_overlapping() {
        let rects = vec![
            DamageRect::new(0, 0, 20, 20),
            DamageRect::new(15, 15, 20, 20),
        ];
        let merged = merge_damage(rects, 0);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0], DamageRect::new(0, 0, 35, 35));
    }

    #[test]
    fn merge_nearby_within_distance() {
        let rects = vec![
            DamageRect::new(0, 0, 10, 10),
            DamageRect::new(15, 0, 10, 10), // 5px gap, within merge_distance=8
        ];
        let merged = merge_damage(rects, 8);
        assert_eq!(merged.len(), 1);
    }

    #[test]
    fn no_merge_distant() {
        let rects = vec![
            DamageRect::new(0, 0, 10, 10),
            DamageRect::new(100, 100, 10, 10),
        ];
        let merged = merge_damage(rects, 8);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn single_rect_unchanged() {
        let rects = vec![DamageRect::new(5, 5, 20, 20)];
        let merged = merge_damage(rects, 8);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0], DamageRect::new(5, 5, 20, 20));
    }

    #[test]
    fn empty_input() {
        let merged = merge_damage(vec![], 8);
        assert!(merged.is_empty());
    }
}
