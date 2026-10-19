#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

use eframe::egui;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{channel, Receiver};
use std::thread;

const PRESETS: [(&str, &str); 5] = [
    ("screen", "Smallest size, low quality (72 dpi)"),
    ("ebook", "Good balance (150 dpi)"),
    ("printer", "High quality (300 dpi)"),
    ("prepress", "Highest quality, color preserving"),
    ("default", "Ghostscript default"),
];

struct Stats {
    output: PathBuf,
    original: u64,
    compressed: u64,
}

enum Status {
    Idle,
    Running,
    Done(Stats),
    Failed(String),
}

struct App {
    input: Option<PathBuf>,
    custom_output: Option<PathBuf>,
    preset: usize,
    force: bool,
    status: Status,
    rx: Option<Receiver<Result<Stats, String>>>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            input: None,
            custom_output: None,
            preset: 1, // ebook
            force: false,
            status: Status::Idle,
            rx: None,
        }
    }
}

fn default_output(input: &Path) -> PathBuf {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document");
    input.with_file_name(format!("{stem}_compressed.pdf"))
}

fn is_pdf(p: &Path) -> bool {
    p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
}

#[cfg(windows)]
fn find_gs() -> PathBuf {
    // The Ghostscript installer does not add itself to PATH, so look in the default folders.
    for root in ["C:\\Program Files\\gs", "C:\\Program Files (x86)\\gs"] {
        if let Ok(rd) = fs::read_dir(root) {
            let mut dirs: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
            dirs.sort();
            for d in dirs.into_iter().rev() {
                for exe in ["gswin64c.exe", "gswin32c.exe"] {
                    let p = d.join("bin").join(exe);
                    if p.is_file() {
                        return p;
                    }
                }
            }
        }
    }
    PathBuf::from("gswin64c") // fall back to PATH
}

fn gs_command() -> Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let mut c = Command::new(find_gs());
        c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: no console flash
        c
    }
    #[cfg(not(windows))]
    {
        Command::new("gs")
    }
}

fn compress(input: PathBuf, output: PathBuf, preset: &str, force: bool) -> Result<Stats, String> {
    if !is_pdf(&input) {
        return Err("Input must be an existing PDF file".into());
    }
    let input = fs::canonicalize(&input).map_err(|e| format!("Cannot access input: {e}"))?;

    if output.exists() && !force {
        return Err("Output already exists. Pick another name or tick “Overwrite”.".into());
    }
    if fs::canonicalize(&output).ok().as_ref() == Some(&input) {
        return Err("Input and output must be different files".into());
    }

    let status = gs_command()
        .arg("-sDEVICE=pdfwrite")
        .arg("-dCompatibilityLevel=1.4")
        .arg(format!("-dPDFSETTINGS=/{preset}"))
        .arg("-dNOPAUSE")
        .arg("-dQUIET")
        .arg("-dBATCH")
        .arg(format!("-sOutputFile={}", output.display()))
        .arg(&input)
        .status()
        .map_err(|e| format!("Could not start Ghostscript (is `gs` installed?): {e}"))?;

    if !status.success() {
        let _ = fs::remove_file(&output);
        return Err("Ghostscript compression failed".into());
    }

    let original = fs::metadata(&input).map(|m| m.len()).unwrap_or(0);
    let compressed = match fs::metadata(&output) {
        Ok(m) if m.len() > 0 => m.len(),
        _ => return Err("Output file is missing or empty".into()),
    };

    Ok(Stats {
        output,
        original,
        compressed,
    })
}

impl App {
    fn set_input(&mut self, p: PathBuf) {
        self.input = Some(p);
        self.custom_output = None;
        self.status = Status::Idle;
    }

    fn effective_output(&self) -> Option<PathBuf> {
        self.custom_output
            .clone()
            .or_else(|| self.input.as_deref().map(default_output))
    }

    fn start(&mut self, ctx: &egui::Context) {
        let (Some(input), Some(output)) = (self.input.clone(), self.effective_output()) else {
            return;
        };
        let preset = PRESETS[self.preset].0;
        let force = self.force;
        let (tx, rx) = channel();
        let ctx = ctx.clone();
        thread::spawn(move || {
            let _ = tx.send(compress(input, output, preset, force));
            ctx.request_repaint();
        });
        self.rx = Some(rx);
        self.status = Status::Running;
    }

    fn poll(&mut self) {
        if let Some(rx) = &self.rx {
            if let Ok(res) = rx.try_recv() {
                self.status = match res {
                    Ok(s) => Status::Done(s),
                    Err(e) => Status::Failed(e),
                };
                self.rx = None;
            }
        }
    }
}

fn mb(bytes: u64) -> f64 {
    bytes as f64 / 1_000_000.0
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll();

        // Drag & drop
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if !matches!(self.status, Status::Running) {
            if let Some(path) = dropped.into_iter().filter_map(|f| f.path).find(|p| is_pdf(p)) {
                self.set_input(path);
            }
        }

        let running = matches!(self.status, Status::Running);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("PDF Compressor");
            ui.add_space(8.0);

            // Input
            ui.horizontal(|ui| {
                ui.add_enabled_ui(!running, |ui| {
                    if ui.button("Choose PDF…").clicked() {
                        if let Some(p) = rfd::FileDialog::new()
                            .add_filter("PDF", &["pdf"])
                            .pick_file()
                        {
                            self.set_input(p);
                        }
                    }
                });
                match &self.input {
                    Some(p) => ui.label(p.display().to_string()),
                    None => ui.weak("…or drop a PDF onto this window"),
                };
            });

            ui.add_space(8.0);

            // Output
            if let Some(out) = self.effective_output() {
                ui.horizontal(|ui| {
                    ui.add_enabled_ui(!running, |ui| {
                        if ui.button("Save as…").clicked() {
                            let mut dlg = rfd::FileDialog::new().add_filter("PDF", &["pdf"]);
                            if let Some(name) = out.file_name().and_then(|n| n.to_str()) {
                                dlg = dlg.set_file_name(name);
                            }
                            if let Some(dir) = out.parent() {
                                dlg = dlg.set_directory(dir);
                            }
                            if let Some(p) = dlg.save_file() {
                                self.custom_output = Some(p);
                                // The save dialog already confirmed overwrite
                                self.force = true;
                            }
                        }
                    });
                    ui.label(out.display().to_string());
                });
                ui.add_space(8.0);
            }

            // Preset
            ui.horizontal(|ui| {
                ui.label("Quality:");
                ui.add_enabled_ui(!running, |ui| {
                    egui::ComboBox::from_id_salt("preset")
                        .selected_text(PRESETS[self.preset].0)
                        .show_ui(ui, |ui| {
                            for (i, (name, _)) in PRESETS.iter().enumerate() {
                                ui.selectable_value(&mut self.preset, i, *name);
                            }
                        });
                });
                ui.weak(PRESETS[self.preset].1);
            });

            ui.add_space(4.0);
            ui.add_enabled(!running, egui::Checkbox::new(&mut self.force, "Overwrite output if it exists"));

            ui.add_space(12.0);

            let can_run = self.input.is_some() && !running;
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(can_run, egui::Button::new("Compress").min_size(egui::vec2(120.0, 30.0)))
                    .clicked()
                {
                    self.start(ctx);
                }
                if running {
                    ui.spinner();
                    ui.label("Compressing…");
                }
            });

            ui.add_space(12.0);
            ui.separator();

            // Result
            match &self.status {
                Status::Idle | Status::Running => {}
                Status::Failed(e) => {
                    ui.colored_label(egui::Color32::LIGHT_RED, e);
                }
                Status::Done(s) => {
                    ui.label(format!("Output: {}", s.output.display()));
                    ui.label(format!("Original:   {:.2} MB", mb(s.original)));
                    ui.label(format!("Compressed: {:.2} MB", mb(s.compressed)));
                    if s.compressed < s.original {
                        let saved = 100.0 * (s.original - s.compressed) as f64 / s.original.max(1) as f64;
                        ui.colored_label(egui::Color32::LIGHT_GREEN, format!("Saved: {saved:.1}%"));
                    } else {
                        ui.colored_label(
                            egui::Color32::YELLOW,
                            "Output is not smaller than the original. Try a lower quality preset.",
                        );
                    }
                }
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([560.0, 340.0])
            .with_min_inner_size([460.0, 300.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "PDF Compressor",
        options,
        Box::new(|_cc| Ok(Box::new(App::default()))),
    )
}
