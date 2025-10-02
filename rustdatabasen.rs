// Stand-alone Rust egui app för kryptering och visning av personer-data
// Läser design från desig.yaml

use eframe::egui;
use eframe::egui::{ScrollArea, RichText, Vec2};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::{Argon2, Algorithm, Params, Version};
use base64::{engine::general_purpose, Engine as _};
use rand::RngCore;

mod desig;
use desig::{load_theme_from_yaml, apply_theme, setup_fonts};

const PERSONER: &str = "/home/matsu/databasen/personer";
const PERSONER2: &str = "/home/matsu/databasen/personer2";

// Derive a 32-byte key using Argon2id with stronger parameters
fn derive_key(password: &str, salt: &[u8]) -> [u8; 32] {
    // Increased from 64MB to 128MB memory, 3 to 4 iterations
    // This makes brute-forcing ~2x slower
    let params = Params::new(131072, 4, 1, None).expect("argon2 params");
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; 32];
    argon2.hash_password_into(password.as_bytes(), salt, &mut key).expect("argon2 derive");
    key
}

// Encrypt and write personer to personer2
fn encrypt_and_save(identifier: &str, password: &str) -> Result<(), String> {
    if !Path::new(PERSONER).exists() {
        return Err("Filen 'personer' saknas".to_string());
    }
    
    let plaintext = fs::read_to_string(PERSONER)
        .map_err(|e| format!("Kunde inte läsa personer: {}", e))?;
    
    // Generate salt and nonce
    let mut salt = [0u8; 16];
    let mut nonce_bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut salt);
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    
    // Derive key
    let key = derive_key(password, &salt);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| format!("Cipher error: {}", e))?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    // Create data to encrypt: PASSWORD_HASH|actual_data
    // This allows us to verify the password is correct when decrypting
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    password.hash(&mut hasher);
    let pwd_hash = hasher.finish();
    let data_with_pwd = format!("PWD:{}|{}", pwd_hash, plaintext);
    
    // Encrypt
    let ciphertext = cipher.encrypt(nonce, data_with_pwd.as_bytes())
        .map_err(|e| format!("Encryption failed: {}", e))?;
    
    // Format: identifier|base64(salt)|base64(nonce)|base64(ciphertext)|attempts|last_fail|lockout
    // Brute-force protection fields: attempts=0, last_fail=0, lockout=0 (no lock)
    let new_line = format!("{}|{}|{}|{}|0|0|0",
        identifier,
        general_purpose::STANDARD.encode(&salt),
        general_purpose::STANDARD.encode(&nonce_bytes),
        general_purpose::STANDARD.encode(&ciphertext)
    );
    
    // Read existing content if file exists
    let mut lines: Vec<String> = Vec::new();
    let mut preserve_bf_data = false;
    let mut old_bf_fields = String::from("0|0|0");
    
    if Path::new(PERSONER2).exists() {
        let existing = fs::read_to_string(PERSONER2)
            .map_err(|e| format!("Kunne inte läsa personer2: {}", e))?;
        
        // Keep all lines that don't match this identifier
        for line in existing.lines() {
            if !line.is_empty() {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() >= 1 {
                    if parts[0] == identifier && parts.len() >= 7 {
                        // Preserve brute-force data when re-encrypting
                        old_bf_fields = format!("{}|{}|{}", parts[4], parts[5], parts[6]);
                        preserve_bf_data = true;
                    } else if parts[0] != identifier {
                        lines.push(line.to_string());
                    }
                }
            }
        }
    }
    
    // Add the new entry (preserve BF data if re-encrypting)
    let final_line = if preserve_bf_data {
        format!("{}|{}|{}|{}|{}",
            identifier,
            general_purpose::STANDARD.encode(&salt),
            general_purpose::STANDARD.encode(&nonce_bytes),
            general_purpose::STANDARD.encode(&ciphertext),
            old_bf_fields
        )
    } else {
        new_line
    };
    
    lines.push(final_line);
    
    // Write all lines back
    let content = lines.join("\n") + "\n";
    fs::write(PERSONER2, content)
        .map_err(|e| format!("Kunde inte skriva personer2: {}", e))?;
    
    Ok(())
}

// Record a failed login attempt for brute-force protection
// Returns remaining attempts (0 if locked out)
fn record_failed_attempt(identifier: &str) -> Result<u32, String> {
    if !Path::new(PERSONER2).exists() {
        return Ok(5); // Default: 5 attempts remaining
    }
    
    let content = fs::read_to_string(PERSONER2)
        .map_err(|e| format!("Kunde inte läsa personer2: {}", e))?;
    
    let mut lines: Vec<String> = Vec::new();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let mut remaining_attempts = 5;
    
    for line in content.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 7 && parts[0] == identifier {
            let attempts: u32 = parts[4].parse().unwrap_or(0) + 1;
            remaining_attempts = if attempts >= 5 { 0 } else { 5 - attempts };
            
            let lockout = if attempts >= 5 {
                now + 900 // Lock for 15 minutes (900 seconds)
            } else {
                0
            };
            
            // Update line with new attempt count
            let updated = format!("{}|{}|{}|{}|{}|{}|{}",
                parts[0], parts[1], parts[2], parts[3],
                attempts, now, lockout
            );
            lines.push(updated);
        } else {
            lines.push(line.to_string());
        }
    }
    
    fs::write(PERSONER2, lines.join("\n") + "\n")
        .map_err(|e| format!("Kunde inte uppdatera personer2: {}", e))?;
    
    Ok(remaining_attempts)
}

// Reset failed attempts after successful login
fn reset_failed_attempts(identifier: &str) -> Result<(), String> {
    if !Path::new(PERSONER2).exists() {
        return Ok(());
    }
    
    let content = fs::read_to_string(PERSONER2)
        .map_err(|e| format!("Kunde inte läsa personer2: {}", e))?;
    
    let mut lines: Vec<String> = Vec::new();
    
    for line in content.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 7 && parts[0] == identifier {
            // Reset counters: attempts=0, last_fail=0, lockout=0
            let updated = format!("{}|{}|{}|{}|0|0|0",
                parts[0], parts[1], parts[2], parts[3]
            );
            lines.push(updated);
        } else {
            lines.push(line.to_string());
        }
    }
    
    fs::write(PERSONER2, lines.join("\n") + "\n")
        .map_err(|e| format!("Kunde inte uppdatera personer2: {}", e))?;
    
    Ok(())
}

// Decrypt and return lines from personer2
fn decrypt_and_load(identifier: &str, password: &str) -> Result<Vec<Vec<String>>, String> {
    if !Path::new(PERSONER2).exists() {
        return Err("Filen 'personer2' saknas".to_string());
    }
    
    let content = fs::read_to_string(PERSONER2)
        .map_err(|e| format!("Kunde inte läsa personer2: {}", e))?;
    
    for line in content.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        // Expected format: ID|salt|nonce|cipher|attempts|last_fail|lockout
        if parts.len() < 4 {
            continue;
        }
        
        if parts[0] != identifier {
            continue;
        }
        
        // Check brute-force protection
        if parts.len() >= 7 {
            let attempts: u32 = parts[4].parse().unwrap_or(0);
            let lockout: u64 = parts[6].parse().unwrap_or(0);
            
            // Check if locked out
            if lockout > 0 {
                let now = SystemTime::now().duration_since(UNIX_EPOCH)
                    .unwrap().as_secs();
                if now < lockout {
                    let remaining_time = lockout - now;
                    let mins = remaining_time / 60;
                    let secs = remaining_time % 60;
                    return Err(format!("🔒 Kontot är låst i {} min {} sek (för många misslyckade försök)", mins, secs));
                } else {
                    // Lockout expired, reset counters
                    let _ = reset_failed_attempts(identifier);
                }
            }
            
            // Check attempt limit (before lockout)
            if attempts >= 5 {
                // This shouldn't happen if lockout is working, but just in case
                return Err("🔒 För många misslyckade försök. Försök igen senare.".to_string());
            }
        }
        
        // Decode salt, nonce, ciphertext
        let salt = general_purpose::STANDARD.decode(parts[1])
            .map_err(|_| "Fel vid avkodning av salt")?;
        let nonce_bytes = general_purpose::STANDARD.decode(parts[2])
            .map_err(|_| "Fel vid avkodning av nonce")?;
        let ciphertext = general_purpose::STANDARD.decode(parts[3])
            .map_err(|_| "Fel vid avkodning av ciphertext")?;
        
        // Derive key
        let key = derive_key(password, &salt);
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| format!("Cipher error: {}", e))?;
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Try to decrypt
        let decryption_result = cipher.decrypt(nonce, ciphertext.as_ref());
        
        // If decryption fails, it's likely wrong password
        if decryption_result.is_err() {
            // WRONG PASSWORD - Update brute-force counter
            let remaining = record_failed_attempt(identifier)?;
            if remaining == 0 {
                return Err("❌ Fel lösenord! Kontot är nu låst i 15 minuter.".to_string());
            } else {
                return Err(format!("❌ Fel lösenord! {} försök kvar.", remaining));
            }
        }
        
        let plaintext_bytes = decryption_result.unwrap();
        let decrypted_data = String::from_utf8(plaintext_bytes)
            .map_err(|_| "Fel vid konvertering av dekrypterad data")?;
        
        // Verify password hash
        // Expected format: PWD:hash|actual_data
        if !decrypted_data.starts_with("PWD:") {
            return Err("Felaktigt dataformat - gammal kryptering?".to_string());
        }
        
        let parts_pwd: Vec<&str> = decrypted_data.splitn(2, '|').collect();
        if parts_pwd.len() != 2 {
            return Err("Felaktigt dataformat".to_string());
        }
        
        // Extract stored password hash
        let stored_hash_str = parts_pwd[0].trim_start_matches("PWD:");
        let stored_hash: u64 = stored_hash_str.parse()
            .map_err(|_| "Kunde inte läsa lösenordshash")?;
        
        // Calculate hash of provided password
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        password.hash(&mut hasher);
        let provided_hash = hasher.finish();
        
        // Verify password matches (double-check, should match if decryption succeeded)
        if stored_hash != provided_hash {
            // WRONG PASSWORD - Update brute-force counter
            let remaining = record_failed_attempt(identifier)?;
            if remaining == 0 {
                return Err("❌ Fel lösenord! Kontot är nu låst i 15 minuter.".to_string());
            } else {
                return Err(format!("❌ Fel lösenord! {} försök kvar.", remaining));
            }
        }
        
        // Password is CORRECT! Reset brute-force counter
        reset_failed_attempts(identifier)?;
        
        // Extract actual data
        let plaintext = parts_pwd[1];
        
        // Parse lines into rows
        let mut rows = Vec::new();
        for data_line in plaintext.lines() {
            let trimmed = data_line.trim();
            if trimmed.is_empty() {
                continue;
            }
            // Split on whitespace
            let fields: Vec<String> = trimmed.split_whitespace()
                .map(|s| s.to_string())
                .collect();
            rows.push(fields);
        }
        
        return Ok(rows);
    }
    
    Err("Ingen post hittades för denna identifierare".to_string())
}

struct RustDatabasenApp {
    identifier: String,
    password: String,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    status: String,
    sort_column: Option<usize>,
    sort_ascending: bool,
}

impl Default for RustDatabasenApp {
    fn default() -> Self {
        Self {
            identifier: String::new(),
            password: String::new(),
            headers: Vec::new(),
            rows: Vec::new(),
            status: "Redo".to_string(),
            sort_column: None,
            sort_ascending: true,
        }
    }
}

impl RustDatabasenApp {
    fn encrypt_copy(&mut self) {
        if self.identifier.is_empty() || self.password.is_empty() {
            self.status = "Fyll i både ID och lösenord".to_string();
            return;
        }
        
        match encrypt_and_save(&self.identifier, &self.password) {
            Ok(_) => {
                self.status = format!("Krypterat och sparat för ID: {}", self.identifier);
            }
            Err(e) => {
                self.status = format!("Fel: {}", e);
            }
        }
    }
    
    fn decrypt_show(&mut self) {
        if self.identifier.is_empty() || self.password.is_empty() {
            self.status = "Fyll i både ID och lösenord".to_string();
            return;
        }
        
        match decrypt_and_load(&self.identifier, &self.password) {
            Ok(rows) => {
                if rows.is_empty() {
                    self.status = "Inga rader hittades".to_string();
                    self.rows = Vec::new();
                    self.headers = Vec::new();
                } else {
                    // Find max column count
                    let max_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
                    
                    // Generate generic column names: Kolumn1, Kolumn2, ...
                    self.headers = (1..=max_cols)
                        .map(|i| format!("Kolumn{}", i))
                        .collect();
                    
                    // Normalize rows to match header count
                    let mut normalized = rows;
                    for row in &mut normalized {
                        if row.len() < max_cols {
                            row.resize(max_cols, String::new());
                        }
                    }
                    
                    self.rows = normalized;
                    self.status = format!("Visar {} rader", self.rows.len());
                }
            }
            Err(e) => {
                self.status = format!("Fel: {}", e);
                self.rows = Vec::new();
                self.headers = Vec::new();
            }
        }
    }
    
    fn sort_by_column(&mut self, col: usize) {
        if col >= self.headers.len() {
            return;
        }
        
        // Toggle sort direction if clicking same column
        if self.sort_column == Some(col) {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_column = Some(col);
            self.sort_ascending = true;
        }
        
        let ascending = self.sort_ascending;
        self.rows.sort_by(|a, b| {
            let val_a = a.get(col).map(|s| s.to_lowercase()).unwrap_or_default();
            let val_b = b.get(col).map(|s| s.to_lowercase()).unwrap_or_default();
            if ascending {
                val_a.cmp(&val_b)
            } else {
                val_b.cmp(&val_a)
            }
        });
    }
}

impl eframe::App for RustDatabasenApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("ID:");
                ui.add(egui::TextEdit::singleline(&mut self.identifier).desired_width(120.0));
                ui.label("Lösenord:");
                ui.add(egui::TextEdit::singleline(&mut self.password)
                    .password(true)
                    .desired_width(120.0));
                
                if ui.button("Kryptera & Kopiera").clicked() {
                    self.encrypt_copy();
                }
                
                if ui.button("Visa").clicked() {
                    self.decrypt_show();
                }
            });
            
            ui.label(RichText::new(&self.status).size(12.0));
        });
        
        egui::CentralPanel::default().show(ctx, |ui| {
            if !self.rows.is_empty() {
                ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("data_table")
                        .striped(true)
                        .min_col_width(120.0)
                        .show(ui, |ui| {
                            // Header row with clickable columns
                            let mut clicked_col = None;
                            for (i, header) in self.headers.iter().enumerate() {
                                let arrow = if self.sort_column == Some(i) {
                                    if self.sort_ascending { " ▲" } else { " ▼" }
                                } else {
                                    ""
                                };
                                if ui.button(format!("{}{}", header, arrow)).clicked() {
                                    clicked_col = Some(i);
                                }
                            }
                            ui.end_row();
                            
                            if let Some(col) = clicked_col {
                                self.sort_by_column(col);
                            }
                            
                            // Data rows
                            for row in &self.rows {
                                for cell in row {
                                    ui.label(cell);
                                }
                                ui.end_row();
                            }
                        });
                });
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Ingen data att visa. Klicka 'Visa' för att dekryptera.");
                });
            }
        });
    }
}

fn main() {
    let native_options = eframe::NativeOptions {
        initial_window_size: Some(Vec2::new(800.0, 600.0)),
        ..Default::default()
    };
    
    let _ = eframe::run_native(
        "Rustdatabasen - Kryptera & Visa",
        native_options,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            let theme = load_theme_from_yaml();
            apply_theme(&cc.egui_ctx, &theme);
            Box::new(RustDatabasenApp::default())
        }),
    );
}
