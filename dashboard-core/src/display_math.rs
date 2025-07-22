/// Display coordinate calculations and transformations
/// This module contains pure functions that can be tested without hardware

/// Rectangle structure for display operations
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self { x, y, width, height }
    }
    
    /// Check if this rectangle intersects with another
    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }
    
    /// Calculate the union of two rectangles
    pub fn union(&self, other: &Rect) -> Rect {
        let x1 = self.x.min(other.x);
        let y1 = self.y.min(other.y);
        let x2 = (self.x + self.width).max(other.x + other.width);
        let y2 = (self.y + self.height).max(other.y + other.height);
        
        Rect::new(x1, y1, x2 - x1, y2 - y1)
    }
    
    /// Calculate the intersection of two rectangles
    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        if !self.intersects(other) {
            return None;
        }
        
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = (self.x + self.width).min(other.x + other.width);
        let y2 = (self.y + self.height).min(other.y + other.height);
        
        Some(Rect::new(x1, y1, x2 - x1, y2 - y1))
    }
    
    /// Calculate area
    pub fn area(&self) -> u32 {
        self.width as u32 * self.height as u32
    }
}

/// Convert coordinates for different display orientations
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Orientation {
    Portrait,
    Landscape,
    PortraitFlipped,
    LandscapeFlipped,
}

pub fn transform_coordinates(x: u16, y: u16, width: u16, height: u16, orientation: Orientation) -> (u16, u16) {
    match orientation {
        Orientation::Portrait => (x, y),
        Orientation::Landscape => (y, width - 1 - x),
        Orientation::PortraitFlipped => (width - 1 - x, height - 1 - y),
        Orientation::LandscapeFlipped => (height - 1 - y, x),
    }
}

/// Calculate optimal dirty rectangles for display updates
pub fn merge_dirty_rects(rects: &[Rect], max_rects: usize) -> Vec<Rect> {
    if rects.is_empty() {
        return vec![];
    }
    
    if rects.len() <= max_rects {
        return rects.to_vec();
    }
    
    // Simple algorithm: merge rectangles that are close together
    let mut result = vec![rects[0]];
    
    for rect in &rects[1..] {
        let mut merged = false;
        
        for existing in &mut result {
            // If rectangles are close or overlapping, merge them
            let expanded_existing = Rect::new(
                existing.x.saturating_sub(10),
                existing.y.saturating_sub(10),
                existing.width + 20,
                existing.height + 20,
            );
            
            if expanded_existing.intersects(rect) {
                *existing = existing.union(rect);
                merged = true;
                break;
            }
        }
        
        if !merged && result.len() < max_rects {
            result.push(*rect);
        }
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_intersection() {
        let r1 = Rect::new(0, 0, 100, 100);
        let r2 = Rect::new(50, 50, 100, 100);
        
        assert!(r1.intersects(&r2));
        
        let intersection = r1.intersection(&r2).unwrap();
        assert_eq!(intersection, Rect::new(50, 50, 50, 50));
    }
    
    #[test]
    fn test_rect_union() {
        let r1 = Rect::new(0, 0, 100, 100);
        let r2 = Rect::new(50, 50, 100, 100);
        
        let union = r1.union(&r2);
        assert_eq!(union, Rect::new(0, 0, 150, 150));
    }
    
    #[test]
    fn test_coordinate_transform() {
        // Test landscape transformation
        let (tx, ty) = transform_coordinates(10, 20, 320, 170, Orientation::Landscape);
        assert_eq!(tx, 20);
        assert_eq!(ty, 309); // 320 - 1 - 10
    }
    
    #[test]
    fn test_dirty_rect_merging() {
        let rects = vec![
            Rect::new(0, 0, 10, 10),
            Rect::new(5, 5, 10, 10), // Overlaps with first
            Rect::new(100, 100, 10, 10), // Far away
        ];
        
        let merged = merge_dirty_rects(&rects, 2);
        assert_eq!(merged.len(), 2);
        
        // First two should be merged
        assert_eq!(merged[0], Rect::new(0, 0, 15, 15));
        assert_eq!(merged[1], Rect::new(100, 100, 10, 10));
    }
    
    #[test]
    fn test_rect_area() {
        let rect = Rect::new(0, 0, 320, 170);
        assert_eq!(rect.area(), 54400);
    }
    
    // Property-based test using quickcheck
    #[quickcheck]
    fn prop_intersection_commutative(r1: (u16, u16, u16, u16), r2: (u16, u16, u16, u16)) -> bool {
        let rect1 = Rect::new(r1.0 % 100, r1.1 % 100, r1.2 % 50 + 1, r1.3 % 50 + 1);
        let rect2 = Rect::new(r2.0 % 100, r2.1 % 100, r2.2 % 50 + 1, r2.3 % 50 + 1);
        
        rect1.intersection(&rect2) == rect2.intersection(&rect1)
    }
}