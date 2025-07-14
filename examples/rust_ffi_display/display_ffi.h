// display_ffi.h - C interface for Rust FFI bindings
#ifndef DISPLAY_FFI_H
#define DISPLAY_FFI_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Display initialization
void display_init(void);
void display_set_brightness(uint8_t brightness);
void display_clear(uint16_t color);

// Basic drawing functions
void display_draw_pixel(uint16_t x, uint16_t y, uint16_t color);
void display_fill_rect(uint16_t x, uint16_t y, uint16_t w, uint16_t h, uint16_t color);
void display_draw_line(uint16_t x0, uint16_t y0, uint16_t x1, uint16_t y1, uint16_t color);

// Text rendering
void display_draw_string(uint16_t x, uint16_t y, const char* text, uint16_t color);
void display_draw_string_transparent(uint16_t x, uint16_t y, const char* text, uint16_t color);

// Optimized operations
void display_update(void);
void display_flush(void);

// Color constants (BGR format for your display)
#define COLOR_BLACK   0xFFFF
#define COLOR_WHITE   0x0000
#define COLOR_RED     0x07FF
#define COLOR_GREEN   0xF81F
#define COLOR_BLUE    0xF8E0
#define COLOR_YELLOW  0x001F
#define COLOR_CYAN    0xF800
#define COLOR_MAGENTA 0x07E0

#ifdef __cplusplus
}
#endif

#endif // DISPLAY_FFI_H