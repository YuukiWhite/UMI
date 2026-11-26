# Product Requirements Document (PRD) — UMI Dynamic Island

**Versi:** 2.0  
**Tanggal:** 26 November 2026  
**Target Platform:** Fedora Linux 44 — KDE Plasma 6.8+ (Wayland / KWin), Wayland-first dengan abstraksi modular  
**Tech Stack:** Pure Rust (`gtk4-rs` + `gtk4-layer-shell` + `tokio` + `zbus` + `rusqlite` + `oo7` + `ashpd`)  
**Model Produk:** Aplikasi personal / open penuh (seluruh 18 state & 7 halaman Settings terbuka tanpa paywall / tier Pro)  
**Referensi Desain:** [design.figcraft](../design.figcraft) (4 halaman: `Dynamic Island Showcase`, `All 18 Notch States`, `UMI Logo`, `Settings Feature`)

---

## 1. Ringkasan Produk

**UMI** (*Ambient Notch Companion*) adalah aplikasi utilitas desktop *always-on-top* berbasis **Rust** berbentuk *Dynamic Island / notch* di tepi atas tengah layar untuk **Fedora KDE Plasma (Wayland)**. UMI menyatukan kontrol media & lirik tersinkronisasi (Spotify, YouTube, YouTube Music), status sistem Linux, utilitas produktivitas harian, asisten **Ask UMI AI** + **Smart OCR**, serta pemantau sisa kuota langganan AI (**Antigravity, Codex, Claude**, dan provider kustom) dalam satu permukaan tepi layar yang hemat RAM dan halus di `60 fps`.

### Tujuan Utama
1. Mewujudkan seluruh 4 halaman desain pada [design.figcraft](../design.figcraft) secara native di **KWin Wayland** menggunakan Pure Rust (`gtk4-rs` + `gtk4-layer-shell`) dengan konsumsi memori ringan (`~25–45 MB` RAM) dan CPU idle `< 1%`.
2. Mengintegrasikan subsistem Linux/KDE secara aman (MPRIS2, UPower, BlueZ, Klipper, Notifikasi, XDG Portals, KWallet) tanpa keylogger global dan tanpa konflik dengan `plasmashell`.
3. Menyediakan 18 state notch interaktif, jendela Settings 7 kategori, System Tray quick-menu, serta pengaturan cepat langsung di dalam notch.
4. Membuka seluruh fitur (termasuk AI, Smart OCR, AI Limits, Widgets, dan gaya Glass/Floating) secara penuh tanpa *paywall*.

---

## 2. Target Lingkungan & Kelayakan Teknis

Berdasarkan verifikasi lingkungan target saat ini:
- **OS & Session:** Fedora Linux 44 (`x86_64`), `XDG_SESSION_TYPE=wayland`, `XDG_CURRENT_DESKTOP=KDE`
- **Compositor:** KWin Wayland (`plasmashell 6.8.3`)
- **Paket Sistem Terverifikasi:**
  - `gtk4` (`4.22.5`) — runtime UI native Wayland
  - `xdg-desktop-portal-kde` (`6.8.3`) — dukungan `GlobalShortcuts` dan `Screenshot` portal resmi Wayland
  - `kf6-kwallet` (`6.32.0`) — penyimpanan rahasia terenkripsi (`org.freedesktop.secrets`)
  - `pipewire` (`1.6.11`) — subsistem audio untuk peak meter EQ
  - `sqlite-libs` (`3.51.2`) — basis data lokal
  - `mesa-vulkan-drivers` (`26.2.7`) — akselerasi GPU
- **Protokol Overlay:** `zwlr_layer_shell_v1` via **`gtk4-layer-shell`** (`gtk4-rs`):
  - `Edge::Top` terpusat horizontal, `Layer::Overlay` (atau `Layer::Top` dengan *fullscreen awareness*)
  - `auto_exclusive_zone(-1)` agar tidak memotong area kerja / *tiling* maupun panel KDE
  - `KeyboardMode::None` saat pasif, beralih ke `KeyboardMode::OnDemand` hanya saat mengetik (`Ask UMI AI` / `Add Task`)
  - Dynamic Wayland input region (`cairo::Region`) yang diperbarui mengikuti bounding box notch + concave ears agar area transparan di luar notch tetap tembus klik (*click-through*)
- **Model Runtime:** **Single Binary Multi-Window (`umi`)** — satu proses utama yang mengelola Notch layer-shell window, Settings window *on-demand*, ikon System Tray (`StatusNotifierItem`), serta endpoint CLI/D-Bus IPC (`umi toggle`, `umi ask`, `umi ocr`, `umi settings`).

---

## 3. Identitas Brand & Design System

Seluruh token visual dan aset merujuk langsung pada [design.figcraft](../design.figcraft).

### 3.1 Brand Logo UMI (`Page UMI Logo`)
- **Bentuk Mark:** **Tapered Notch / Dynamic Island** murni — bar tipis tepi atas yang melengkung *concave* ke bawah membentuk kapsul huruf **"U"** (tanpa telinga hewan dan tanpa antena robot).
- **Anatomi Wajah Maskot:** Sepasang mata *squircle* putih (`#FFFFFF`) dengan glint biru langit (`#7DD3FC`) dan *micro-smile* halus (`#FFFFFF99`).
- **Varian Ukuran & Tile:**
  - **Light App Tile (`256×256`, `72×72`, `56×56`, `26×26`, `20×20`, `18×18`):** tile rounded-square gradasi `#F6F9FF -> #DCE6FF` (`cornerRadius = 25%`), kepala `#05070E` berproporsi `0.76 × 0.49` tile dengan rim gradasi `#38BDF8 -> #6366F1 -> #A855F7`.
  - **Dark Emblem (`256×256`):** mark standalone `#000000` dengan rim menyala `#19B8FF -> #6366F1 -> #7B4BFF` untuk splash/dark surfaces.
  - **Wordmark:** `UMI` (*Inter Display Bold / ExtraBold*, `#ECEFF7`), tagline `Ambient Notch Companion` (`#7DD3FC`).

### 3.2 Skala & 4 Tier Dimensi Notch
Ukuran dasar (`dp`) diskalakan oleh pengaturan `Notch Display Scale` (default `1.0×`, showcase `1.3×`):

| Tier | State | Ukuran Dasar (`dp`) | Skala `1.3×` (`px`) | Bottom Radius (`dp` / `px`) | Concave Ear `--e` (`dp` / `px`) |
|---|---|---:|---:|---:|---:|
| **Compact** | `01 Resting`, `02 Closed` | `150 × 30` | `195 × 39` | `14` / `18.2` | `10` / `13` |
| **Peek / Alert** | `03 Notif`, `04 Headphones`, `05 Charging`, `18 QuotaLow` | `310 × 60` | `403 × 78` | `19` / `24.7` | `13` / `17` |
| **Expanded** | `06 Music`, `07 Focus`, `09 Prayer`, `10 Water`, `11 Shelf`, `12 Clip`, `13 Todo`, `14 Widgets`, `15 AI Prompt`, `16 AI Answer`, `17 AI Limits`, `In-Notch Settings` | `420 × 140` | `546 × 182` | `32` / `41.6` | `22` / `28.6` |
| **Wide** | `08 Lyrics` | `520 × 140` | `676 × 182` | `32` / `41.6` | `22` / `28.6` |

### 3.3 Warna, Efek & Tipografi
- **Surface Notch:** `#000000` (mode `Attached` / `Floating`) atau frosted glass translucent (mode `Glass`), drop shadow `0 14px 34px -16px rgba(0,0,0,0.95)`, sudut atas kiri & kanan menyatu ke tepi layar lewat *concave quarter-circle ears*.
- **Palet Utama & Aksen:**
  - Canvas / Panel: `#03040A`, `#070912`, `#0A0D18`, `#0C101E`, Border `rgba(236, 239, 247, 0.10)`
  - Brand Aura: `linear-gradient(90deg, #38BDF8, #6366F1 50%, #A855F7)`
  - Battery / Charging / Todo / Codex: `#2EDB6B`, `#2FD06B`, `#33E07A`, `#10B981`
  - Media / Water / Antigravity: `#5EC2FF`, `#56C8FF`, `#2699FF`, `#38BDF8`, `#0E4A86`
  - Focus Timer: `#5B6CFF -> #A855F7` (break: `#22D3EE -> #34D399`)
  - Prayer / Claude Low Quota: `#F59E0B`, `#FBBF24`, `#EF4444`
- **Tipografi:** `Inter` & `Inter Display` (atau sans-serif sistem) dengan `tabular-nums` untuk jam, timer, persentase, dan reset countdown.
- **Animasi:** Damped spring physics setara kurva `cubic-bezier(0.34, 1.32, 0.5, 1)` (`~620ms` expand, `~500ms` shrink, `~450ms` radius morph) dengan preset kecepatan `Snappy`, `Balanced`, dan `Calm`.

---

## 4. Kebutuhan Fungsional — 18 State Notch

### 4.1 Komponen Bersama Mode Expanded
Kecuali `08 Lyrics` dan `16 Ask UMI AI Answer`, seluruh mode *Expanded* menggunakan shell konsisten:
- **Header:** Judul modul di kiri (`x: 24`), ring baterai hijau + tanggal & jam (`Wed 26 Nov 2026 14:18`) di tengah (dapat disembunyikan via Settings), serta 5-bar live equalizer putih di kanan (atau badge status kontekstual seperti `3 active` di AI Limits / switcher `Shelf | Clipboard`).
- **Navigasi Halaman:** Tombol panah kiri/kanan (`22×22px`), *dot indicator* halaman aktif di bawah (titik aktif memanjang menjadi kapsul putih `15px`), serta dukungan *scroll wheel* / swipe horizontal. Urutan dan visibilitas halaman dapat diatur pengguna di `Settings -> Notch Pages`.

### 4.2 Spesifikasi Lengkap 18 State

| # | State | Tier | Pemicu / Perilaku | Elemen UI & Integrasi |
|---:|---|---|---|---|
| 1 | **Resting (`pet`)** | Compact | Default saat idle tanpa media/alert aktif (jika `Idle Mascot Eyes` ON). | Ambient glow `#38BDF8 -> #6366F1 -> #A855F7` di tepi bawah + mata squircle putih dengan sky glint `#7DD3FC`, *micro-smile* `#FFFFFF88`, dan animasi kedip berkala. |
| 2 | **Closed (`idle`)** | Compact | Saat notch tertutup namun jam/media ringkas aktif (atau `Minimal Notch Mode` menyusutkan menjadi garis bezel tipis). | Ring baterai hijau di kiri, jam bold (`07:25`) di tengah, 5-bar EQ animatif di kanan saat audio bermain. |
| 3 | **Notification (`notif`)** | Peek | Notifikasi desktop baru masuk via D-Bus monitor (auto-dismiss setelah `N` detik, klik untuk fokus aplikasi). | Ikon aplikasi (`32×32`, mis. `#25D366` Messages), nama aplikasi, pengirim (`Farid`), isi singkat 1 baris, timestamp (`now`). |
| 4 | **Headphones (`bt`)** | Peek | Event koneksi Bluetooth audio (`org.bluez.Device1`). | Ring baterai `#5EC2FF` + ikon Bluetooth, nama perangkat (`AirBuds`) + `Connected · 80%`, badge lingkaran headphone di kanan. |
| 5 | **Charging (`charge`)** | Peek | Event AC adapter tersambung (`org.freedesktop.UPower`). | Ring hijau `#33E07A` + ikon petir putih, `Charging / Charger connected`, persentase besar (`84%`) di kanan. |
| 6 | **Music (`music`)** | Expanded | Hover notch saat media aktif atau dari pager. | Integrasi **MPRIS2** (Spotify, YouTube/YouTube Music di browser, player lokal): album art (`62×62`), judul (`Blue Horizon`), artis (`Summer Coast`), progress bar seek interaktif, waktu elapsed/remaining, tombol `Prev / Pause-Play / Next`, dan tombol ikon Lyrics. |
| 7 | **Focus Timer (`pomo`)** | Expanded | Dipilih dari pager atau saat sesi Pomodoro aktif. | Label sesi (`Focus · 2 of 4`), countdown besar (`18:42`), estimasi selesai (`10:27 until done`), progress bar ungu-indigo, tombol `Pause/Resume` & `Reset`, timeline 8-segmen (`1 2 3 4 Long`) dengan *glowing head*, ringkasan durasi (`2 h 10 min · 4 sessions`). |
| 8 | **Lyrics (`lyrics`)** | Wide | Klik ikon Lyrics di mode Music. | Notch melebar ke `520×140 dp`, backdrop ambient biru dinamis, panel kiri kontrol musik ringkas + progress bar, divider vertikal, panel kanan 3+ baris lirik tersinkronisasi (baris aktif putih terang, baris lain redup). Otomatis fetch **LRCLIB** cloud API untuk Spotify, YouTube, & YouTube Music + cache lokal. Klik baris untuk seek lagu. |
| 9 | **Prayer (`prayer`)** | Expanded | Dipilih dari pager atau pre-prayer nudge mendekat. | Kota (`Bandung`), label `Next` + nama sholat (`Ashar`), jam & countdown (`15:18 in 42 min`), progress bar biru-ungu, grafik **Sun Arc** 5 waktu (`Subuh`, `Dzuhur`, `Ashar`, `Maghrib`, `Isya`) dengan posisi matahari bercahaya. Hitungan 100% offline (Kemenag RI / metode lain) + opsi *auto-pause* musik saat masuk waktu sholat. |
| 10 | **Water (`water`)** | Expanded | Dipilih dari pager atau pengingat minum berkala. | Total konsumsi (`1.5 / 1.5 L`), toggle reminder aktif, `Today · 100% · Goal reached`, progress bar biru, tombol `+Drink` (tambah sesuai `Bottle Size`) & `Settings` (membuka **In-Notch Water Settings**), serta chart batang per jam (`09–15`, skala `0–400 ml`). |
| 11 | **Shelf (`shelf`)** | Expanded | Drag file dari file manager ke atas tepi layar (auto-expand) atau dari pager. | Tombol `Clear`, drop-zone garis putus-putus (`Drop files here / Drag out anytime · click to open`), kartu file tersimpan (`preview.fig · 4.8 MB` + thumbnail). Menyimpan URI/path file (tidak menggandakan isi file ke RAM), mendukung *drag-out* `text/uri-list` & klik buka via `xdg-open`. |
| 12 | **Clipboard (`clip`)** | Expanded | Toggle switcher `Shelf | Clipboard` di kanan atas view Shelf. | Switcher pill `#5C54FF`, 3 kartu histori clipboard terbaru (teks + usia relatif seperti `2 min ago`), footer `4 items · click to copy`. Sinkron real-time via `org.kde.klipper` dengan pengecualian secret/password. |
| 13 | **To-do (`todo`)** | Expanded | Dipilih dari pager atau pengingat tenggat tugas. | Counter `4/5 Task Done Today`, tombol `Add Task` (input inline) & `Edit Task`, daftar task scrollable berlatar hijau transparan, ikon centang `#2FD06B`, teks *strikethrough*, serta opsi sinkronisasi acara dari Google Calendar iCal. |
| 14 | **Widgets (`widgets`)** | Expanded | Dipilih dari pager. | 2 slot widget berdampingan yang dapat dipilih pengguna: default `THIS WEEK` (kalender `M–S`, lingkaran `#0A84FF` hari ini, dot task berwarna, acara berikutnya) dan `BATTERY` (ring `#30C759`, petir, `82% Charging / Plugged in`), plus opsi Weather (MET Norway), World Clock, Countdown, Next Prayer. |
| 15 | **Ask UMI AI — Prompt (`ai_prompt`)** | Expanded | Shortcut global `Ctrl + Alt + Space` (via XDG Portal / `umi ask`) atau dari pager. | Header `Ask UMI AI` (tanpa badge provider statis), input box dengan border gradasi `#38BDF8 -> #6366F1 -> #A855F7`, kursor aktif, tombol kirim `↑`, 4 quick chips (`Summarize`, `Translate · Tab`, `OCR · Ctrl+Alt+T`, `100usd to idr=`), dan hint footer. Mendukung **Quick Math & Currency** lokal instan di dalam input box. |
| 16 | **Ask UMI AI — Answer & Translate (`ai_answer`)** | Expanded | Setelah mengirim prompt di Notch 15, menekan `Tab` (Translate), atau selesai **Smart OCR (`Ctrl + Alt + T`)**. | Switcher mode `Ask | Translate`, echo prompt pengguna, kartu jawaban scrollable (mendukung bullet/teks hasil OCR), tombol `Copy`, `Retry`, `Ask follow-up...`, serta metadata latency (`UMI AI · 0.4s`). |
| 17 | **AI Limits — Subscription Quota (`ai_limits`)** | Expanded | Dipilih dari pager atau klik alert kuota rendah. | Header `AI Limits` sejajar `x: 24` (tanpa ikon gauge) + pill status `3 active`, 3 kartu berdampingan: **Antigravity** (`Pro · 5h window`, `78% left`, bar `#38BDF8 -> #6366F1`, `Resets in 2h 15m`), **Codex** (`Plus · Weekly`, `42% left`, bar `#10B981 -> #2DD4BF`, `Resets in 3d 4h`), **Claude** (`Max · Session`, `18% low`, bar `#F59E0B -> #EF4444`, `Resets in 48m`). Mendukung built-in read-only adapters + `ScriptProvider` kustom (JSON stdout). |
| 18 | **AI Limits — Low Quota Peek Alert (`ai_quota_low`)** | Peek | Pemicu otomatis saat sisa kuota salah satu provider turun di bawah ambang batas (default `< 20%`). | Ring peringatan amber `#F59E0B` (`18%` arc + `!`), judul `Claude · Quota Low`, subjudul `18% remaining · Resets in 48m`, persentase besar `18%` di kanan. Klik alert membuka Notch 17. |

---

## 5. Kebutuhan Fungsional — Settings, System Tray & In-Notch Settings

Merujuk langsung pada `Page Settings Feature` di [design.figcraft](../design.figcraft):

### 5.1 Desktop Settings Window (`860 × 604`)
- **Window Chrome:** Titlebar gelap memakai ikon **UMI Tapered Notch (`20×20`)** + judul `UMI Settings` di kiri, serta kontrol window minimal (`—`, `□`, `×`) di kanan.
- **Colorful Sidebar (`224px`):** Logo UMI (`26×26`) + versi `v1.3`, 7 item navigasi dengan ikon badge rounded-square (`22×22`) warna-warni gradasi, dan footer privasi `Stored locally · No tracking`:
  1. **General** (`#38BDF8 -> #2563EB`): Live Notch Preview (`Resting | Closed | Minimal`), toggle `Start at Login` (`~/.config/autostart/`), `Idle Mascot Eyes`, `Minimal Notch Mode`, `Show Date & Clock in Header`, pemilih cepat `Notch Style`, chip `Active Pages`, dan ringkasan shortcut.
  2. **Appearance** (`#EC4899 -> #8B5CF6`): Visual picker `Notch Style` (`Attached`, `Floating`, `Glass`), 6 swatch `Accent Aura`, pilihan `Mascot Eye Tint` (`Sky Glint`, `Mint`, `Rose`, `White`), slider `Glass Background Blur` (`px`) & `Notch Display Scale` (`%`), toggle `Smooth Concave Fillets`, preset `Spring Animation Speed` (`Snappy | Balanced | Calm`), dan `Dim Screen Behind Expanded Notch`.
  3. **Notch Pages** (`#818CF8 -> #4F46E5`): Daftar 7+ modul dengan *drag handle* (`⋮⋮`), indikator warna modul, toggle aktif/nonaktif per halaman, dan tombol `Reset Default`.
  4. **Focus & Water** (`#2DD4BF -> #0EA5E9`): Pengaturan Pomodoro (`Focus Session Length`, `Short / Long Break`, `Sessions per Round`, `Auto-start Next Session`) dan Hydration (`Daily Hydration Goal`, `Bottle / Cup Size`, `Active Reminder Hours`, `Gentle Reminder Sound`).
  5. **Prayer Times** (`#FBBF24 -> #EA580C`): Preview 5 jadwal harian lokal, pemilih `Location Source` (`My Location (~1 km)` / `Pick City`), dropdown kota & `Calculation Method` (`Kemenag RI`, MWL, ISNA, dll.), `Pre-Prayer Nudge` (`5/10/15 min`), toggle `Auto-Pause Music on Adzan`, dan `Show Sun Arc Countdown`.
  6. **AI & Shortcuts** (`#A855F7 -> #38BDF8`): Input **API Key** multi-provider (Gemini untuk Ask UMI AI + Smart OCR, plus Claude / OpenAI-compatible / Ollama lokal) dengan tombol verifikasi & penyimpanan aman di KWallet, manajemen **AI Quota Providers** (Antigravity, Codex, Claude + folder script kustom `~/.config/umi/quota-scripts/`), input `Google Calendar iCal`, dan tabel shortcut global (`Ask UMI Overlay`, `Translate Mode`, `Smart Screen OCR`, `Quick Math & Currency`, `Open UMI Settings`).
  7. **About & Data** (`#34D399 -> #059669`): Kartu versi `UMI Desktop` + logo `72×72` + `Check for Updates`, ringkasan penyimpanan lokal offline (To-do, Hydration Log, Shelf Paths, Clipboard, Quota Cache), aksi `Export Settings JSON`, `Open Data Folder`, `Reset All Data`, serta kredit sumber data (`LRCLIB`, `MET Norway`).

### 5.2 System Tray Quick Menu (`SNI / ksni`)
Klik kanan ikon UMI (`18×18`) di system tray KDE membuka menu ringkas: header `UMI · Active`, toggle cepat `Start at Login`, `UMI Mascot`, `Minimal notch`, lalu `Settings... (Ctrl+,)`, `About UMI`, dan `Exit`.

### 5.3 In-Notch Quick Settings (`Water Settings`)
Tombol `Settings` di dalam `Notch 10 · Water` membuka panel pengaturan ringkas langsung di dalam notch (`Daily Goal` stepper, `Bottle Size` stepper, interval `45m | 60m | 90m`, dan tombol `Open Full Settings →`).

---

## 6. Spesifikasi Khusus AI, Lyrics, Smart OCR & Integrasi KDE Wayland

### 6.1 Synced Lyrics Otomatis (Spotify, YouTube, YouTube Music)
- Mendengarkan sinyal D-Bus **MPRIS2** dari seluruh player aktif (`spotify`, `firefox`, `chromium`, `brave`, `youtube-music`, `vlc`, dll.).
- **Title Sanitizer:** Sebelum memanggil **LRCLIB API** (`https://lrclib.net/api/get` dengan fallback `/api/search`), judul & artis dari YouTube / YouTube Music dibersihkan dari noise (`(Official Video)`, `[MV]`, `(Lyrics)`, `| ...`, `- Topic`, `ft./feat.`).
- **Sinkronisasi & Cache:** Lirik `.lrc` yang ditemukan disimpan ke tabel `lyrics_cache` di SQLite (`~/.local/share/umi/umi.db`) agar pemutaran ulang instan dan tetap jalan saat offline. Baris aktif disinkronkan real-time terhadap MPRIS2 `Position` dengan interpolasi timer lokal `100ms`.

### 6.2 Smart OCR via Gemini Vision (`Ctrl + Alt + T`)
- Menggunakan **XDG Screenshot Portal** (`org.freedesktop.portal.Screenshot` via `ashpd`) agar pengguna dapat memilih area layar secara native di KDE Wayland tanpa izin khusus.
- Gambar hasil tangkapan dikirim langsung ke **Gemini Vision API** menggunakan Gemini API key milik pengguna yang tersimpan di KWallet.
- Teks/kode/rumus hasil ekstraksi langsung ditampilkan di `Notch 16 (Ask UMI AI — Answer)` dan otomatis disalin ke clipboard.

### 6.3 AI Limits (`Antigravity, Codex, Claude` + Script Kustom)
- Setiap provider mengimplementasikan trait `QuotaProvider` di Rust dengan interval polling hemat daya (mis. tiap `60–120 detik` dan *on-demand* saat notch dibuka).
- **Built-in Read-Only Adapters:** Membaca informasi kuota/sesi lokal secara *read-only* dan aman untuk Antigravity, Codex, dan Claude (bila tersedia di mesin pengguna), dengan *graceful degradation* ke status `Unavailable / Manual` jika belum login.
- **Generic `ScriptProvider`:** Menjalankan skrip opsional di `~/.config/umi/quota-scripts/*.sh` (dengan timeout ketat `3 detik`) yang mengembalikan JSON standar `{provider, plan, remaining_pct, resets_in_sec, status}` sehingga pengguna dapat menambahkan provider AI lain tanpa kompilasi ulang.

### 6.4 Integrasi Sistem KDE Plasma 6 (Wayland)

| Subsistem | Mekanisme Linux / KDE (Rust) | Catatan |
|---|---|---|
| **Overlay & DnD** | `gtk4-rs` + `gtk4-layer-shell` + `wl_data_device` | Click-through via `cairo::Region`, drag-in & drag-out URI file untuk Shelf. |
| **Media** | `zbus` → `org.mpris.MediaPlayer2.Player` | Multi-player tracking + auto-pause saat Adzan. |
| **Baterai & Charger** | `zbus` → `org.freedesktop.UPower` | Memicu `Notch 05 (Charging)` & `Widgets (Battery)`. |
| **Bluetooth Audio** | `zbus` → `org.bluez` (`Device1`, `Battery1`) | Memicu `Notch 04 (Headphones)`. |
| **Clipboard** | `zbus` → `org.kde.klipper` | Sinkron latar belakang Wayland + abaikan hint password (`x-kde-passwordManagerHint`). |
| **Notifikasi** | `zbus` → D-Bus Passive Monitor (`BecomeMonitor`) | Tidak bentrok dengan `plasmashell`; opsi Replace Mode di Settings. |
| **Global Hotkeys & OCR** | `ashpd` → `GlobalShortcuts` & `Screenshot` Portal | Native KDE Wayland, tanpa keylogger/`evdev`. Fallback CLI `umi ask` / `umi ocr`. |
| **Secret Storage** | `oo7` → KWallet (`org.freedesktop.secrets`) | Enkripsi API key Gemini/Claude/OpenAI di KDE Wallet. |
| **Audio EQ** | PipeWire lightweight peak monitor / fallback MPRIS | Aktif hanya saat `PlaybackStatus == Playing` agar CPU idle `< 1%`. |

---

## 7. Kebutuhan Non-Fungsional

1. **Performa & Efisiensi:** CPU idle (`Resting`) `< 1%`, target RAM `< 45 MB`, animasi spring stabil `60 fps` via GTK4/GSK GPU renderer.
2. **Privasi & Offline-First:** Tidak ada akun UMI, tidak ada telemetri. Config di `~/.config/umi/config.toml`, data lokal di `~/.local/share/umi/umi.db`, API key terenkripsi di KWallet. Permintaan jaringan hanya keluar saat pengguna memakai fitur online (LRCLIB, Ask UMI AI / Gemini Vision OCR, Weather MET Norway, atau iCal sync).
3. **Multi-Monitor & Fractional Scaling:** Mendukung pemilihan monitor target (`gdk4::Display::monitors`) serta fractional scaling Wayland KDE.
4. **Fullscreen Awareness:** Opsi menyembunyikan atau menciutkan notch saat aplikasi fullscreen aktif + shortcut cepat `Meta+N` (`umi toggle`).
5. **Persistensi & Portabilitas:** Ekspor/impor pengaturan JSON dari `Settings -> About & Data` dan penyimpanan data lokal yang bertahan saat update aplikasi.

---

## 8. Roadmap Implementasi (5 Fase)

1. **Fase 1 — Core Rust, Layer-Shell & Visual Motion:** Setup Cargo (`gtk4-rs`, `gtk4-layer-shell`, `tokio`), window transparan, concave ears, dynamic `cairo::Region`, damped spring 60fps, state `01 Resting` & `02 Closed`, kerangka pager, SNI Tray (`ksni`), dan CLI/D-Bus IPC.
2. **Fase 2 — Integrasi Sistem KDE Wayland:** `zbus` adapters untuk MPRIS2 (`06 Music`), LRCLIB + YouTube/YTM sanitizer (`08 Lyrics`), UPower (`05 Charging`, `14 Battery`), BlueZ (`04 Headphones`), Klipper (`12 Clipboard`), dan D-Bus passive Notification listener (`03 Notification`).
3. **Fase 3 — Produktivitas & Utilitas Lokal:** `07 Focus Timer` (Pomodoro 8-segmen), `09 Prayer` (kalkulator astronomi offline + Sun Arc Cairo canvas + auto-pause media), `10 Water` (chart per jam + In-Notch Settings), `11 Shelf` (Wayland DnD drag-in/out), `13 To-do` + iCal, dan `14 Widgets`.
4. **Fase 4 — Ask UMI AI, Smart OCR & AI Limits:** Integrasi KWallet (`oo7`), `ashpd` (`GlobalShortcuts` + `Screenshot`), `15/16 Ask UMI AI` (multi-provider BYOK + Translate + Quick Math + Gemini Vision OCR), serta `17/18 AI Limits` (Antigravity, Codex, Claude adapters + `ScriptProvider` + alert `< 20%`).
5. **Fase 5 — Settings 7 Screen, Polish & Packaging:** Implementasi lengkap 7 screen Settings dengan sidebar warna-warni, Live Preview, autostart `.desktop`, pengujian multi-monitor/sleep-wake/fullscreen di Fedora 44 KDE Wayland, dan packaging RPM / binary rilis.

---

## 9. Risiko Teknis & Mitigasi

| Risiko | Dampak | Mitigasi |
|---|---|---|
| **Konflik kepemilikan `org.freedesktop.Notifications` dengan `plasmashell`** | Gagal mendaftar D-Bus atau mematikan notifikasi bawaan KDE. | Gunakan D-Bus `BecomeMonitor` passive observer sebagai default; Replace Mode hanya jika diaktifkan manual. |
| **Judul YouTube / YouTube Music tidak cocok dengan database LRCLIB** | Synced lyrics gagal ditemukan untuk video musik. | Terapkan *title sanitizer* berlapis (hapus tag `[Official Video]`, `- Topic`, durasi toleransi `±5 detik`) + fallback `/api/search` + edit manual query. |
| **Perubahan format session/CLI lokal untuk `AI Limits` (Antigravity/Codex/Claude)** | Built-in adapter gagal membaca sisa kuota setelah update tools eksternal. | Isolasi tiap provider di balik trait `QuotaProvider` dengan *graceful fallback* + sediakan `ScriptProvider` JSON agar bisa diperbaiki cepat tanpa recompile. |
| **Area transparan memblokir klik jendela di bawah notch** | Tab browser / toolbar atas tidak dapat diklik. | Perbarui `cairo::Region` input mask pada setiap frame animasi spring sesuai bounding box aktual notch + concave ears. |
| **Latensi atau kegagalan jaringan saat Smart OCR (Gemini Vision)** | Pengguna menunggu terlalu lama atau offline. | Tampilkan indikator loading animatif di notch dengan timeout terkontrol, error toast yang jelas, serta tombol `Retry`. |
| **Notch mengganggu video/game fullscreen** | Menutupi HUD atau video di tepi atas layar. | Sediakan deteksi fullscreen KWin + shortcut global toggle (`Meta+N` / `umi toggle`). |
