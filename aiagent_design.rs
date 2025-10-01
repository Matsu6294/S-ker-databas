use eframe::egui;
use eframe::egui::{CentralPanel, RichText};
use std::fs;
use std::path::PathBuf;
use std::env;

// Enhanced palette chooser with more Swedish/English color keywords
fn choose_palette(prompt: &str) -> (String, String, String, String) {
    let p = prompt.to_lowercase();
    
    // Dark themes
    if p.contains("dark") || p.contains("noir") || p.contains("mörk") || p.contains("svart") || p.contains("black") {
        return ("20,20,20".into(), "230,230,235".into(), "30,30,30".into(), "24,24,24".into());
    }
    
    // Blue themes
    if p.contains("blå") || p.contains("blue") || p.contains("blått") {
        // Check if dark blue or light blue
        if p.contains("mörk") || p.contains("dark") {
            return ("10,20,40".into(), "220,230,240".into(), "15,25,45".into(), "12,22,42".into());
        } else {
            return ("200,220,240".into(), "10,20,40".into(), "210,225,245".into(), "205,222,242".into());
        }
    }
    
    // Red themes
    if p.contains("röd") || p.contains("red") || p.contains("rött") {
        return ("240,220,220".into(), "80,20,20".into(), "245,225,225".into(), "242,222,222".into());
    }
    
    // Green themes
    if p.contains("grön") || p.contains("green") || p.contains("grönt") {
        return ("220,240,220".into(), "20,60,20".into(), "225,245,225".into(), "222,242,222".into());
    }
    
    // Warm/orange themes
    if p.contains("warm") || p.contains("orange") || p.contains("sun") || p.contains("gul") || p.contains("yellow") {
        return ("255,247,237".into(), "60,40,20".into(), "255,238,210".into(), "255,245,230".into());
    }
    
    // Pastel themes
    if p.contains("pastel") || p.contains("soft") || p.contains("ljus") || p.contains("light") {
        return ("250,250,255".into(), "40,40,60".into(), "245,245,255".into(), "235,250,245".into());
    }
    
    // Neon/cyber themes
    if p.contains("neon") || p.contains("cyber") || p.contains("stark") || p.contains("bright") {
        return ("5,5,10".into(), "200,255,220".into(), "20,20,30".into(), "10,10,20".into());
    }
    
    // Gray themes
    if p.contains("grå") || p.contains("gray") || p.contains("grey") || p.contains("grått") {
        return ("180,180,180".into(), "40,40,40".into(), "190,190,190".into(), "185,185,185".into());
    }
    
    // Default: neutral light theme with good contrast
    ("230,235,240".into(), "20,25,30".into(), "240,242,245".into(), "235,238,242".into())
}

fn parse_rgb(s: &str) -> (u8,u8,u8) {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() == 3 {
        let a = parts[0].trim().parse::<u8>().unwrap_or(0);
        let b = parts[1].trim().parse::<u8>().unwrap_or(0);
        let c = parts[2].trim().parse::<u8>().unwrap_or(0);
        (a,b,c)
    } else { (200,200,200) }
}

fn data_dir_for_exe() -> PathBuf {
    // Prefer the directory containing the running executable (so when the binary
    // lives in /home/matsu/databasen it will read/write files there). Fall back
    // to the current working directory if we can't determine the exe path.
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            return parent.to_path_buf();
        }
    }
    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn generate_desig(prompt: &str) -> Result<PathBuf, std::io::Error> {
    let (bg, text, even, odd) = choose_palette(prompt);
    let bg_t = parse_rgb(&bg);
    let text_t = parse_rgb(&text);
    let even_t = parse_rgb(&even);
    let odd_t = parse_rgb(&odd);

    let dir = data_dir_for_exe();
    let desig_path = dir.join("desig.yaml");
    let bak_path = dir.join("desig.yaml.bak");

    if desig_path.exists() {
        let _ = fs::copy(&desig_path, &bak_path);
    }

    let yaml = format!(
        "bg: [{},{},{}]\ntext: [{},{},{}]\nheading_size: 18.0\nrow_even: [{},{},{}]\nrow_odd: [{},{},{}]\n",
        bg_t.0, bg_t.1, bg_t.2,
        text_t.0, text_t.1, text_t.2,
        even_t.0, even_t.1, even_t.2,
        odd_t.0, odd_t.1, odd_t.2,
    );

    fs::write(&desig_path, yaml)?;
    Ok(desig_path)
}

struct AppState {
    prompt: String,
    status: String,
    // Individual color controls
    bg_r: u8,
    bg_g: u8,
    bg_b: u8,
    text_r: u8,
    text_g: u8,
    text_b: u8,
    row_even_r: u8,
    row_even_g: u8,
    row_even_b: u8,
    row_odd_r: u8,
    row_odd_g: u8,
    row_odd_b: u8,
    heading_size: f32,
    use_prompt: bool,  // Toggle between prompt mode and manual color mode
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            status: "Klar".into(),
            // Default light theme
            bg_r: 230, bg_g: 235, bg_b: 240,
            text_r: 20, text_g: 25, text_b: 30,
            row_even_r: 240, row_even_g: 242, row_even_b: 245,
            row_odd_r: 235, row_odd_g: 238, row_odd_b: 242,
            heading_size: 18.0,
            use_prompt: false,
        }
    }
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.heading("🎨 AI Agent - Designa desig.yaml");
            
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.use_prompt, true, "📝 Prompt-läge");
                ui.radio_value(&mut self.use_prompt, false, "🎨 Färgväljare");
            });
            ui.separator();
            
            if self.use_prompt {
                // Prompt mode
                ui.label("Skriv in en fri design-prompt (t.ex. 'mörk blå', 'ljus grön', 'modern'):");
                ui.text_edit_multiline(&mut self.prompt);
                
                ui.horizontal(|ui| {
                    if ui.button(RichText::new("✨ Generera från prompt").size(14.0)).clicked() {
                        let (bg, text, even, odd) = choose_palette(&self.prompt);
                        let bg_t = parse_rgb(&bg);
                        let text_t = parse_rgb(&text);
                        let even_t = parse_rgb(&even);
                        let odd_t = parse_rgb(&odd);
                        
                        // Update manual controls with generated values
                        self.bg_r = bg_t.0; self.bg_g = bg_t.1; self.bg_b = bg_t.2;
                        self.text_r = text_t.0; self.text_g = text_t.1; self.text_b = text_t.2;
                        self.row_even_r = even_t.0; self.row_even_g = even_t.1; self.row_even_b = even_t.2;
                        self.row_odd_r = odd_t.0; self.row_odd_g = odd_t.1; self.row_odd_b = odd_t.2;
                        
                        self.status = "✓ Färger genererade! Klicka 'Spara' för att tillämpa.".to_string();
                    }
                    
                    if ui.button(RichText::new("💾 SPARA TILL desig.yaml").size(14.0).strong()).clicked() {
                        let dir = data_dir_for_exe();
                        let desig_path = dir.join("desig.yaml");
                        let bak_path = dir.join("desig.yaml.bak");

                        if desig_path.exists() {
                            let _ = fs::copy(&desig_path, &bak_path);
                        }

                        let yaml = format!(
                            "bg: [{},{},{}]\ntext: [{},{},{}]\nheading_size: {}\nrow_even: [{},{},{}]\nrow_odd: [{},{},{}]\n",
                            self.bg_r, self.bg_g, self.bg_b,
                            self.text_r, self.text_g, self.text_b,
                            self.heading_size,
                            self.row_even_r, self.row_even_g, self.row_even_b,
                            self.row_odd_r, self.row_odd_g, self.row_odd_b,
                        );

                        match fs::write(&desig_path, yaml) {
                            Ok(_) => {
                                self.status = format!("✓ desig.yaml sparad: {}", desig_path.display());
                            }
                            Err(e) => {
                                self.status = format!("✗ Misslyckades: {}", e);
                            }
                        }
                    }
                });
            } else {
                // Visual color picker mode with large preview boxes
                ui.horizontal(|ui| {
                    ui.label("Justera färgerna med skjutreglagen →");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(RichText::new("💾 SPARA TILL desig.yaml").size(16.0).strong()).clicked() {
                            let dir = data_dir_for_exe();
                            let desig_path = dir.join("desig.yaml");
                            let bak_path = dir.join("desig.yaml.bak");

                            if desig_path.exists() {
                                let _ = fs::copy(&desig_path, &bak_path);
                            }

                            let yaml = format!(
                                "bg: [{},{},{}]\ntext: [{},{},{}]\nheading_size: {}\nrow_even: [{},{},{}]\nrow_odd: [{},{},{}]\n",
                                self.bg_r, self.bg_g, self.bg_b,
                                self.text_r, self.text_g, self.text_b,
                                self.heading_size,
                                self.row_even_r, self.row_even_g, self.row_even_b,
                                self.row_odd_r, self.row_odd_g, self.row_odd_b,
                            );

                            match fs::write(&desig_path, yaml) {
                                Ok(_) => {
                                    self.status = format!("✓ desig.yaml sparad: {}", desig_path.display());
                                }
                                Err(e) => {
                                    self.status = format!("✗ Misslyckades: {}", e);
                                }
                            }
                        }
                    });
                });
                
                ui.separator();
                
                egui::ScrollArea::vertical().show(ui, |ui| {
                    // Background color
                    ui.group(|ui| {
                        ui.label(RichText::new("🖼️ Bakgrundsfärg").strong().size(16.0));
                        ui.horizontal(|ui| {
                            // Large color preview box
                            let (rect, _response) = ui.allocate_exact_size(
                                egui::vec2(150.0, 80.0),
                                egui::Sense::hover()
                            );
                            ui.painter().rect_filled(
                                rect,
                                4.0,
                                egui::Color32::from_rgb(self.bg_r, self.bg_g, self.bg_b)
                            );
                            ui.painter().rect_stroke(
                                rect,
                                4.0,
                                egui::Stroke::new(2.0, egui::Color32::GRAY)
                            );
                            
                            ui.vertical(|ui| {
                                ui.label("🔴 Röd:");
                                ui.add(egui::Slider::new(&mut self.bg_r, 0..=255).show_value(true));
                                ui.label("🟢 Grön:");
                                ui.add(egui::Slider::new(&mut self.bg_g, 0..=255).show_value(true));
                                ui.label("🔵 Blå:");
                                ui.add(egui::Slider::new(&mut self.bg_b, 0..=255).show_value(true));
                            });
                        });
                        ui.label(format!("RGB: ({}, {}, {})", self.bg_r, self.bg_g, self.bg_b));
                    });
                    
                    ui.add_space(10.0);
                    
                    // Text color
                    ui.group(|ui| {
                        ui.label(RichText::new("📝 Textfärg").strong().size(16.0));
                        ui.horizontal(|ui| {
                            // Large color preview box with sample text
                            let (rect, _response) = ui.allocate_exact_size(
                                egui::vec2(150.0, 80.0),
                                egui::Sense::hover()
                            );
                            ui.painter().rect_filled(
                                rect,
                                4.0,
                                egui::Color32::from_rgb(self.bg_r, self.bg_g, self.bg_b)
                            );
                            ui.painter().text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "Text exempel",
                                egui::FontId::proportional(18.0),
                                egui::Color32::from_rgb(self.text_r, self.text_g, self.text_b)
                            );
                            ui.painter().rect_stroke(
                                rect,
                                4.0,
                                egui::Stroke::new(2.0, egui::Color32::GRAY)
                            );
                            
                            ui.vertical(|ui| {
                                ui.label("🔴 Röd:");
                                ui.add(egui::Slider::new(&mut self.text_r, 0..=255).show_value(true));
                                ui.label("🟢 Grön:");
                                ui.add(egui::Slider::new(&mut self.text_g, 0..=255).show_value(true));
                                ui.label("🔵 Blå:");
                                ui.add(egui::Slider::new(&mut self.text_b, 0..=255).show_value(true));
                            });
                        });
                        ui.label(format!("RGB: ({}, {}, {})", self.text_r, self.text_g, self.text_b));
                    });
                    
                    ui.add_space(10.0);
                    
                    // Row colors side by side
                    ui.group(|ui| {
                        ui.label(RichText::new("📊 Radfärger (zebra-mönster)").strong().size(16.0));
                        ui.horizontal(|ui| {
                            // Even row
                            ui.vertical(|ui| {
                                ui.label("Jämna rader:");
                                let (rect, _response) = ui.allocate_exact_size(
                                    egui::vec2(120.0, 60.0),
                                    egui::Sense::hover()
                                );
                                ui.painter().rect_filled(
                                    rect,
                                    4.0,
                                    egui::Color32::from_rgb(self.row_even_r, self.row_even_g, self.row_even_b)
                                );
                                ui.painter().rect_stroke(
                                    rect,
                                    4.0,
                                    egui::Stroke::new(2.0, egui::Color32::GRAY)
                                );
                                ui.label("🔴"); ui.add(egui::Slider::new(&mut self.row_even_r, 0..=255).show_value(true));
                                ui.label("🟢"); ui.add(egui::Slider::new(&mut self.row_even_g, 0..=255).show_value(true));
                                ui.label("🔵"); ui.add(egui::Slider::new(&mut self.row_even_b, 0..=255).show_value(true));
                                ui.label(format!("RGB: ({},{},{})", self.row_even_r, self.row_even_g, self.row_even_b));
                            });
                            
                            ui.separator();
                            
                            // Odd row
                            ui.vertical(|ui| {
                                ui.label("Udda rader:");
                                let (rect, _response) = ui.allocate_exact_size(
                                    egui::vec2(120.0, 60.0),
                                    egui::Sense::hover()
                                );
                                ui.painter().rect_filled(
                                    rect,
                                    4.0,
                                    egui::Color32::from_rgb(self.row_odd_r, self.row_odd_g, self.row_odd_b)
                                );
                                ui.painter().rect_stroke(
                                    rect,
                                    4.0,
                                    egui::Stroke::new(2.0, egui::Color32::GRAY)
                                );
                                ui.label("🔴"); ui.add(egui::Slider::new(&mut self.row_odd_r, 0..=255).show_value(true));
                                ui.label("🟢"); ui.add(egui::Slider::new(&mut self.row_odd_g, 0..=255).show_value(true));
                                ui.label("🔵"); ui.add(egui::Slider::new(&mut self.row_odd_b, 0..=255).show_value(true));
                                ui.label(format!("RGB: ({},{},{})", self.row_odd_r, self.row_odd_g, self.row_odd_b));
                            });
                        });
                    });
                    
                    ui.add_space(10.0);
                    
                    // Heading size
                    ui.group(|ui| {
                        ui.label(RichText::new("📐 Rubrikstorlek").strong().size(16.0));
                        ui.add(egui::Slider::new(&mut self.heading_size, 10.0..=32.0)
                            .text("pixels")
                            .show_value(true));
                        ui.label(
                            RichText::new("Exempel rubrik")
                                .size(self.heading_size)
                                .strong()
                        );
                    });
                });
            }
            
            // Status message at bottom (always visible)
            ui.separator();
            if !self.status.is_empty() {
                ui.label(RichText::new(&self.status).size(13.0).italics());
            }
        });
    }
}

fn main() {
    // simple CLI: --generate "prompt" to run the generator non-interactively
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 3 && args[1] == "--generate" {
        let prompt = &args[2];
        match generate_desig(prompt) {
            Ok(p) => {
                println!("Wrote desig.yaml: {}", p.display());
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("Failed to write desig.yaml: {}", e);
                std::process::exit(2);
            }
        }
    }

    let native_options = eframe::NativeOptions::default();
    let _ = eframe::run_native("AI Agent - Design generator", native_options, Box::new(|_cc| {
        Box::new(AppState::default())
    }));
}
