// Graph and chart components for data visualization

use crate::display::{Display, Color, FontRenderer};
use heapless::Vec;

#[derive(Debug, Clone, Copy)]
pub struct DataPoint {
    pub value: f32,
    pub timestamp: u32, // Seconds since start
}

pub struct LineGraph {
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    data: Vec<DataPoint, 100>,
    min_value: f32,
    max_value: f32,
    auto_scale: bool,
    grid_color: Color,
    line_color: Color,
    background_color: Color,
    title: heapless::String<32>,
}

impl LineGraph {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
            data: Vec::new(),
            min_value: 0.0,
            max_value: 100.0,
            auto_scale: true,
            grid_color: Color(0x2104), // Dark gray
            line_color: Color::GREEN,
            background_color: Color::BLACK,
            title: heapless::String::new(),
        }
    }
    
    pub fn set_title(&mut self, title: &str) {
        self.title.clear();
        self.title.push_str(title).ok();
    }
    
    pub fn set_range(&mut self, min: f32, max: f32) {
        self.min_value = min;
        self.max_value = max;
        self.auto_scale = false;
    }
    
    pub fn add_point(&mut self, value: f32, timestamp: u32) {
        if self.data.push(DataPoint { value, timestamp }).is_err() {
            // Remove oldest point if buffer is full
            self.data.remove(0);
            self.data.push(DataPoint { value, timestamp }).ok();
        }
        
        if self.auto_scale {
            self.update_scale();
        }
    }
    
    pub fn clear(&mut self) {
        self.data.clear();
    }
    
    fn update_scale(&mut self) {
        if let Some(first) = self.data.first() {
            self.min_value = first.value;
            self.max_value = first.value;
            
            for point in &self.data {
                if point.value < self.min_value {
                    self.min_value = point.value;
                }
                if point.value > self.max_value {
                    self.max_value = point.value;
                }
            }
            
            // Add some padding
            let range = self.max_value - self.min_value;
            self.min_value -= range * 0.1;
            self.max_value += range * 0.1;
        }
    }
    
    pub fn draw(&self, display: &mut Display) {
        // Background
        display.fill_rect(self.x, self.y, self.width, self.height, self.background_color);
        
        // Title
        if !self.title.is_empty() {
            display.draw_text_centered_5x7(self.x, self.y + 2, self.width, &self.title, Color::WHITE);
        }
        
        let graph_y = if self.title.is_empty() { self.y } else { self.y + 12 };
        let graph_height = if self.title.is_empty() { self.height } else { self.height - 12 };
        
        // Grid
        self.draw_grid(display, graph_y, graph_height);
        
        // Data line
        if self.data.len() >= 2 {
            let points_per_pixel = self.data.len() as f32 / self.width as f32;
            
            for i in 1..self.width {
                let data_index1 = ((i - 1) as f32 * points_per_pixel) as usize;
                let data_index2 = (i as f32 * points_per_pixel) as usize;
                
                if data_index1 < self.data.len() && data_index2 < self.data.len() {
                    let point1 = &self.data[data_index1];
                    let point2 = &self.data[data_index2];
                    
                    let y1 = self.value_to_y(point1.value, graph_y, graph_height);
                    let y2 = self.value_to_y(point2.value, graph_y, graph_height);
                    
                    display.draw_line(self.x + i - 1, y1, self.x + i, y2, self.line_color);
                }
            }
        }
        
        // Border
        display.draw_rect(self.x, graph_y, self.width, graph_height, Color::WHITE);
    }
    
    fn draw_grid(&self, display: &mut Display, graph_y: u16, graph_height: u16) {
        // Horizontal grid lines
        for i in 1..5 {
            let y = graph_y + (graph_height * i / 5);
            for x in (self.x..self.x + self.width).step_by(4) {
                display.set_pixel(x, y, self.grid_color);
            }
        }
        
        // Vertical grid lines
        for i in 1..8 {
            let x = self.x + (self.width * i / 8);
            for y in (graph_y..graph_y + graph_height).step_by(4) {
                display.set_pixel(x, y, self.grid_color);
            }
        }
    }
    
    fn value_to_y(&self, value: f32, graph_y: u16, graph_height: u16) -> u16 {
        let normalized = (value - self.min_value) / (self.max_value - self.min_value);
        let y = graph_height as f32 * (1.0 - normalized);
        (graph_y as f32 + y) as u16
    }
}

pub struct BarChart {
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    values: Vec<(heapless::String<8>, f32), 8>,
    max_value: f32,
    bar_color: Color,
    background_color: Color,
    show_values: bool,
}

impl BarChart {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
            values: Vec::new(),
            max_value: 100.0,
            bar_color: Color::PRIMARY_BLUE,
            background_color: Color::BLACK,
            show_values: true,
        }
    }
    
    pub fn add_bar(&mut self, label: &str, value: f32) -> Result<(), ()> {
        let mut label_string = heapless::String::new();
        label_string.push_str(label).ok();
        self.values.push((label_string, value))
    }
    
    pub fn clear(&mut self) {
        self.values.clear();
    }
    
    pub fn set_max_value(&mut self, max: f32) {
        self.max_value = max;
    }
    
    pub fn draw(&self, display: &mut Display) {
        // Background
        display.fill_rect(self.x, self.y, self.width, self.height, self.background_color);
        
        if self.values.is_empty() {
            return;
        }
        
        let bar_width = (self.width - 4) / self.values.len() as u16;
        let spacing = 2;
        
        for (i, (label, value)) in self.values.iter().enumerate() {
            let bar_x = self.x + 2 + (i as u16 * bar_width);
            let bar_height = ((value / self.max_value) * (self.height - 20) as f32) as u16;
            let bar_y = self.y + self.height - bar_height - 15;
            
            // Draw bar
            display.fill_rect(
                bar_x + spacing,
                bar_y,
                bar_width - spacing * 2,
                bar_height,
                self.bar_color
            );
            
            // Draw value on top
            if self.show_values {
                let value_text = format_float(*value);
                display.draw_text_centered_5x7(
                    bar_x,
                    bar_y - 10,
                    bar_width,
                    &value_text,
                    Color::WHITE
                );
            }
            
            // Draw label at bottom
            display.draw_text_centered_5x7(
                bar_x,
                self.y + self.height - 12,
                bar_width,
                label,
                Color::WHITE
            );
        }
        
        // Border
        display.draw_rect(self.x, self.y, self.width, self.height - 15, Color::WHITE);
    }
}

// Helper function to format float values
fn format_float(value: f32) -> heapless::String<8> {
    let mut s = heapless::String::new();
    
    if value >= 100.0 {
        s.push_str(&(value as u32).to_string()).ok();
    } else if value >= 10.0 {
        let whole = value as u32;
        let decimal = ((value - whole as f32) * 10.0) as u32;
        s.push_str(&whole.to_string()).ok();
        s.push('.').ok();
        s.push((b'0' + decimal as u8) as char).ok();
    } else {
        let whole = value as u32;
        let decimal = ((value - whole as f32) * 100.0) as u32;
        s.push_str(&whole.to_string()).ok();
        s.push('.').ok();
        s.push((b'0' + (decimal / 10) as u8) as char).ok();
        s.push((b'0' + (decimal % 10) as u8) as char).ok();
    }
    
    s
}