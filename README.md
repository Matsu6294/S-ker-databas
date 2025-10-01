# 🔒 Secure Data Manager

Ett säkert krypterings- och designsystem med två självständiga program byggda i Rust.

## 📦 Programmen

### 1. 🔐 rustdatabasen
**Krypterar och dekrypterar känslig data med lösenordsskydd**

#### Funktioner:
- ✅ Läser från `personer` (klartext)
- ✅ Krypterar med AEAD (AES-256-GCM) + Argon2id key derivation
- ✅ **Lösenordsverifiering** - Hash:en sparas i krypterad data
- ✅ **Flera kategorier** - Stödjer flera ID:n i samma personer2-fil
- ✅ Sorterbara kolumner med klickbara headers (▲/▼)
- ✅ **Dynamiska kolumnnamn** - Kolumn1, Kolumn2, Kolumn3 osv.
- ✅ Alignerade kolumner i Grid-layout
- ✅ Läser tema från `desig.yaml`

#### Användning:
```bash
./rustdatabasen
```

**I GUI:**
1. Fyll i **ID** (identifieringskod, t.ex. "personal", "projekt", "kunder")
2. Fyll i **Lösenord** 
3. Klicka **"Kryptera & Kopiera"** 
   - Läser `personer` → krypterar → lägger till/uppdaterar i `personer2`
   - Flera ID:n kan finnas samtidigt!
4. Klicka **"Visa"** 
   - Dekrypterar och verifierar lösenord
   - Visar data i sorterad tabell
   - Felmeddelande om fel lösenord!
5. Klicka på kolumnrubriker för att sortera (↑/↓)

### 2. 🎨 aiagent_design
**Visuellt designverktyg för att skapa teman**

#### Funktioner:
- ✨ **Två arbetslägen:**
  - **📝 Prompt-läge** - Skriv fri text ("mörk blå", "ljus grön")
  - **🎨 Färgväljare** - Grafiska RGB-skjutreglage med live-förhandsvisning
- 🖼️ **Stora färgrutor** - Se exakt hur färgerna ser ut
- 📊 **Text-på-bakgrund preview** - Kontrollera kontrast innan sparning
- 🔴🟢🔵 **RGB-skjutreglage** för varje färgelement (0-255)
- 📐 **Rubrikstorlek** med live-förhandsvisning (10-32px)
- 💾 **Stor SPARA-knapp** alltid synlig
- 🔄 Automatisk backup till `desig.yaml.bak`

#### Användning:
```bash
./aiagent_design
```

**Prompt-läge:**
1. Välj "📝 Prompt-läge"
2. Skriv prompt som "mörk blå bakgrund" eller "ljus modern"
3. Klicka **"✨ Generera från prompt"** - färger genereras
4. Klicka **"💾 SPARA TILL desig.yaml"**

**Färgväljare-läge:**
1. Välj "🎨 Färgväljare"
2. Dra skjutreglage för:
   - 🖼️ Bakgrundsfärg (stor färgruta)
   - 📝 Textfärg (med text-exempel över bakgrund)
   - 📊 Jämna/Udda radfärger (zebra-mönster)
   - 📐 Rubrikstorlek
3. Se live-förhandsvisning i stora färgrutor
4. Klicka **"💾 SPARA TILL desig.yaml"**

**CLI-läge:**
```bash
./aiagent_design --generate "din prompt här"
```

#### Stödda färgnyckelord:
- Svenska: blå, röd, grön, gul, svart, grå, ljus, mörk
- Engelska: blue, red, green, yellow, black, gray, dark, light, warm, pastel, neon

## 📁 Filer

### För att köra programmen (minsta uppsättning):
```
rustdatabasen          # Huvudprogram (13M binär)
aiagent_design         # Designverktyg (13M binär)
desig.yaml             # Tema-konfiguration
personer               # Din data (klartext)
personer2              # Krypterad data (skapas automatiskt)
```

### För utveckling/kompilering:
```
rustdatabasen.rs       # Källkod för rustdatabasen
aiagent_design.rs      # Källkod för aiagent_design
desig.rs               # Delad modul för tema-laddning
Cargo.toml             # Byggkonfiguration
Cargo.lock             # Versionslåsning
target/                # Kompilerade filer
```

### Automatiskt skapade:
```
desig.yaml.bak         # Backup av tema (skapas vid uppdatering)
personer2              # Krypterad data (skapas vid första kryptering)
```

### Backup/historik:
```
rustdatabasen_gtk_backup.rs  # Gammal GTK-version (före egui-migrering)
```

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

## 🔐 Kryptering & Säkerhet

### Krypterings-specifikation:
- **Algoritm:** AES-256-GCM (AEAD - Authenticated Encryption with Associated Data)
- **KDF:** Argon2id (memory=64MB, iterations=3, parallelism=1)
- **Salt:** 16 bytes (slumpmässig per post)
- **Nonce:** 12 bytes (slumpmässig per post)
- **Lösenordsverifiering:** Hash sparas i krypterad data

### Format i personer2:
```
ID|base64(salt)|base64(nonce)|base64(ciphertext)
```

**Innehåll i dekrypterad data:**
```
PWD:lösenordshash|faktisk_data
```

### Säkerhetsfunktioner:
✅ **Lösenord verifieras** - Tydligt felmeddelande om fel lösenord  
✅ **Ingen klartext** - Lösenord sparas aldrig i klartext  
✅ **Per-post salt/nonce** - Varje kryptering är unik  
✅ **Flera kategorier** - Olika ID:n kan ha olika lösenord  
✅ **Uppdatering utan överskrivning** - Lägger till nya ID:n utan att radera gamla

### Säkerhetsnivåer:
| Vad | Säkerhet |
|-----|----------|
| Krypterad data (personer2) | ⭐⭐⭐⭐⭐ Mycket säkert |
| Lösenordsverifiering | ⭐⭐⭐⭐ Säkert (hash i krypterad data) |
| Lösenord i minne | ⭐⭐ Varning (klartext i RAM) |

## 💡 Användningsexempel

### Scenario 1: Flera avdelningar i samma fil
```bash
# Kryptera personal-data
ID: personal
Lösenord: mitt_lösenord_123
→ Klicka "Kryptera & Kopiera"

# Kryptera projekt-data (samma personer-fil)
ID: projekt
Lösenord: annat_lösenord_456
→ Klicka "Kryptera & Kopiera"

# Nu finns båda i personer2!

# Visa personal-data
ID: personal
Lösenord: mitt_lösenord_123
→ Klicka "Visa"

# Visa projekt-data
ID: projekt  
Lösenord: annat_lösenord_456
→ Klicka "Visa"
```

### Scenario 2: Skapa tema med färgväljare
```bash
./aiagent_design
→ Välj "🎨 Färgväljare"
→ Dra Bakgrund: R=30, G=60, B=120 (mörk blå)
→ Dra Text: R=255, G=255, B=255 (vit)
→ Se live-förhandsvisning
→ Klicka "💾 SPARA"
```

## 🐛 Felsökning

### "Fel lösenord!"
- Kontrollera att du använder rätt lösenord för detta ID
- Lösenord är case-sensitive
- Olika ID:n kan ha olika lösenord

### "Ingen post hittades för denna identifierare"
- ID:t finns inte i personer2
- Kryptera först med "Kryptera & Kopiera"

### "Felaktigt dataformat - gammal kryptering?"
- Filen krypterades med gammal version
- Kryptera om data med nya programmet

### Programmet startar inte
```bash
chmod +x rustdatabasen aiagent_design
```

## 📚 Dependencies

- `eframe 0.22` - GUI framework (egui)
- `aes-gcm` - AEAD kryptering
- `argon2` - Key derivation (KDF)
- `base64` - Encoding
- `rand` - Random number generation
- `serde` + `serde_yaml` - YAML parsing
- `serde` + `serde_yaml` - YAML parsing

## Säkerhet

⚠️ **VIKTIGT:**
- Lösenordet lagras INTE
- Varje kryptering använder nya slumpmässiga salt och nonce
- Fel lösenord ger dekrypteringsfel
- Ingen bruteforce-skydd implementerat (använd starka lösenord!)
