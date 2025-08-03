"""Comprehensive Web UI tests using Playwright"""

import pytest
from playwright.sync_api import Page, expect
import time
import json
from utils.base_test import ESP32TestBase as BaseTest


@pytest.mark.ui
@pytest.mark.integration
@pytest.mark.slow
class TestWebUIComprehensive(BaseTest):
    """Comprehensive web UI testing"""
    
    @pytest.fixture
    def page_url(self, device_ip):
        """Get the page URL"""
        return f"http://{device_ip}"
        
    def test_page_structure(self, page: Page, page_url):
        """Test complete page structure"""
        page.goto(page_url)
        
        # Wait for page to load
        page.wait_for_load_state("networkidle")
        
        # Check main sections exist
        sections = {
            "header": ["h1", "header", ".header", "#header"],
            "navigation": ["nav", ".nav", ".menu", ".navigation"],
            "main_content": ["main", ".content", "#content", ".main"],
            "status": [".status", "#status", ".device-status"],
            "metrics": [".metrics", "#metrics", ".stats"],
            "settings": [".settings", "#settings", ".config"]
        }
        
        found_sections = {}
        for section, selectors in sections.items():
            for selector in selectors:
                if page.locator(selector).count() > 0:
                    found_sections[section] = selector
                    break
                    
        self.log_info("Found page sections:")
        for section, selector in found_sections.items():
            self.log_info(f"  {section}: {selector}")
            
        # Should have at least header and some content
        assert len(found_sections) >= 2, "Page missing major sections"
        
    def test_real_time_updates(self, page: Page, page_url):
        """Test real-time data updates"""
        page.goto(page_url)
        
        # Find elements that should update
        update_selectors = [
            ".uptime", "#uptime", "[data-field='uptime']",
            ".temperature", "#temperature", "[data-field='temperature']",
            ".cpu-usage", "#cpu-usage", "[data-field='cpu_usage']",
            ".heap-free", "#heap-free", "[data-field='heap_free']",
            ".fps", "#fps", "[data-field='fps_actual']"
        ]
        
        # Collect initial values
        initial_values = {}
        for selector in update_selectors:
            elements = page.locator(selector)
            if elements.count() > 0:
                initial_values[selector] = elements.first.text_content()
                
        if not initial_values:
            self.log_warning("No updatable elements found")
            return
            
        # Wait for updates
        self.log_info("Waiting for real-time updates...")
        page.wait_for_timeout(5000)
        
        # Check for changes
        changes = 0
        for selector, initial_value in initial_values.items():
            elements = page.locator(selector)
            if elements.count() > 0:
                current_value = elements.first.text_content()
                if current_value != initial_value:
                    changes += 1
                    self.log_info(f"  {selector}: '{initial_value}' -> '{current_value}'")
                    
        assert changes > 0, "No real-time updates detected"
        self.log_info(f"Detected {changes} real-time updates")
        
    def test_settings_form_comprehensive(self, page: Page, page_url):
        """Test all settings form functionality"""
        page.goto(page_url)
        
        # Find settings section
        settings_triggers = [
            "text=Settings", "text=Config", "text=Configuration",
            ".settings-button", "#settings-btn", "[aria-label='Settings']"
        ]
        
        clicked = False
        for trigger in settings_triggers:
            if page.locator(trigger).count() > 0:
                page.locator(trigger).first.click()
                clicked = True
                break
                
        if not clicked:
            self.log_warning("Settings trigger not found")
            return
            
        # Wait for settings to appear
        page.wait_for_timeout(1000)
        
        # Test all form inputs
        self.test_brightness_control(page)
        self.test_wifi_settings(page)
        self.test_display_settings(page)
        self.test_ota_settings(page)
        
    def test_brightness_control(self, page: Page):
        """Test brightness control specifically"""
        brightness_inputs = [
            "input[name='brightness']",
            "input#brightness",
            "input[type='range'][data-setting='brightness']",
            ".brightness-slider"
        ]
        
        for selector in brightness_inputs:
            slider = page.locator(selector)
            if slider.count() > 0:
                # Get current value
                current = slider.evaluate("el => el.value")
                self.log_info(f"Current brightness: {current}")
                
                # Change value
                new_value = "30" if current != "30" else "70"
                slider.fill(new_value)
                
                # Trigger change event
                slider.dispatch_event("change")
                
                # Look for save button
                save_btn = page.locator("button:has-text('Save')").or_(
                    page.locator("button[type='submit']")
                )
                
                if save_btn.count() > 0:
                    save_btn.first.click()
                    
                    # Wait for save
                    page.wait_for_timeout(1000)
                    
                    # Check for success message
                    success_selectors = [".success", ".alert-success", ".message-success"]
                    for sel in success_selectors:
                        if page.locator(sel).count() > 0:
                            self.log_info("Settings saved successfully")
                            break
                            
                break
                
    def test_wifi_settings(self, page: Page):
        """Test WiFi configuration UI"""
        wifi_inputs = [
            "input[name='wifi_ssid']",
            "input#wifi-ssid",
            "input[placeholder*='SSID']"
        ]
        
        for selector in wifi_inputs:
            if page.locator(selector).count() > 0:
                ssid_input = page.locator(selector).first
                current_ssid = ssid_input.evaluate("el => el.value")
                self.log_info(f"Current SSID: {current_ssid}")
                
                # Check if password field exists
                password_input = page.locator("input[type='password']")
                if password_input.count() > 0:
                    self.log_info("WiFi password field found")
                    
                # Test scan button if exists
                scan_btn = page.locator("button:has-text('Scan')").or_(
                    page.locator("button[data-action='wifi-scan']")
                )
                
                if scan_btn.count() > 0:
                    scan_btn.first.click()
                    page.wait_for_timeout(3000)
                    
                    # Check for scan results
                    results = page.locator(".wifi-network, .scan-result")
                    if results.count() > 0:
                        self.log_info(f"Found {results.count()} WiFi networks")
                        
                break
                
    def test_display_settings(self, page: Page):
        """Test display-related settings"""
        # Auto-brightness toggle
        auto_brightness = page.locator("input[name='auto_brightness']").or_(
            page.locator("input#auto-brightness")
        )
        
        if auto_brightness.count() > 0:
            is_checked = auto_brightness.is_checked()
            self.log_info(f"Auto-brightness: {'ON' if is_checked else 'OFF'}")
            
            # Toggle it
            auto_brightness.click()
            
        # Theme selector
        theme_selectors = [
            "select[name='theme']",
            "input[name='theme'][type='radio']",
            ".theme-selector"
        ]
        
        for selector in theme_selectors:
            if page.locator(selector).count() > 0:
                self.log_info("Theme selector found")
                break
                
    def test_ota_settings(self, page: Page):
        """Test OTA update settings in UI"""
        ota_elements = [
            "input[name='ota_enabled']",
            "button:has-text('Check for Updates')",
            ".ota-status",
            "#ota-version"
        ]
        
        ota_found = False
        for selector in ota_elements:
            if page.locator(selector).count() > 0:
                ota_found = True
                self.log_info(f"OTA element found: {selector}")
                
        if ota_found:
            # Test check for updates button
            check_btn = page.locator("button:has-text('Check for Updates')")
            if check_btn.count() > 0:
                check_btn.first.click()
                
                # Wait for response
                page.wait_for_timeout(2000)
                
                # Look for status update
                status_elements = page.locator(".ota-status, .update-status")
                if status_elements.count() > 0:
                    status = status_elements.first.text_content()
                    self.log_info(f"OTA status: {status}")
                    
    def test_responsive_breakpoints(self, page: Page, page_url):
        """Test all responsive breakpoints"""
        breakpoints = [
            {"name": "mobile-portrait", "width": 320, "height": 568},
            {"name": "mobile-landscape", "width": 568, "height": 320},
            {"name": "tablet-portrait", "width": 768, "height": 1024},
            {"name": "tablet-landscape", "width": 1024, "height": 768},
            {"name": "desktop", "width": 1280, "height": 720},
            {"name": "wide", "width": 1920, "height": 1080}
        ]
        
        for breakpoint in breakpoints:
            page.set_viewport_size({
                "width": breakpoint["width"],
                "height": breakpoint["height"]
            })
            
            page.goto(page_url)
            page.wait_for_load_state("networkidle")
            
            # Check if content is visible
            body = page.locator("body")
            is_visible = body.is_visible()
            
            # Check for mobile menu
            mobile_menu = page.locator(".mobile-menu, .hamburger, .menu-toggle")
            has_mobile_menu = mobile_menu.count() > 0
            
            self.log_info(f"{breakpoint['name']} ({breakpoint['width']}x{breakpoint['height']}):")
            self.log_info(f"  Content visible: {is_visible}")
            self.log_info(f"  Mobile menu: {has_mobile_menu}")
            
            # Take screenshot for visual verification
            # page.screenshot(path=f"screenshots/{breakpoint['name']}.png")
            
    def test_keyboard_navigation(self, page: Page, page_url):
        """Test keyboard navigation"""
        page.goto(page_url)
        
        # Tab through elements
        tabbable_elements = []
        
        for i in range(20):  # Tab up to 20 times
            page.keyboard.press("Tab")
            focused = page.evaluate("() => document.activeElement.tagName + '#' + document.activeElement.id + '.' + document.activeElement.className")
            tabbable_elements.append(focused)
            
        # Check if we can tab through elements
        unique_elements = set(tabbable_elements)
        self.log_info(f"Found {len(unique_elements)} tabbable elements")
        
        # Test Enter key on buttons
        buttons = page.locator("button")
        if buttons.count() > 0:
            buttons.first.focus()
            page.keyboard.press("Enter")
            page.wait_for_timeout(500)
            
    def test_error_states_ui(self, page: Page, page_url):
        """Test UI error states"""
        page.goto(page_url)
        
        # Disconnect network (simulate)
        page.route("**/api/**", lambda route: route.abort())
        
        # Try to interact with the page
        buttons = page.locator("button")
        if buttons.count() > 0:
            buttons.first.click()
            
        # Wait for error message
        page.wait_for_timeout(2000)
        
        # Check for error indicators
        error_selectors = [
            ".error", ".alert-error", ".error-message",
            ".offline", ".connection-error"
        ]
        
        error_found = False
        for selector in error_selectors:
            if page.locator(selector).count() > 0:
                error_found = True
                error_text = page.locator(selector).first.text_content()
                self.log_info(f"Error state shown: {error_text}")
                break
                
        if not error_found:
            self.log_warning("No error state UI found")
            
    def test_accessibility(self, page: Page, page_url):
        """Test basic accessibility features"""
        page.goto(page_url)
        
        # Check for ARIA labels
        aria_elements = page.locator("[aria-label]")
        self.log_info(f"Elements with ARIA labels: {aria_elements.count()}")
        
        # Check for alt text on images
        images = page.locator("img")
        images_with_alt = page.locator("img[alt]")
        if images.count() > 0:
            self.log_info(f"Images with alt text: {images_with_alt.count()}/{images.count()}")
            
        # Check for form labels
        inputs = page.locator("input, select, textarea")
        labels = page.locator("label")
        self.log_info(f"Form inputs: {inputs.count()}, Labels: {labels.count()}")
        
        # Check heading hierarchy
        h1_count = page.locator("h1").count()
        h2_count = page.locator("h2").count()
        h3_count = page.locator("h3").count()
        
        self.log_info(f"Heading hierarchy: H1={h1_count}, H2={h2_count}, H3={h3_count}")
        
        # Check for skip links
        skip_links = page.locator("a[href^='#']")
        if skip_links.count() > 0:
            self.log_info(f"Skip links found: {skip_links.count()}")
            
    def test_performance_metrics(self, page: Page, page_url):
        """Test page performance metrics"""
        # Navigate and measure
        start_time = time.time()
        response = page.goto(page_url)
        load_time = time.time() - start_time
        
        # Get performance metrics
        metrics = page.evaluate("""() => {
            const perf = window.performance;
            const navigation = perf.getEntriesByType('navigation')[0];
            return {
                domContentLoaded: navigation.domContentLoadedEventEnd - navigation.domContentLoadedEventStart,
                loadComplete: navigation.loadEventEnd - navigation.loadEventStart,
                firstPaint: perf.getEntriesByName('first-paint')[0]?.startTime || 0,
                firstContentfulPaint: perf.getEntriesByName('first-contentful-paint')[0]?.startTime || 0
            };
        }""")
        
        self.log_info("Performance Metrics:")
        self.log_info(f"  Page load time: {load_time:.2f}s")
        self.log_info(f"  DOM Content Loaded: {metrics['domContentLoaded']:.0f}ms")
        self.log_info(f"  Load Complete: {metrics['loadComplete']:.0f}ms")
        self.log_info(f"  First Paint: {metrics['firstPaint']:.0f}ms")
        self.log_info(f"  First Contentful Paint: {metrics['firstContentfulPaint']:.0f}ms")
        
        # Performance assertions
        assert load_time < 5.0, f"Page load too slow: {load_time:.2f}s"