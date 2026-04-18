# Setup Guide

> **English** | [فارسی](#راهنمای-نصب-و-راه‌اندازی)

---

## Quick Start for Non-Experts: --wizard

If you don't want to manually edit JSON, use the interactive wizard:

```bash
sudo ./sni-spoof --wizard
```

It will ask you:
1. **Upstream server IP:port** — e.g., `1.2.3.4:443`
2. **Local listen port** — defaults to `40443`
3. **SNI pool** — choose from hCaptcha (recommended), Cloudflare, or custom
4. **Enable DPI evasion** — enable fragmentation + payload padding

Then generates `config.json` and exits. Run again with your config.

---

## Quick Start for Scripts: --preset

Pre-configured templates for common use cases:

```bash
# hCaptcha pool (6 SNIs, no fragmentation)
./sni-spoof --preset hcaptcha

# Cloudflare pool (4 SNIs, no fragmentation)
./sni-spoof --preset cloudflare

# Stealth mode (hCaptcha pool + fragmentation enabled + padding)
./sni-spoof --preset stealth
```

Presets generate `config.json` with placeholder `UPSTREAM_IP`. Edit the file and replace it with your actual upstream IP, then run.

---

## Option 1: Download Prebuilt Binaries (Recommended)

Download the latest release from the [Releases](https://github.com/akonil/sni-spoofing-unified/releases) page.

### Linux

```bash
# Download and extract
wget https://github.com/akonil/sni-spoofing-unified/releases/latest/download/sni-spoof-linux-x64.tar.gz
tar xzf sni-spoof-linux-x64.tar.gz

# Edit config.json, then run
sudo ./sni-spoof-linux-x64 config.json
```

### macOS (Intel)

```bash
curl -L -o sni-spoof-macos-x64.tar.gz https://github.com/akonil/sni-spoofing-unified/releases/latest/download/sni-spoof-macos-x64.tar.gz
tar xzf sni-spoof-macos-x64.tar.gz
sudo ./sni-spoof-macos-x64 config.json
```

### macOS (Apple Silicon)

```bash
curl -L -o sni-spoof-macos-arm64.tar.gz https://github.com/akonil/sni-spoofing-unified/releases/latest/download/sni-spoof-macos-arm64.tar.gz
tar xzf sni-spoof-macos-arm64.tar.gz
sudo ./sni-spoof-macos-arm64 config.json
```

### Windows

1. Download `sni-spoof-windows-x64.zip` from [Releases](https://github.com/akonil/sni-spoofing-unified/releases)
2. Extract and edit `config.json`
3. Run as Administrator: `sni-spoof-windows-x64.exe config.json`

---

## Option 2: Build from Source

### Linux

### 1. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

### 2. Build

```bash
git clone <repo-url> sni-spoofing-unified
cd sni-spoofing-unified
cargo build --release
```

### 3. Configure

Edit `config.json`:

```json
{
  "debounce_logs": false,
  "jitter": {
    "min_ms": 1,
    "max_ms": 8
  },
  "timeouts": {
    "handshake_timeout_ms": 5000,
    "confirmation_timeout_ms": 2000
  },
  "listeners": [
    {
      "listen": "0.0.0.0:40443",
      "connect": "YOUR_SERVER_IP:443",
      "fake_sni_pool": ["www.speedtest.net", "www.google.com"],
      "max_connections_per_sec": 0,
      "gaming_mode": false
    }
  ],
  "advanced": {
    "payload_padding": {
      "min_extra_bytes": 0,
      "max_extra_bytes": 128
    },
    "fragmentation": {
      "enabled": false,
      "fragments": 2,
      "delay_ms": 1
    }
  }
}
```

Replace `YOUR_SERVER_IP` with your actual upstream server IP.

**Configuration notes:**

- **Jitter** (default 1–8 ms) adds random delay before sending the fake ClientHello. Disables timing-based DPI detection. Set `max_ms: 0` to disable.
- **fake_sni_pool**: Array of decoy domains. One is chosen randomly per connection. Backward compatible: if using old `fake_sni` field, it's automatically wrapped into a pool.
- **max_connections_per_sec**: Rate limit (0 = unlimited). Useful to prevent connection floods.
- **Payload Padding** (advanced): Adds 0-128 random bytes to vary fake ClientHello size, defeating fixed-size fingerprinting.
- **Fragmentation** (advanced): Splits fake ClientHello into 2-3 TCP segments, confusing DPI reassembly. Set `enabled: true` for stricter DPI.

### 4. Run

```bash
# Option A: using sudo
sudo ./target/release/sni-spoof config.json

# Option B: grant capability so you don't need sudo every time
sudo setcap cap_net_raw+ep ./target/release/sni-spoof
./target/release/sni-spoof config.json
```

### 5. Run as a systemd service (optional)

Create `/etc/systemd/system/sni-spoof.service`:

```ini
[Unit]
Description=SNI Spoof Proxy
After=network.target

[Service]
ExecStart=/path/to/sni-spoof /path/to/config.json
Restart=on-failure
Environment=RUST_LOG=warn
AmbientCapabilities=CAP_NET_RAW

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now sni-spoof
sudo systemctl status sni-spoof
```

---

## macOS

### 1. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

### 2. Build

```bash
git clone <repo-url> sni-spoofing-unified
cd sni-spoofing-unified
cargo build --release
```

### 3. Configure

Edit `config.json` (same format as Linux above).

### 4. Run

macOS uses BPF devices for packet capture, which requires root:

```bash
sudo ./target/release/sni-spoof config.json
```

**Troubleshooting:** If you see `failed to open BPF device`, make sure:
- You are running with `sudo`
- No other packet capture tool is using all BPF devices (e.g. Wireshark)

---

## Windows

### 1. Install Rust

Download and run the installer from [rustup.rs](https://rustup.rs).  
Also install [Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) if prompted.

### 2. Install WinDivert

WinDivert is bundled and compiled automatically by the build. No manual install needed.

### 3. Build

Open **Command Prompt** or **PowerShell**:

```cmd
git clone <repo-url> sni-spoofing-unified
cd sni-spoofing-unified
cargo build --release
```

### 4. Configure

Edit `config.json` (same format as above).

### 5. Run

Right-click Command Prompt → **Run as administrator**, then:

```cmd
.\target\release\sni-spoof.exe config.json
```

---

## Verifying It Works

1. Start the proxy — you should see a log line like:
   ```
   listener started listen=0.0.0.0:40443 upstream=... sni_pool=[...]
   ```

2. Point your client (VPN app, browser proxy, etc.) at `127.0.0.1:40443`.

3. Enable `info` logs to see connections being handled and SNI stats:
   ```bash
   RUST_LOG=info sudo ./target/release/sni-spoof config.json
   ```

When running with multiple SNIs, the proxy logs success/failure per SNI every 60 seconds:
```
INFO SNI stats (top 10 by success):
INFO   www.speedtest.net → ok=142 fail=3
INFO   www.google.com → ok=98 fail=1
```

This helps you identify which SNIs work best in your region.

---

## Troubleshooting

| Problem | Cause | Fix |
|---------|-------|-----|
| `failed to open raw socket` (Linux) | Missing permissions | Run with `sudo` or set `CAP_NET_RAW` |
| `failed to open BPF device` (macOS) | Not running as root | Run with `sudo` |
| `failed to open WinDivert` (Windows) | Not Administrator | Run cmd as Administrator |
| `could not determine local IP` | No route to upstream IP | Check your network / upstream IP in config |
| `timeout waiting for fake ACK` | Upstream unreachable or DPI blocking | Try a different `fake_sni` domain |
| Binary not found | Build not run | Run `cargo build --release` first |

---

## Quick Start Scripts

The project includes ready-to-use launcher scripts for all platforms:

### Linux / macOS

The `run.sh` script handles building, permissions, and launching:

```bash
# Use default config.json
./run.sh

# Use a custom config
./run.sh /path/to/config.json

# Run with info logs
RUST_LOG=info ./run.sh config.json
```

The script automatically:
- Checks for binary; builds if missing (`cargo build --release`)
- Handles sudo permissions (prompts if needed)
- Sets default log level to `warn`

### Windows

The `run.bat` script handles building, admin check, and launching:

```cmd
REM Right-click Command Prompt, select "Run as administrator", then:
run.bat

REM Use custom config
run.bat C:\path\to\config.json

REM Run with info logs
set RUST_LOG=info
run.bat config.json
```

The script automatically:
- Checks for Administrator privileges
- Checks for binary; builds if missing (`cargo build --release`)
- Sets default log level to `warn`
- Shows pause prompt to view output

---

---

# راهنمای نصب و راه‌اندازی

> [English](#setup-guide) | **فارسی**

---

## گزینه ۱: دانلود فایل‌های از پیش ساخته‌شده (توصیه‌شده)

جدیدترین نسخه را از صفحه [Releases](https://github.com/akonil/sni-spoofing-unified/releases) دانلود کنید.

### لینوکس

```bash
# دانلود و استخراج
wget https://github.com/akonil/sni-spoofing-unified/releases/latest/download/sni-spoof-linux-x64.tar.gz
tar xzf sni-spoof-linux-x64.tar.gz

# فایل config.json را ویرایش کنید، سپس اجرا
sudo ./sni-spoof-linux-x64 config.json
```

### macOS (Intel)

```bash
curl -L -o sni-spoof-macos-x64.tar.gz https://github.com/akonil/sni-spoofing-unified/releases/latest/download/sni-spoof-macos-x64.tar.gz
tar xzf sni-spoof-macos-x64.tar.gz
sudo ./sni-spoof-macos-x64 config.json
```

### macOS (Apple Silicon)

```bash
curl -L -o sni-spoof-macos-arm64.tar.gz https://github.com/akonil/sni-spoofing-unified/releases/latest/download/sni-spoof-macos-arm64.tar.gz
tar xzf sni-spoof-macos-arm64.tar.gz
sudo ./sni-spoof-macos-arm64 config.json
```

### ویندوز

۱. `sni-spoof-windows-x64.zip` را از [Releases](https://github.com/akonil/sni-spoofing-unified/releases) دانلود کنید
۲. استخراج کرده و `config.json` را ویرایش کنید
۳. به عنوان Administrator اجرا کنید: `sni-spoof-windows-x64.exe config.json`

---

## گزینه ۲: ساخت از سورس

### لینوکس

### ۱. نصب Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

### ۲. ساخت (Build)

```bash
git clone <آدرس-ریپو> sni-spoofing-unified
cd sni-spoofing-unified
cargo build --release
```

### ۳. پیکربندی

فایل `config.json` را ویرایش کنید:

```json
{
  "debounce_logs": false,
  "jitter": {
    "min_ms": 1,
    "max_ms": 8
  },
  "timeouts": {
    "handshake_timeout_ms": 5000,
    "confirmation_timeout_ms": 2000
  },
  "listeners": [
    {
      "listen": "0.0.0.0:40443",
      "connect": "YOUR_SERVER_IP:443",
      "fake_sni": "www.speedtest.net",
      "gaming_mode": false
    }
  ]
}
```

> **جیتر (Jitter):** یک تأخیر تصادفی ۱ تا ۸ میلی‌ثانیه‌ای قبل از ارسال ClientHello جعلی اضافه می‌کند و تحلیل زمانی DPI را ناکام می‌کند. مقادیر پیش‌فرض را تغییر ندهید مگر دلیل خاصی داشته باشید. برای غیرفعال کردن: `"max_ms": 0`.
>
> **مهلت‌های اتصال (Timeouts):** `handshake_timeout_ms` (پیش‌فرض ۵۰۰۰ ms) مدت زمان انتظار برای ایجاد اتصال TCP را کنترل می‌کند. `confirmation_timeout_ms` (پیش‌فرض ۲۰۰۰ ms) مدت زمان انتظار برای تأیید تزریق پکت جعلی را کنترل می‌کند. اگر اتصالات شما کند یا ناپایدار است، این مقادیر را تنظیم کنید.

`YOUR_SERVER_IP` را با آی‌پی واقعی سرور مقصد خود جایگزین کنید.

### ۴. اجرا

```bash
# گزینه الف: با sudo
sudo ./target/release/sni-spoof config.json

# گزینه ب: اعطای capability تا دیگر نیازی به sudo نباشد
sudo setcap cap_net_raw+ep ./target/release/sni-spoof
./target/release/sni-spoof config.json
```

### ۵. اجرا به عنوان سرویس systemd (اختیاری)

فایل `/etc/systemd/system/sni-spoof.service` را بسازید:

```ini
[Unit]
Description=SNI Spoof Proxy
After=network.target

[Service]
ExecStart=/path/to/sni-spoof /path/to/config.json
Restart=on-failure
Environment=RUST_LOG=warn
AmbientCapabilities=CAP_NET_RAW

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now sni-spoof
sudo systemctl status sni-spoof
```

---

## macOS

### ۱. نصب Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

### ۲. ساخت

```bash
git clone <آدرس-ریپو> sni-spoofing-unified
cd sni-spoofing-unified
cargo build --release
```

### ۳. پیکربندی

فایل `config.json` را مثل مثال بالا ویرایش کنید.

### ۴. اجرا

macOS از BPF device برای ضبط پکت استفاده می‌کند که نیاز به root دارد:

```bash
sudo ./target/release/sni-spoof config.json
```

**رفع اشکال:** اگر خطای `failed to open BPF device` دیدید، مطمئن شوید:
- با `sudo` اجرا می‌کنید
- ابزار دیگری مثل Wireshark همه‌ی BPF device ها را اشغال نکرده باشد

---

## ویندوز

### ۱. نصب Rust

نصب‌کننده را از [rustup.rs](https://rustup.rs) دانلود و اجرا کنید.  
در صورت نیاز، [Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) را هم نصب کنید.

### ۲. WinDivert

WinDivert به صورت خودکار در زمان build کامپایل می‌شود. نیازی به نصب دستی نیست.

### ۳. ساخت

Command Prompt یا PowerShell را باز کنید:

```cmd
git clone <آدرس-ریپو> sni-spoofing-unified
cd sni-spoofing-unified
cargo build --release
```

### ۴. پیکربندی

فایل `config.json` را مثل مثال بالا ویرایش کنید.

### ۵. اجرا

روی Command Prompt راست‌کلیک کنید → **Run as administrator**، سپس:

```cmd
.\target\release\sni-spoof.exe config.json
```

---

## تأیید کارکرد صحیح

۱. پروکسی را اجرا کنید — باید پیامی مثل این ببینید:
   ```
   listener started listen=0.0.0.0:40443 upstream=... sni=...
   ```

۲. کلاینت خود (اپ VPN، پروکسی مرورگر و غیره) را به `127.0.0.1:40443` هدایت کنید.

۳. برای دیدن جزئیات اتصال‌ها، لاگ `info` را فعال کنید:
   ```bash
   RUST_LOG=info sudo ./target/release/sni-spoof config.json
   ```

---

## رفع اشکال (Troubleshooting)

| مشکل | علت | راه‌حل |
|------|-----|--------|
| `failed to open raw socket` (لینوکس) | عدم دسترسی | با `sudo` اجرا کنید یا `CAP_NET_RAW` بدهید |
| `failed to open BPF device` (macOS) | اجرا بدون root | با `sudo` اجرا کنید |
| `failed to open WinDivert` (ویندوز) | اجرا بدون Administrator | cmd را به عنوان Administrator باز کنید |
| `could not determine local IP` | مسیری به آی‌پی upstream وجود ندارد | شبکه و آی‌پی کانفیگ را بررسی کنید |
| `timeout waiting for fake ACK` | upstream در دسترس نیست یا DPI بلاک می‌کند | یک دامنه‌ی `fake_sni` دیگر امتحان کنید |
| فایل اجرایی پیدا نمی‌شود | build اجرا نشده | ابتدا `cargo build --release` بزنید |

---

## انتخاب fake_sni مناسب

مهم‌ترین تنظیم برای دور زدن فیلترینگ، انتخاب درست `fake_sni` است:

- **باید** یک دامنه‌ی معروف، محبوب و قابل دسترس (فیلتر نشده) باشد
- **باید** روی پورت ۴۴۳ (HTTPS) فعال باشد
- دامنه‌هایی که معمولاً خوب کار می‌کنند:
  - `www.speedtest.net`
  - `www.google.com`
  - `security.vercel.com`
  - `cdn.cloudflare.com`
  - `ajax.googleapis.com`

اگر یک `fake_sni` کار نمی‌کند، چند دامنه‌ی دیگر را امتحان کنید.

---

## حالت Gaming Mode

برای بازی‌های آنلاین و اپلیکیشن‌های حساس به تأخیر، `gaming_mode` را فعال کنید:

```json
{
  "listen": "0.0.0.0:40443",
  "connect": "YOUR_SERVER_IP:443",
  "fake_sni": "www.speedtest.net",
  "gaming_mode": true
}
```

**تفاوت:**
- `gaming_mode: false` → بافر ۲۵۶ کیلوبایت → throughput بالا، مناسب دانلود/استریم
- `gaming_mode: true` → بافر ۳۲ کیلوبایت → تأخیر کمتر، مناسب بازی/ویدیوکال

در هر دو حالت `TCP_NODELAY` فعال است تا از تأخیر الگوریتم Nagle جلوگیری شود.

---

## کنترل لاگ‌ها

| سطح | چه می‌بینید |
|-----|------------|
| `error` | فقط خطاهای مهلک |
| `warn` (پیش‌فرض) | خطاها + هشدارهای اتصال |
| `info` | اطلاعات راه‌اندازی + رویدادهای هر اتصال |
| `debug` | ردیابی دقیق هر پکت |

```bash
# پیش‌فرض (فقط warn و بالاتر)
sudo ./target/release/sni-spoof config.json

# با اطلاعات بیشتر
RUST_LOG=info sudo ./target/release/sni-spoof config.json

# برای دیباگ کامل
RUST_LOG=debug sudo ./target/release/sni-spoof config.json
```

اگر `debounce_logs: true` در کانفیگ باشد، پیام‌های تکراری حداکثر هر ۵ ثانیه یک‌بار نمایش داده می‌شوند — مناسب محیط production برای جلوگیری از flood شدن لاگ‌ها.

---

## اسکریپت‌های سریع راه‌اندازی

پروژه شامل اسکریپت‌های راه‌اندازی آماده برای تمام پلتفرم‌ها است:

### لینوکس / macOS

اسکریپت `run.sh` ساخت، دسترسی و راه‌اندازی را مدیریت می‌کند:

```bash
# استفاده از config.json پیش‌فرض
./run.sh

# استفاده از کانفیگ دلخواه
./run.sh /path/to/config.json

# اجرا با لاگ‌های info
RUST_LOG=info ./run.sh config.json
```

اسکریپت خودکار:
- باینری را بررسی می‌کند و در صورت نبود ساخت می‌کند (`cargo build --release`)
- دسترسی sudo را مدیریت می‌کند (در صورت نیاز درخواست می‌کند)
- سطح لاگ پیش‌فرض را `warn` تنظیم می‌کند

### ویندوز

اسکریپت `run.bat` ساخت، بررسی دسترسی و راه‌اندازی را مدیریت می‌کند:

```cmd
REM Command Prompt را راست‌کلیک و "Run as administrator" را انتخاب کنید، سپس:
run.bat

REM استفاده از کانفیگ دلخواه
run.bat C:\path\to\config.json

REM اجرا با لاگ‌های info
set RUST_LOG=info
run.bat config.json
```

اسکریپت خودکار:
- دسترسی Administrator را بررسی می‌کند
- باینری را بررسی می‌کند و در صورت نبود ساخت می‌کند (`cargo build --release`)
- سطح لاگ پیش‌فرض را `warn` تنظیم می‌کند
- pause نشان می‌دهد تا خروجی را ببینید
