// Entry point for the bundled binary. Everything lives in the library crate
// (`zoom_slides_picker_lib`) so it stays unit-testable; this file only forwards to it.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    zoom_slides_picker_lib::run();
}
