// Minimal single-file Rust GTK app for copying from `personer` to `personer2.txt` and viewing `personer2.txt`.
// - Two buttons: "Kopiera till personer2.txt" and "Visa personer2.txt"
// - Headless flags: --copy-only and --debug-test

use gtk::prelude::*;
use gio::prelude::*;
use glib;
use std::thread;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use gtk::{Application, ApplicationWindow, Button, TextView};
use std::process::Command;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use rand::RngCore;
use rand::rngs::OsRng;
use base64::{engine::general_purpose, Engine as _};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::{Argon2, Params, Algorithm, Version};
// pbkdf2 unused now
use std::path::Path;

// Simple debug logger: append timestamped messages to debug_log
fn debug_log(msg: &str) {
    let _ = OpenOptions::new()
        .create(true)
        .append(true)
        .open("debug_log")
        .and_then(|mut f| {
            let _ = writeln!(f, "{} - {}", chrono::Local::now().to_rfc3339(), msg);
            Ok(())
        });
}

// Derive a 32-byte key using Argon2id with conservative default parameters.
fn derive_argon2_key(password: &[u8], salt: &[u8]) -> [u8; 32] {
    // Params: memory_kib=65536 (~64 MiB), iterations=3, parallelism=1
    let params = Params::new(65536, 3, 1, None).expect("argon2 params");
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; 32];
    argon2.hash_password_into(password, salt, &mut key).expect("argon2 derive");
    key
}

const PERSONER: &str = "/home/matsu/databasen/personer";
const PERSONER2: &str = "/home/matsu/databasen/personer2.txt";

fn main() {
    // Headless modes
    if std::env::args().nth(1).as_deref() == Some("--copy-only") {
    // headless mode: prompt for identifier (visible) and password (hidden)
    use std::io::{self, Write as IoWrite};
    print!("Identifier for this entry: ");
    io::stdout().flush().ok();
    let mut id = String::new();
    io::stdin().read_line(&mut id).ok();
    let id = id.trim().to_string();
    let pwd = rpassword::prompt_password("Password: ").unwrap_or_default();
        if let Err(e) = copy_personer_to_personer2_with_encrypt(&id, &pwd) {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
        return;
    }
    if std::env::args().nth(1).as_deref() == Some("--debug-test") {
        match run_debug_test() {
            Ok(_) => println!("DEBUG TEST: PASS"),
            Err(e) => eprintln!("DEBUG TEST: FAIL - {}", e),
        }
        return;
    }

    // Headless show/decrypt: --show-id [IDENTIFIER]
    if std::env::args().nth(1).as_deref() == Some("--show-id") {
        let identifier = std::env::args().nth(2).unwrap_or_else(|| {
            use std::io::{self, Write};
            print!("Identifier to show: ");
            io::stdout().flush().ok();
            let mut s = String::new();
            io::stdin().read_line(&mut s).ok();
            s.trim().to_string()
        });
        let pwd = rpassword::prompt_password("Password: ").unwrap_or_default();
        match decrypt_entries(&identifier, &pwd) {
            Ok(lines) => {
                if lines.is_empty() {
                    eprintln!("No entries found for identifier '{}', printing raw file:", identifier);
                    match read_personer2() {
                        Ok(s) => println!("{}", s),
                        Err(e) => eprintln!("Could not read personer2.txt: {}", e),
                    }
                } else {
                    for l in lines { println!("{}", l); }
                }
                std::process::exit(0);
            }
            Err(e) => { eprintln!("Decrypt error: {}", e); std::process::exit(2); }
        }
    }

    let app = Application::new(Some("com.example.rustdatabasen"), Default::default()).expect("Failed to create GTK application");
    app.connect_activate(|app| {
        let window = ApplicationWindow::new(app);
        window.set_title("Rustdatabasen — enkel kopiering");
        window.set_default_size(600, 400);

        // Top area: inline identifier + password entries (visible in main window)
        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 6);

    let top_grid = gtk::Grid::new();
    // compact spacing for a smaller prompt
    top_grid.set_row_spacing(2);
    top_grid.set_column_spacing(4);
    let id_label = gtk::Label::new(Some("Id:"));
    let id_entry = gtk::Entry::new();
    id_entry.set_placeholder_text(Some("id"));
    // make entry visually compact
    id_entry.set_width_chars(12);
    // Make the entry approximately 90 pixels wide
    id_entry.set_size_request(90, -1);
    id_entry.set_margin_end(6);
    let pwd_label = gtk::Label::new(Some("Pwd:"));
    let pwd_entry = gtk::Entry::new();
    pwd_entry.set_visibility(false);
    pwd_entry.set_width_chars(12);
    // Make the password entry approximately 90 pixels wide
    pwd_entry.set_size_request(90, -1);
    pwd_entry.set_margin_end(6);
    top_grid.attach(&id_label, 0, 0, 1, 1);
    top_grid.attach(&id_entry, 1, 0, 1, 1);
    top_grid.attach(&pwd_label, 0, 1, 1, 1);
    top_grid.attach(&pwd_entry, 1, 1, 1, 1);

    // Put prompts and buttons into a top box so they sit at the top of the window
    let top_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
    top_box.set_hexpand(true);
    top_box.set_margin_top(6);
    top_box.set_margin_start(6);
    top_box.set_margin_end(6);

    // Design prompt entry: small single-line prompt where user writes formatting instructions
    let prompt_entry = gtk::Entry::new();
    prompt_entry.set_placeholder_text(Some("Skriv formatinstruktioner här"));
    // make prompt visually compact (approx 90 px)
    prompt_entry.set_width_chars(20);
    prompt_entry.set_size_request(90, -1);
    prompt_entry.set_margin_bottom(4);

    // Additional design prompt: describe text color / background
    let design_entry = gtk::Entry::new();
    design_entry.set_placeholder_text(Some("Design: textfärg, bakgrund"));
    design_entry.set_width_chars(20);
    design_entry.set_size_request(90, -1);
    design_entry.set_margin_bottom(4);

        // Scrolled text display
        let scrolled = gtk::ScrolledWindow::new(gtk::NONE_ADJUSTMENT, gtk::NONE_ADJUSTMENT);
        scrolled.set_vexpand(true);
    let display = TextView::new();
    display.set_editable(false);
    // start with an empty display; only show content when user clicks "Visa"
    display.get_buffer().unwrap().set_text("");
    scrolled.add(&display);

        // Buttons row
    let btn_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let btn_copy = Button::with_label("Kopiera");
    let btn_show = Button::with_label("Visa");
    let btn_format = Button::with_label("Formatera");
    // make buttons compact (approx 90 px width)
    btn_copy.set_size_request(90, -1);
    btn_show.set_size_request(90, -1);
    btn_format.set_size_request(90, -1);
        btn_box.pack_start(&btn_copy, false, false, 0);
        btn_box.pack_start(&btn_show, false, false, 0);
        btn_box.pack_start(&btn_format, false, false, 0);

    // status label for short messages
    let status_label = gtk::Label::new(None);
    status_label.set_halign(gtk::Align::Start);
    status_label.set_margin_top(4);

    // Channel to send UI updates from background threads: (message, is_status)
    let (tx, rx) = glib::MainContext::channel::<(String, bool)>(glib::PRIORITY_DEFAULT);
    // Only allow display updates (is_status==false) when user explicitly clicked "Visa".
    let allow_display = Arc::new(AtomicBool::new(false));
    // Attach receiver to update UI widgets
    let status_clone_for_rx = status_label.clone();
    let display_for_rx = display.clone();
    let allow_display_for_rx = allow_display.clone();
    rx.attach(None, move |(msg, is_status)| {
        if is_status {
            status_clone_for_rx.set_text(&msg);
        } else {
            // Only update the display if the user asked to show.
            if allow_display_for_rx.load(Ordering::SeqCst) {
                if let Some(buffer) = display_for_rx.get_buffer() {
                    buffer.set_text(&msg);
                }
                // After writing the display, clear the flag so further background jobs can't overwrite it
                allow_display_for_rx.store(false, Ordering::SeqCst);
            } else {
                debug_log("Ignored display update because allow_display is false");
            }
        }
        glib::Continue(true)
    });

    // Pack top grid and buttons into top_box, then place scrolled below
    top_box.pack_start(&top_grid, false, false, 0);
    // pack the prompt just above the buttons so it sits near the actions
    top_box.pack_start(&prompt_entry, false, false, 0);
    // pack design prompt below the main prompt
    top_box.pack_start(&design_entry, false, false, 0);
    top_box.pack_start(&btn_box, false, false, 0);
    top_box.pack_start(&status_label, false, false, 0);

    vbox.pack_start(&top_box, false, false, 0);
    vbox.pack_start(&scrolled, true, true, 0);

    let id_entry_clone = id_entry.clone();
    let pwd_entry_clone = pwd_entry.clone();
    let status_clone = status_label.clone();

        let tx_copy = tx.clone();
        let allow_display_clone = allow_display.clone();
        btn_copy.connect_clicked(move |_| {
            // ensure display updates are disabled while copying
            allow_display_clone.store(false, Ordering::SeqCst);
            // Log click
            let _ = std::fs::OpenOptions::new().create(true).append(true).open("action.log").and_then(|mut f| {
                let _ = writeln!(f, "{} - CLICK copy", chrono::Local::now().to_rfc3339());
                Ok(())
            });

            let identifier = id_entry_clone.get_text().to_string();
            let password = pwd_entry_clone.get_text().to_string();

            if identifier.is_empty() {
                status_clone.set_text("Fyll i identifier i övre fältet innan kopiering.");
                return;
            }
            // Run the potentially slow encryption+append in a background thread
            let id = identifier.clone();
            let pwd = password.clone();
            let tx2 = tx_copy.clone();
            // update status quickly
            status_clone.set_text("Kopierar...\n");
            thread::spawn(move || {
                let r = std::panic::catch_unwind(|| {
                    debug_log(&format!("copy thread start (id={})", id));
                    let res = copy_personer_to_personer2_with_encrypt(&id, &pwd);
                    match res {
                        Ok(_) => {
                            // write verifier for this id if not present
                            let _ = write_verifier_if_missing(&id, &pwd);
                            let _ = tx2.send(("Klar: kopierat till personer2.txt".to_string(), true));
                            debug_log(&format!("copy thread ok (id={})", id));
                        }
                        Err(e) => {
                            let _ = tx2.send((format!("Kunde inte kopiera: {}", e), true));
                            debug_log(&format!("copy thread error (id={}, err={})", id, e));
                        }
                    }
                });
                if let Err(e) = r {
                    debug_log(&format!("copy thread panicked (id={:?})", e));
                    let _ = tx2.send(("Internt fel under kopiering".to_string(), true));
                }
            });
        });

        let id_entry_clone2 = id_entry.clone();
    let pwd_entry_clone2 = pwd_entry.clone();
    let status_clone2 = status_label.clone();
        let tx_show = tx.clone();
        let allow_display_clone2 = allow_display.clone();
        btn_show.connect_clicked(move |_| {
            // enable a single display update from background threads
            allow_display_clone2.store(true, Ordering::SeqCst);
            // Log click
            let _ = std::fs::OpenOptions::new().create(true).append(true).open("action.log").and_then(|mut f| {
                let _ = writeln!(f, "{} - CLICK show", chrono::Local::now().to_rfc3339());
                Ok(())
            });

            let identifier = id_entry_clone2.get_text().to_string();
            let password = pwd_entry_clone2.get_text().to_string();
            if identifier.is_empty() {
                status_clone2.set_text("Fyll i identifier innan du visar innehåll.");
                return;
            }
            status_clone2.set_text("Läser och dekrypterar i bakgrunden...");
            let id = identifier.clone();
            let pwd = password.clone();
            let tx2 = tx_show.clone();
            thread::spawn(move || {
                let r = std::panic::catch_unwind(|| {
                    debug_log(&format!("show thread start (id={})", id));
                    // quick verify password using stored verifier (if present)
                    match verify_password_for_identifier(&id, &pwd) {
                        Ok(Some(true)) => {
                            // proceed to decrypt entries
                            let lines = decrypt_entries(&id, &pwd).unwrap_or_default();
                            if lines.is_empty() {
                                let _ = tx2.send(("Hittade poster men kunde inte dekryptera dem.".to_string(), true));
                                debug_log(&format!("show thread: no lines decrypted for id={}", id));
                            } else {
                                let out = lines.join("\n");
                                let _ = tx2.send((format!("Visar dekrypterade poster ({} rader)", lines.len()), true));
                                let _ = tx2.send((out, false));
                                debug_log(&format!("show thread success decrypt for id={} ({} rows)", id, lines.len()));
                            }
                        }
                        Ok(Some(false)) => {
                            let _ = tx2.send(("Fel lösenord för vald identifier.".to_string(), true));
                            debug_log(&format!("show thread: verifier found but password wrong for id={}", id));
                        }
                        Ok(None) => {
                            // no verifier stored — fall back to full scan/decrypt
                            debug_log(&format!("show thread: no verifier for id={}, falling back to full decrypt", id));
                            match decrypt_entries(&id, &pwd) {
                                Ok(lines) => {
                                    if lines.is_empty() {
                                        let _ = tx2.send(("Hittade poster men kunde inte dekryptera dem.".to_string(), true));
                                    } else {
                                        let out = lines.join("\n");
                                        let _ = tx2.send((format!("Visar dekrypterade poster ({} rader)", lines.len()), true));
                                        let _ = tx2.send((out, false));
                                    }
                                }
                                Err(e) => {
                                    let _ = tx2.send((format!("Kunde inte dekryptera: {}", e), true));
                                }
                            }
                        }
                        Err(e) => {
                            let _ = tx2.send((format!("Verifieringsfel: {}", e), true));
                        }
                    }
                });
                if let Err(e) = r {
                    debug_log(&format!("show thread panicked (id={:?})", e));
                    let _ = tx2.send(("Internt fel under visning".to_string(), true));
                }
            });
        });

    // Formatter button: takes the currently displayed text and the prompt, calls local CLI ai_formatter
    let prompt_entry_clone = prompt_entry.clone();
    let design_entry_clone = design_entry.clone();
    let display_clone_for_format = display.clone();
    let status_clone3 = status_label.clone();
    let tx_format = tx.clone();
    let allow_display_clone3 = allow_display.clone();
        btn_format.connect_clicked(move |_| {
            // allow a single display update for the formatter result
            allow_display_clone3.store(true, Ordering::SeqCst);
            // Log click
            let _ = std::fs::OpenOptions::new().create(true).append(true).open("action.log").and_then(|mut f| {
                let _ = writeln!(f, "{} - CLICK format", chrono::Local::now().to_rfc3339());
                Ok(())
            });

            // read prompt, design instruction and current displayed text
            let prompt_text = prompt_entry_clone.get_text().to_string();
            let design_text = design_entry_clone.get_text().to_string();
            // get the current display buffer content
            let buf_opt = display_clone_for_format.get_buffer();
            let display_text = if let Some(buf) = buf_opt {
                let start = buf.get_start_iter();
                let end = buf.get_end_iter();
                buf.get_text(&start, &end, true).map(|s| s.to_string()).unwrap_or_default()
            } else { String::new() };

            if display_text.trim().is_empty() {
                status_clone3.set_text("Inga dekrypterade poster att formatera. Klicka 'Visa' först.");
                return;
            }

            status_clone3.set_text("Formaterar i bakgrunden...");
            let prompt_clone = prompt_text.clone();
            let design_clone = design_text.clone();
            let tx2 = tx_format.clone();
            thread::spawn(move || {
                let r = std::panic::catch_unwind(|| {
                    debug_log(&format!("format thread start (prompt_len={}, text_len={})", prompt_clone.len(), display_text.len()));
                    // write a temp file with prompt and text separated by a delimiter
                    let tmp_path = format!("/tmp/rustdatabasen_ai_{}.txt", chrono::Local::now().timestamp_nanos());
                    // include design instruction together with user prompt so formatter sees both
                    let mut combined_prompt = prompt_clone.clone();
                    if !design_clone.trim().is_empty() {
                        combined_prompt = format!("{}\nDesign: {}", combined_prompt, design_clone.trim());
                    }
                    let content = format!("{}\n===\n{}", combined_prompt, display_text);
                    if let Err(e) = std::fs::write(&tmp_path, content) {
                        let _ = tx2.send((format!("Kunde inte skriva tempfil för AI: {}", e), true));
                        debug_log(&format!("format thread: write tmp failed: {}", e));
                        return;
                    }
                    // Prefer a local Rust formatter binary `./ai_formatter_rs` if present, otherwise fall back to python3 ai_formatter.py <tmpfile>
                    let output = if Path::new("./ai_formatter_rs").exists() {
                        Command::new("./ai_formatter_rs").arg(&tmp_path).output()
                    } else {
                        Command::new("python3").arg("ai_formatter.py").arg(&tmp_path).output()
                    };
                    // remove temp file
                    let _ = std::fs::remove_file(&tmp_path);
                    match output {
                        Ok(o) => {
                            if !o.status.success() {
                                let err = String::from_utf8_lossy(&o.stderr).to_string();
                                let _ = tx2.send((format!("AI-formatören misslyckades: {}", err), true));
                                debug_log(&format!("format thread: ai exit nonzero: {}", err));
                            } else {
                                let out = String::from_utf8_lossy(&o.stdout).to_string();
                                let out_len = out.len();
                                let _ = tx2.send(("Formatering klar".to_string(), true));
                                let _ = tx2.send((out.clone(), false));
                                debug_log(&format!("format thread: success, out_len={}", out_len));
                            }
                        }
                        Err(e) => {
                            let _ = tx2.send((format!("Kunde inte köra ai_formatter: {}", e), true));
                            debug_log(&format!("format thread: spawn failed: {}", e));
                        }
                    }
                });
                if let Err(e) = r {
                    debug_log(&format!("format thread panicked (err={:?})", e));
                    let _ = tx2.send(("Internt fel i formateraren".to_string(), true));
                }
            });
        });

        window.add(&vbox);
        window.show_all();
    });
    let args: Vec<String> = std::env::args().collect();
    app.run(args.as_slice());
}

// New: encrypt plaintext and append line to PERSONER2 in format: identifier:BASE64(salt||nonce||ciphertext)
fn encrypt_and_write(identifier: &str, password: &str, plaintext: &str) -> std::io::Result<()> {
    // generate salt
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    // derive key with Argon2id
    let key = derive_argon2_key(password.as_bytes(), &salt);
    let cipher = Aes256Gcm::new_from_slice(&key).expect("key length");
    // generate nonce
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ct = cipher.encrypt(nonce, plaintext.as_bytes()).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("encrypt failed: {}", e)))?;
    // compose blob salt||nonce||ct
    let mut blob: Vec<u8> = Vec::new();
    blob.extend_from_slice(&salt);
    blob.extend_from_slice(&nonce_bytes);
    blob.extend_from_slice(&ct);
    let b64 = general_purpose::STANDARD.encode(&blob);
    debug_log(&format!("encrypt: id={}, plaintext_len={}, blob_len={}, b64_len={}", identifier, plaintext.len(), blob.len(), b64.len()));
    // write line
    let mut f = OpenOptions::new().create(true).append(true).open(PERSONER2)?;
    f.write_all(identifier.as_bytes())?;
    f.write_all(b":")?;
    f.write_all(b64.as_bytes())?;
    f.write_all(b"\n")?;
    f.flush()?;
    Ok(())
}

// Write a small verifier blob for an identifier so we can quickly validate a password
// without attempting to decrypt all entries. Verifier stored as a special line:
// __pw__:<identifier>:BASE64(salt||nonce||ciphertext_of("VERIFIER:<identifier>"))
fn write_verifier_if_missing(identifier: &str, password: &str) -> std::io::Result<()> {
    // check if verifier exists
    if Path::new(PERSONER2).exists() {
        let data = std::fs::read_to_string(PERSONER2)?;
        for l in data.lines() {
            if l.starts_with(&format!("__pw__:{}:", identifier)) {
                return Ok(()); // already exists
            }
        }
    }
    // create verifier
    let plain = format!("VERIFIER:{}", identifier);
    // generate salt
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    // derive key with Argon2id
    let key = derive_argon2_key(password.as_bytes(), &salt);
    let cipher = Aes256Gcm::new_from_slice(&key).expect("key length");
    // nonce
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ct = cipher.encrypt(nonce, plain.as_bytes()).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("encrypt verifier failed: {}", e)))?;
    let mut blob: Vec<u8> = Vec::new();
    blob.extend_from_slice(&salt);
    blob.extend_from_slice(&nonce_bytes);
    blob.extend_from_slice(&ct);
    let b64 = general_purpose::STANDARD.encode(&blob);
    let mut f = OpenOptions::new().create(true).append(true).open(PERSONER2)?;
    f.write_all(b"__pw__:")?;
    f.write_all(identifier.as_bytes())?;
    f.write_all(b":")?;
    f.write_all(b64.as_bytes())?;
    f.write_all(b"\n")?;
    f.flush()?;
    debug_log(&format!("wrote verifier for id={}", identifier));
    Ok(())
}

// Verify password for given identifier. Returns Ok(Some(true)) if verifier exists and password OK,
// Ok(Some(false)) if verifier exists but password wrong, Ok(None) if no verifier present.
fn verify_password_for_identifier(identifier: &str, password: &str) -> std::io::Result<Option<bool>> {
    if !Path::new(PERSONER2).exists() {
        return Ok(None);
    }
    let data = std::fs::read_to_string(PERSONER2)?;
    for l in data.lines() {
        if l.starts_with(&format!("__pw__:{}:", identifier)) {
            if let Some(rest) = l.strip_prefix(&format!("__pw__:{}:", identifier)) {
                let rest_trim = rest.trim_start_matches(':').trim();
                let blob = match general_purpose::STANDARD.decode(rest_trim) {
                    Ok(b) => b,
                    Err(_) => return Ok(Some(false)),
                };
                if blob.len() < 16 + 12 + 1 { return Ok(Some(false)); }
                let salt = &blob[0..16];
                let nonce = &blob[16..28];
                let ct = &blob[28..];
                let key = derive_argon2_key(password.as_bytes(), salt);
                let cipher = match Aes256Gcm::new_from_slice(&key) {
                    Ok(c) => c,
                    Err(_) => return Ok(Some(false)),
                };
                let nonce_obj = Nonce::from_slice(nonce);
                match cipher.decrypt(nonce_obj, ct) {
                    Ok(pt) => {
                        if let Ok(s) = String::from_utf8(pt) {
                            if s == format!("VERIFIER:{}", identifier) {
                                return Ok(Some(true));
                            }
                        }
                        return Ok(Some(false));
                    }
                    Err(_) => return Ok(Some(false)),
                }
            }
        }
    }
    Ok(None)
}

fn copy_personer_to_personer2_with_encrypt(identifier: &str, password: &str) -> std::io::Result<()> {
    let src = Path::new(PERSONER);
    if !src.exists() {
        return Ok(());
    }
    let mut persons = String::new();
    File::open(src)?.read_to_string(&mut persons)?;
    if persons.trim().is_empty() {
        return Ok(());
    }

    // Performance: derive key once per copy operation (reuse same salt for all lines)
    // This reduces expensive PBKDF2 calls. Security note: reusing the same salt for
    // multiple entries under the same identifier weakens per-line uniqueness of the salt,
    // but still uses a random salt per copy operation. If you need per-line salts,
    // revert to encrypting each line independently (slower).
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    let key = derive_argon2_key(password.as_bytes(), &salt);
    let cipher = Aes256Gcm::new_from_slice(&key).expect("key length");

    // Open target once for appending to avoid repeated open/flush overhead
    let mut f = OpenOptions::new().create(true).append(true).open(PERSONER2)?;
    for line in persons.lines() {
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ct = cipher.encrypt(nonce, line.as_bytes()).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("encrypt failed: {}", e)))?;
        let mut blob: Vec<u8> = Vec::new();
        // write the same salt for each entry (see note above)
        blob.extend_from_slice(&salt);
        blob.extend_from_slice(&nonce_bytes);
        blob.extend_from_slice(&ct);
        let b64 = general_purpose::STANDARD.encode(&blob);
        debug_log(&format!("batch encrypt: id={}, plaintext_len={}, blob_len={}, b64_len={}", identifier, line.len(), blob.len(), b64.len()));
        f.write_all(identifier.as_bytes())?;
        f.write_all(b":")?;
        f.write_all(b64.as_bytes())?;
        f.write_all(b"\n")?;
    }
    f.flush()?;
    Ok(())
}

fn decrypt_entries(identifier: &str, password: &str) -> std::io::Result<Vec<String>> {
    // read personer2.txt and find lines starting with identifier:
    let mut out: Vec<String> = Vec::new();
    if !Path::new(PERSONER2).exists() {
        return Ok(out);
    }
    let data = std::fs::read_to_string(PERSONER2)?;
    for line in data.lines() {
        if let Some(rest) = line.strip_prefix(&format!("{}:", identifier)) {
            // rest is base64 blob
            let rest_trim = rest.trim();
            debug_log(&format!("decrypt: id={}, b64_len={} line_len={}", identifier, rest_trim.len(), line.len()));
            let blob = match general_purpose::STANDARD.decode(rest_trim) {
                Ok(b) => b,
                Err(e) => {
                    debug_log(&format!("base64 decode error for id={} err={}", identifier, e));
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, format!("base64 decode: {}", e)));
                }
            };
            if blob.len() < 16 + 12 + 1 { continue; }
            let salt = &blob[0..16];
            let nonce = &blob[16..28];
            let ct = &blob[28..];
            debug_log(&format!("decrypt: id={}, blob_len={}, salt_len={}, nonce_len={}, ct_len={}", identifier, blob.len(), salt.len(), nonce.len(), ct.len()));
            // derive key with Argon2id
            let key = derive_argon2_key(password.as_bytes(), salt);
            let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("cipher init: {}", e)))?;
            let nonce_obj = Nonce::from_slice(nonce);
            let pt = match cipher.decrypt(nonce_obj, ct) {
                Ok(p) => p,
                Err(e) => {
                    debug_log(&format!("decrypt failed for id={} err={}", identifier, e));
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, format!("decrypt failed: {}", e)));
                }
            };
            let s = String::from_utf8(pt).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("utf8: {}", e)))?;
            out.push(s);
        }
    }
    Ok(out)
}

fn read_personer2() -> std::io::Result<String> {
    let mut s = String::new();
    if Path::new(PERSONER2).exists() {
        File::open(PERSONER2)?.read_to_string(&mut s)?;
    }
    Ok(s)
}

fn run_debug_test() -> std::io::Result<()> {
    use std::fs::{create_dir_all, remove_dir_all};
    let tmpdir = "/tmp/rustdatabasen-debug";
    let _ = remove_dir_all(tmpdir);
    create_dir_all(tmpdir)?;
    let test_personer = format!("{}/personer", tmpdir);
    let test_personer2 = format!("{}/personer2.txt", tmpdir);
    std::fs::write(&test_personer, "Alice 30\nBob 25\n")?;
    std::fs::write(&test_personer2, "")?;
    let persons = std::fs::read_to_string(&test_personer)?;
    let existing = std::fs::read_to_string(&test_personer2)?;
    // simulate append
    let mut f = OpenOptions::new().create(true).append(true).open(&test_personer2)?;
    if !existing.is_empty() && !existing.ends_with('\n') { f.write_all(b"\n")?; }
    f.write_all(persons.as_bytes())?;
    if !persons.ends_with('\n') { f.write_all(b"\n")?; }
    f.flush()?;
    let got = std::fs::read_to_string(&test_personer2)?;
    if got.contains("Alice 30") && got.contains("Bob 25") {
        let _ = remove_dir_all(tmpdir);
        Ok(())
    } else {
        Err(std::io::Error::new(std::io::ErrorKind::Other, "content mismatch after append"))
    }
}
