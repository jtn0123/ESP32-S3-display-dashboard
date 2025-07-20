// Enhanced dirty rectangle management for optimized display updates

use super::DirtyRect;

const MAX_DIRTY_RECTS: usize = 16;
const MERGE_THRESHOLD: usize = 10;

/// Manages multiple dirty rectangles with automatic merging
pub struct DirtyRectManager {
    rects: [Option<DirtyRect>; MAX_DIRTY_RECTS],
    count: usize,
    // Stats for monitoring
    merge_count: u32,
    update_count: u32,
}

impl DirtyRectManager {
    pub fn new() -> Self {
        Self {
            rects: [None; MAX_DIRTY_RECTS],
            count: 0,
            merge_count: 0,
            update_count: 0,
        }
    }
    
    /// Add a dirty rectangle to the manager
    pub fn add_rect(&mut self, x: u16, y: u16, width: u16, height: u16) {
        if width == 0 || height == 0 {
            return;
        }
        
        let new_rect = DirtyRect::new(x, y, width, height);
        
        // Try to merge with existing rectangles
        for i in 0..self.count {
            if let Some(rect) = self.rects[i] {
                // Check if rectangles overlap or are adjacent
                if self.should_merge(&rect, &new_rect) {
                    // Merge the rectangles
                    let mut merged = rect;
                    merged.merge(&new_rect);
                    self.rects[i] = Some(merged);
                    self.merge_count += 1;
                    self.coalesce_overlapping();
                    return;
                }
            }
        }
        
        // Add as new rectangle if we have space
        if self.count < MAX_DIRTY_RECTS {
            self.rects[self.count] = Some(new_rect);
            self.count += 1;
        } else {
            // Too many rectangles - merge all into one
            self.merge_all();
            self.add_rect(x, y, width, height);
        }
        
        // Auto-merge if we're getting too many rectangles
        if self.count >= MERGE_THRESHOLD {
            self.merge_all();
        }
    }
    
    /// Check if two rectangles should be merged
    fn should_merge(&self, rect1: &DirtyRect, rect2: &DirtyRect) -> bool {
        // Check if rectangles overlap
        let overlap = !(rect1.x + rect1.width < rect2.x ||
                       rect2.x + rect2.width < rect1.x ||
                       rect1.y + rect1.height < rect2.y ||
                       rect2.y + rect2.height < rect1.y);
        
        if overlap {
            return true;
        }
        
        // Check if rectangles are adjacent (within 8 pixels)
        const ADJACENCY_THRESHOLD: u16 = 8;
        
        // Horizontal adjacency
        if rect1.y < rect2.y + rect2.height + ADJACENCY_THRESHOLD &&
           rect2.y < rect1.y + rect1.height + ADJACENCY_THRESHOLD {
            if rect1.x + rect1.width + ADJACENCY_THRESHOLD >= rect2.x &&
               rect1.x <= rect2.x {
                return true;
            }
            if rect2.x + rect2.width + ADJACENCY_THRESHOLD >= rect1.x &&
               rect2.x <= rect1.x {
                return true;
            }
        }
        
        // Vertical adjacency
        if rect1.x < rect2.x + rect2.width + ADJACENCY_THRESHOLD &&
           rect2.x < rect1.x + rect1.width + ADJACENCY_THRESHOLD {
            if rect1.y + rect1.height + ADJACENCY_THRESHOLD >= rect2.y &&
               rect1.y <= rect2.y {
                return true;
            }
            if rect2.y + rect2.height + ADJACENCY_THRESHOLD >= rect1.y &&
               rect2.y <= rect1.y {
                return true;
            }
        }
        
        false
    }
    
    /// Coalesce any overlapping rectangles
    fn coalesce_overlapping(&mut self) {
        let mut changed = true;
        while changed {
            changed = false;
            
            for i in 0..self.count {
                if self.rects[i].is_none() {
                    continue;
                }
                
                for j in (i + 1)..self.count {
                    if let (Some(rect1), Some(rect2)) = (self.rects[i], self.rects[j]) {
                        if self.should_merge(&rect1, &rect2) {
                            let mut merged = rect1;
                            merged.merge(&rect2);
                            self.rects[i] = Some(merged);
                            self.rects[j] = None;
                            changed = true;
                            self.merge_count += 1;
                        }
                    }
                }
            }
            
            // Compact the array
            if changed {
                self.compact();
            }
        }
    }
    
    /// Merge all rectangles into one bounding box
    pub fn merge_all(&mut self) {
        if self.count == 0 {
            return;
        }
        
        let mut bounds = self.rects[0].unwrap();
        
        for i in 1..self.count {
            if let Some(rect) = self.rects[i] {
                bounds.merge(&rect);
            }
        }
        
        self.clear();
        self.rects[0] = Some(bounds);
        self.count = 1;
        self.merge_count += 1;
    }
    
    /// Remove None entries and compact the array
    fn compact(&mut self) {
        let mut write_idx = 0;
        
        for read_idx in 0..MAX_DIRTY_RECTS {
            if self.rects[read_idx].is_some() {
                if write_idx != read_idx {
                    self.rects[write_idx] = self.rects[read_idx];
                    self.rects[read_idx] = None;
                }
                write_idx += 1;
            }
        }
        
        self.count = write_idx;
    }
    
    /// Get iterator over dirty rectangles
    pub fn iter(&self) -> impl Iterator<Item = &DirtyRect> {
        self.rects[0..self.count]
            .iter()
            .filter_map(|r| r.as_ref())
    }
    
    /// Check if there are any dirty rectangles
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    
    /// Clear all dirty rectangles
    pub fn clear(&mut self) {
        for i in 0..self.count {
            self.rects[i] = None;
        }
        self.count = 0;
        self.update_count += 1;
    }
    
    /// Get statistics
    pub fn get_stats(&self) -> (usize, u32, u32) {
        (self.count, self.merge_count, self.update_count)
    }
    
    /// Calculate total area covered by dirty rectangles
    pub fn total_area(&self) -> u32 {
        self.iter()
            .map(|r| r.width as u32 * r.height as u32)
            .sum()
    }
}