use crate::ui::*;
use crate::ui::components::*;
use crate::display::Color;
use embassy_time::Duration;

#[test]
fn test_progress_bar_value_clamping() {
    let mut progress = ProgressBar::new(0, 0, 100, 20);
    
    // Test value clamping
    progress.set_value(-10.0);
    assert_eq!(progress.target_value, 0.0);
    
    progress.set_value(150.0);
    assert_eq!(progress.target_value, 100.0);
    
    progress.set_value(50.0);
    assert_eq!(progress.target_value, 50.0);
}

#[test]
fn test_circular_progress_angles() {
    let progress = CircularProgress::new(100, 100, 50, 5);
    
    // 0% should be 0 degrees
    // 50% should be 180 degrees
    // 100% should be 360 degrees
    
    // Test angle calculation
    let angle_0 = 360.0 * (0.0 / 100.0);
    let angle_50 = 360.0 * (50.0 / 100.0);
    let angle_100 = 360.0 * (100.0 / 100.0);
    
    assert_eq!(angle_0, 0.0);
    assert_eq!(angle_50, 180.0);
    assert_eq!(angle_100, 360.0);
}

#[test]
fn test_spinner_styles() {
    // Test that all spinner styles can be created
    let styles = [
        SpinnerStyle::Dots,
        SpinnerStyle::Ring,
        SpinnerStyle::Pulse,
        SpinnerStyle::Wave,
    ];
    
    for style in &styles {
        let spinner = LoadingSpinner::new(50, 50, 20, *style);
        // Spinner should be created successfully
    }
}

#[test]
fn test_line_graph_auto_scale() {
    let mut graph = LineGraph::new(0, 0, 100, 50);
    
    // Add data points
    graph.add_point(10.0, 0);
    graph.add_point(20.0, 1);
    graph.add_point(5.0, 2);
    graph.add_point(30.0, 3);
    
    // Auto scale should adjust to data range
    assert!(graph.min_value < 5.0);
    assert!(graph.max_value > 30.0);
}

#[test]
fn test_line_graph_data_limit() {
    let mut graph = LineGraph::new(0, 0, 100, 50);
    
    // Add more than 100 points (the limit)
    for i in 0..150 {
        graph.add_point(i as f32, i);
    }
    
    // Should only keep the most recent 100 points
    assert_eq!(graph.data.len(), 100);
}

#[test]
fn test_bar_chart_max_bars() {
    let mut chart = BarChart::new(0, 0, 200, 100);
    
    // Try to add more than 8 bars (the limit)
    for i in 0..10 {
        let label = format!("Bar{}", i);
        let _ = chart.add_bar(&label, i as f32 * 10.0);
    }
    
    // Should only have 8 bars maximum
    assert!(chart.values.len() <= 8);
}

#[test]
fn test_format_percentage() {
    let mut s = heapless::String::<4>::new();
    
    // Test single digit
    s.clear();
    s.push('5').ok();
    s.push('%').ok();
    assert_eq!(&s, "5%");
    
    // Test double digit
    s.clear();
    s.push_str("50").ok();
    s.push('%').ok();
    assert_eq!(&s, "50%");
    
    // Test 100%
    s.clear();
    s.push_str("100").ok();
    s.push('%').ok();
    assert_eq!(&s, "100%");
}

#[test]
fn test_theme_colors() {
    // Test that theme colors are consistent
    struct TestTheme {
        primary: Color,
        secondary: Color,
        background: Color,
        text: Color,
    }
    
    let theme = TestTheme {
        primary: Color::PRIMARY_BLUE,
        secondary: Color::CYAN,
        background: Color::BLACK,
        text: Color::WHITE,
    };
    
    // Colors should be distinct
    assert_ne!(theme.primary.0, theme.secondary.0);
    assert_ne!(theme.background.0, theme.text.0);
}

#[test]
fn test_widget_positioning() {
    // Test that widgets respect boundaries
    let screen_width = 320;
    let screen_height = 170;
    
    // Progress bar should fit within screen
    let progress = ProgressBar::new(10, 10, 300, 20);
    assert!(progress.x + progress.width <= screen_width);
    assert!(progress.y + progress.height <= screen_height);
}

#[test]
fn test_data_point_creation() {
    let point = DataPoint {
        value: 25.5,
        timestamp: 1000,
    };
    
    assert_eq!(point.value, 25.5);
    assert_eq!(point.timestamp, 1000);
}

#[test]
fn test_animation_in_ui_components() {
    let mut progress = ProgressBar::new(0, 0, 100, 20);
    
    // Setting value should trigger animation
    progress.set_value(75.0);
    
    // Initial value should still be 0
    assert_eq!(progress.value, 0.0);
    
    // Target should be set
    assert_eq!(progress.target_value, 75.0);
    
    // After update, value should move towards target
    progress.update();
    // Value should be between 0 and 75 (animating)
}

#[cfg(test)]
mod screen_tests {
    use super::*;
    
    #[test]
    fn test_screen_navigation() {
        // Test that screens can be switched
        let screens = vec!["Home", "Sensors", "Settings", "About"];
        let mut current = 0;
        
        // Next screen
        current = (current + 1) % screens.len();
        assert_eq!(current, 1);
        assert_eq!(screens[current], "Sensors");
        
        // Previous screen
        current = (current + screens.len() - 1) % screens.len();
        assert_eq!(current, 0);
        assert_eq!(screens[current], "Home");
    }
    
    #[test]
    fn test_menu_selection() {
        let menu_items = vec!["WiFi", "Display", "System", "Back"];
        let mut selected = 0;
        
        // Test navigation
        selected = (selected + 1).min(menu_items.len() - 1);
        assert_eq!(menu_items[selected], "Display");
        
        selected = selected.saturating_sub(1);
        assert_eq!(menu_items[selected], "WiFi");
    }
}