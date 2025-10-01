# Rustdatabasen - Stand-alone System

Detta projekt består av två självständiga program:

## 1. rustdatabasen
**Krypterar och dekrypterar person-data**

### Funktioner:
- Läser från `personer` (klartext)
- Krypterar med AEAD (AES-256-GCM) + Argon2id
- Sparar till `personer2` (krypterad)
- Dekrypterar och visar i tabell med sortering
- Läser design från `desig.yaml`

### Användning:
```bash
./rustdatabasen
```

**I GUI:**
1. Fyll i **ID** (identifieringskod)
2. Fyll i **Lösenord**
3. Klicka **"Kryptera & Kopiera"** för att kryptera `personer` → `personer2`
4. Klicka **"Visa"** för att dekryptera och visa data
5. Klicka på kolumnrubriker för att sortera (↑/↓)

## 2. aiagent_design
**Designverktyg för att uppdatera desig.yaml**

### Funktioner:
- GUI för att skapa design-teman
- Genererar `desig.yaml` baserat på prompt
- Heuristisk färgval (mörk, varm, pastel, neon, etc.)

### Användning:
```bash
./aiagent_design
```

**I GUI:**
1. Skriv en design-prompt (t.ex. "mörk, modern" eller "blå bakgrund")
2. Klicka **"Generera/uppdatera desig.yaml"**
3. Filen `desig.yaml` uppdateras (backup till `desig.yaml.bak`)

**CLI-läge:**
```bash
./aiagent_design --generate "din prompt här"
```

## Filer

### Nödvändiga filer:
- `rustdatabasen` - Huvudprogram (körbar)
- `aiagent_design` - Designverktyg (körbar)
- `desig.yaml` - Design/tema-konfiguration
- `personer` - Källdata (klartext)
- `personer2` - Krypterad data
- `rustdatabasen.rs` - Källkod för rustdatabasen
- `aiagent_design.rs` - Källkod för aiagent_design
- `desig.rs` - Hjälpmodul för tema-laddning
- `Cargo.toml` - Byggkonfiguration

### Backup-filer:
- `rustdatabasen_gtk_backup.rs` - Gammal GTK-version (backup)
- `desig.yaml.bak` - Backup av design-fil

## Bygga från källkod

```bash
cargo build --release
cp target/release/rustdatabasen .
cp target/release/aiagent_design .
chmod +x rustdatabasen aiagent_design
```

## desig.yaml Format

```yaml
bg: [30,60,120]           # Bakgrundsfärg (RGB)
text: [255,255,255]       # Textfärg (RGB)
heading_size: 18.0        # Rubrikstorlek
row_even: [40,70,140]     # Jämna rader
row_odd: [25,50,110]      # Udda rader
```

## Kryptering

- **Algoritm:** AES-256-GCM (AEAD)
- **KDF:** Argon2id (memory=64MB, iterations=3, parallelism=1)
- **Salt:** 16 bytes (slumpmässig per post)
- **Nonce:** 12 bytes (slumpmässig per post)
- **Format i personer2:** `ID|base64(salt)|base64(nonce)|base64(ciphertext)`

## Dependencies

- `eframe` - GUI framework
- `aes-gcm` - AEAD kryptering
- `argon2` - Key derivation
- `base64` - Encoding
- `rand` - Random number generation
- `serde` + `serde_yaml` - YAML parsing

## Säkerhet

⚠️ **VIKTIGT:**
- Lösenordet lagras INTE
- Varje kryptering använder nya slumpmässiga salt och nonce
- Fel lösenord ger dekrypteringsfel
- Ingen bruteforce-skydd implementerat (använd starka lösenord!)
