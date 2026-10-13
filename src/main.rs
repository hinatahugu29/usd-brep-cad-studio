mod app;
mod browser;
mod camera;
mod dialogs;
mod edge_feature_dialog;
mod edge_picker;
mod gizmo;
mod kernel;
mod measure;
mod params;
mod primitive_bounds;
mod renderer;
mod section_analysis;
mod shortcut_box;
mod sketch;
mod sketch_palette;
mod theme;
mod timeline;
mod toolbar;
mod viewcube;

use app::CadApp;
use eframe::egui::Vec2;

fn setup_dll_paths() {
    #[cfg(windows)]
    {
        unsafe extern "system" {
            fn SetDllDirectoryA(lpPathName: *const u8) -> i32;
        }
        let path = b"C:\\Users\\T03000\\AppData\\Local\\Programs\\Python\\Python312\0";
        unsafe {
            SetDllDirectoryA(path.as_ptr());
        }
    }
}

fn main() -> eframe::Result<()> {
    setup_dll_paths();
    kernel::ensure_python_init();

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("USD-BRep CAD Studio")
            .with_inner_size(Vec2::new(1280.0, 800.0))
            .with_min_inner_size(Vec2::new(800.0, 500.0)),
        multisampling: 4,
        depth_buffer: 24,
        ..Default::default()
    };

    eframe::run_native(
        "USD-BRep CAD Studio",
        native_options,
        Box::new(|cc| Ok(Box::new(CadApp::new(cc)))),
    )
}
