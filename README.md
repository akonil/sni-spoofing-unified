# SNI Spoof — Unified Rust Proxy

> **English** | [فارسی](#فارسی)

---

A cross-platform SNI spoofing proxy written in Rust. Helps access content blocked by network filtering by spoofing the domain name (SNI) during connection setup. Works on **Linux**, **macOS**, and **Windows**.

### New to SNI Spoofing?

If you're unfamiliar with how SNI spoofing works, see [**SNI_SPOOFING_EXPLAINED_FA.md**](./SNI_SPOOFING_EXPLAINED_FA.md) for a beginner-friendly explanation in Persian (فارسی).

---

# PART 1: FOR REGULAR USERS

## Quick Start (30 seconds)

### Option A: Interactive Setup (Recommended)

```bash
sudo ./sni-spoof --wizard
```

Answer a few simple questions and the tool creates a config for you.

### Option B: Quick Preset

```bash
./sni-spoof --preset hcaptcha   # Recommended for most users
./sni-spoof --preset cloudflare # Alternative
./sni-spoof --preset stealth    # For stricter filters
```

---

## Get the Binary

Download the latest version for your system from [Releases](https://github.com/akonil/sni-spoofing-unified/releases):

**Linux:**
```bash
wget https://github.com/akonil/sni-spoofing-unified/releases/latest/download/sni-spoof-linux-x64.tar.gz
tar xzf sni-spoof-linux-x64.tar.gz
sudo ./sni-spoof-linux-x64 --wizard
```

**macOS (Intel or Apple Silicon):**
```bash
# Intel
curl -L -o sni-spoof-macos-x64.tar.gz https://github.com/akonil/sni-spoofing-unified/releases/latest/download/sni-spoof-macos-x64.tar.gz
tar xzf sni-spoof-macos-x64.tar.gz
sudo ./sni-spoof-macos-x64 --wizard

# Apple Silicon
curl -L -o sni-spoof-macos-arm64.tar.gz https://github.com/akonil/sni-spoofing-unified/releases/latest/download/sni-spoof-macos-arm64.tar.gz
tar xzf sni-spoof-macos-arm64.tar.gz
sudo ./sni-spoof-macos-arm64 --wizard
```

**Windows:** Download `sni-spoof-windows-x64.zip`, extract, then run `cmd` as Administrator and type:
```cmd
.\sni-spoof-windows-x64.exe --wizard
```

---

## Basic Configuration

If you prefer to edit the config file directly, create `config.json`:

```json
{
  "listeners": [
    {
      "listen": "0.0.0.0:40443",
      "connect": "YOUR_SERVER_IP:443",
      "fake_sni_pool": ["www.speedtest.net", "www.google.com"]
    }
  ]
}
```

Replace `YOUR_SERVER_IP` with your target server's IP address.

Then run:
```bash
sudo ./sni-spoof config.json
```

---

## Test It Works

1. Point your browser/app to `127.0.0.1:40443` (or whatever port you set in `listen`)
2. If you see connections in the logs, it's working!

---

## Need Help?

| Problem | Solution |
|---------|----------|
| "Permission denied" | Run with `sudo` (or use launcher script: `./run.sh`) |
| Can't connect | Check that `YOUR_SERVER_IP` is correct in config |
| "Config file not found" | Make sure `config.json` exists in the current directory |

---

---

# PART 2: FOR ADVANCED USERS

## How It Works

SNI (Server Name Indication) is sent unencrypted during the TLS handshake. DPI firewalls inspect this to block connections. This tool:

1. Intercepts the real TLS ClientHello from your client
2. Injects a **fake ClientHello** with a decoy SNI (e.g., `speedtest.net`) at a deliberately wrong TCP sequence number
3. DPI sees the decoy SNI and allows the connection
4. The real server ignores the fake packet (wrong sequence) and processes the real one
5. Data flows transparently between client and server

---

## Requirements

| Platform | Requirement |
|----------|-------------|
| Linux    | `sudo` or `CAP_NET_RAW` capability |
| macOS    | `sudo` (BPF device access) |
| Windows  | Run as **Administrator** (WinDivert) |

**Build tools:** [Rust](https://rustup.rs) ≥ 1.70 (optional, only if building from source)

---

## Build from Source

### Debug build (for development):
```bash
cargo build
```

### Release build (optimized, recommended):
```bash
cargo build --release
```

Binary location:
- Linux/macOS: `target/release/sni-spoof`
- Windows: `target\release\sni-spoof.exe`

---

## Advanced Configuration

Complete configuration schema:

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
      "connect": "172.67.139.236:443",
      "fake_sni_pool": ["security.vercel.com", "cdn.vercel.com"],
      "max_connections_per_sec": 0,
      "gaming_mode": false
    }
  ],
  "advanced": {
    "payload_padding": { "min_extra_bytes": 0, "max_extra_bytes": 128 },
    "fragmentation": { "enabled": false, "fragments": 2, "delay_ms": 1 }
  }
}
```

### Global Configuration Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `debounce_logs` | bool | `false` | Suppress repeated identical log messages within a 5-second window. Useful in production to avoid log flooding. Keep `false` during debugging. |
| `jitter.min_ms` | number | `1` | Minimum random delay (ms) before sending fake ClientHello. Defeats timing-based DPI analysis. |
| `jitter.max_ms` | number | `8` | Maximum random delay (ms). Set to `0` to disable jitter (not recommended). |
| `timeouts.handshake_timeout_ms` | number | `5000` | TCP handshake timeout (ms). Increase for slow servers, decrease for faster failure detection. |
| `timeouts.confirmation_timeout_ms` | number | `2000` | Time to wait (ms) for fake packet injection confirmation. Increase if sniffer is slow. |
| `listeners` | array | — | Array of listener definitions (see below). |

### Per-Listener Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `listen` | string | — | Local address:port to accept connections on. Use `0.0.0.0:PORT` for all interfaces. |
| `connect` | string | — | Upstream server IP:port (real destination). |
| `fake_sni` | string | — | (Legacy) Single decoy SNI. Deprecated; use `fake_sni_pool` instead. |
| `fake_sni_pool` | array | `[]` | Array of decoy hostnames. One chosen randomly per connection. Falls back to `fake_sni` if empty. Examples: `speedtest.net`, `www.google.com`, `security.vercel.com`. |
| `max_connections_per_sec` | number | `0` | Rate limit (0 = unlimited). Prevents connection floods. |
| `gaming_mode` | bool | `false` | `true`: 32 KB buffers (low latency). `false`: 256 KB buffers (high throughput). |

### Advanced DPI Evasion Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `advanced.payload_padding.min_extra_bytes` | number | `0` | Minimum random bytes appended to fake ClientHello. |
| `advanced.payload_padding.max_extra_bytes` | number | `0` | Maximum random bytes appended. `0` = disabled. Varies packet size to defeat fingerprinting. |
| `advanced.fragmentation.enabled` | bool | `false` | Split fake ClientHello into N TCP segments. Confuses DPI reassembly logic. |
| `advanced.fragmentation.fragments` | number | `2` | Number of segments (2 or 3). More fragments = harder to reassemble. |
| `advanced.fragmentation.delay_ms` | number | `1` | Millisecond delay between segments. Small delay (1-5 ms) helps defeat timeout-based reassembly. |

---

## Advanced DPI Evasion Techniques

### 1. SNI Pool Rotation

Instead of using a single fake SNI, rotate through multiple domains per connection:

```json
"fake_sni_pool": [
  "hcaptcha.com",
  "newassets.hcaptcha.com",
  "js.hcaptcha.com",
  "api.hcaptcha.com"
]
```

Each connection randomly picks one SNI from the pool, making traffic less predictable to DPI fingerprinting.

### 2. Payload Padding

Add random extra bytes to the fake ClientHello to vary packet size:

```json
"advanced": {
  "payload_padding": {
    "min_extra_bytes": 0,
    "max_extra_bytes": 128
  }
}
```

Prevents DPI from fingerprinting based on exact packet size (e.g., "all blocks are exactly 517 bytes").

### 3. Fragmentation

Split the fake ClientHello into multiple TCP segments:

```json
"advanced": {
  "fragmentation": {
    "enabled": true,
    "fragments": 2,
    "delay_ms": 1
  }
}
```

Makes reassembly harder for DPI engines. Each fragment arrives as a separate TCP packet with a small delay between them.

---

---

## Multiple Listeners

Define multiple listeners to proxy different ports in one config:

```json
{
  "listeners": [
    {
      "listen": "0.0.0.0:40443",
      "connect": "172.67.139.236:443",
      "fake_sni_pool": ["security.vercel.com", "cdn.vercel.com"]
    },
    {
      "listen": "0.0.0.0:40080",
      "connect": "172.67.139.236:80",
      "fake_sni_pool": ["speedtest.net", "www.google.com"]
    }
  ]
}
```

Each listener operates independently with its own SNI pool and rate limiting.

---

## Running the Proxy

```bash
# Default (read config.json in current directory)
sudo ./sni-spoof

# Custom config file
sudo ./sni-spoof /path/to/config.json

# With verbose logging
RUST_LOG=info sudo ./sni-spoof config.json
```

**Windows (run Command Prompt as Administrator first):**
```cmd
.\sni-spoof.exe config.json
```

---

## Monitoring and Statistics

### Connection Statistics

Enable info logging to see per-connection metrics:

```bash
RUST_LOG=info sudo ./sni-spoof config.json
```

Output:
```
INFO connection opened  upstream=172.67.139.236:443 active=3 total=47
INFO connection closed  upstream=172.67.139.236:443 active=2 total=47
```

- **active** — concurrent connections
- **total** — cumulative since startup

### SNI Success/Failure Stats

When using `fake_sni_pool` with multiple SNIs, the proxy logs success/failure counts every 60 seconds:

```
INFO SNI stats (top 10 by success):
INFO   security.vercel.com → ok=142 fail=3
INFO   cdn.vercel.com → ok=98 fail=1
```

Use this to identify which SNI domains work best in your region.

---

## Logging Levels

Control verbosity with `RUST_LOG`:

| Level | Content |
|-------|---------|
| `error` | Fatal errors only |
| `warn` | Errors + connection warnings (default) |
| `info` | Startup, per-connection events, SNI stats |
| `debug` | Detailed per-packet tracing |
| `trace` | Very verbose internal state |

Example:
```bash
RUST_LOG=debug sudo ./sni-spoof config.json
```

When `debounce_logs: true`, repeated warnings are suppressed and printed once every 5 seconds with a count of skipped messages.

---

## Choosing SNI Domains

When selecting `fake_sni_pool` domains:

- **Popular domains** work best: `www.speedtest.net`, `www.google.com`, `security.vercel.com`
- **Must support HTTPS** (port 443)
- **Must not be blocked** in your region
- **Rotate multiple domains** for better DPI evasion

If one SNI stops working, try others. The SNI stats output helps identify what works.

---

## Project Structure

```
sni-spoofing-unified/
├── src/
│   ├── main.rs          # Entry point: loads config, starts sniffer + listeners
│   ├── config.rs        # Config deserialization & validation (SNI pool, advanced DPI options)
│   ├── debounce.rs      # Rate-limited logging module
│   ├── handler.rs       # Per-connection logic: SNI rotation, padding, fragmentation, tracking
│   ├── listener.rs      # TCP accept loop with rate limiting
│   ├── relay.rs         # Bidirectional data relay
│   ├── shutdown.rs      # Graceful signal handling (Ctrl+C / SIGTERM)
│   ├── error.rs         # Typed error definitions
│   ├── proto.rs         # Internal channel message types (Registration, SnifferResult)
│   ├── stats.rs         # Connection stats and SniTracker (SNI success/failure counts)
│   ├── wizard.rs        # Interactive config wizard and named presets (--wizard, --preset)
│   ├── packet/          # Raw packet parsing (Ethernet, IP, TCP, TLS)
│   └── sniffer/         # Platform-specific packet capture backends
│       ├── linux.rs     # AF_PACKET raw socket
│       ├── macos.rs     # BPF device
│       ├── windows.rs   # WinDivert
│       └── mod.rs       # Shared sniffer state machine (fragment injection)
├── config.json          # Example configuration
├── Cargo.toml
└── .gitignore
```

---

## Acknowledgments

This project is based on the excellent work of the following contributors:

- **[@therealaleph/sni-spoofing-rust](https://github.com/therealaleph/sni-spoofing-rust)** — The immediate base of this fork. A clean Rust implementation of the SNI spoofing technique with cross-platform support.

- **[@patterniha/SNI-Spoofing](https://github.com/patterniha/SNI-Spoofing)** — The original creator of this DPI bypass technique using IP/TCP header manipulation. All credit for the core idea and method goes to patterniha.

- **[@atarevals/SNI-Spoofing](https://github.com/atarevals/SNI-Spoofing)** — Alternative implementation that provided additional ideas and insights.

This unified rewrite adds configurable timeouts, randomized jitter, connection statistics, gaming mode, and debounced logging while maintaining compatibility with the original technique.

## License

MIT License — See [LICENSE](LICENSE) file for details.

---

---

# فارسی

> [English](#sni-spoof--unified-rust-proxy) | **فارسی**

---

یک پروکسی جعل SNI با کارایی بالا و چندسکویی، نوشته‌شده به Rust. با تزریق یک TLS ClientHello جعلی با SNI فریب‌کارانه در طول مذاکره TCP، فیلترینگ مبتنی بر DPI را دور می‌زند و سپس ترافیک را به صورت شفاف ریلی می‌کند.

**سیستم‌عامل‌های پشتیبانی‌شده:** Linux · macOS · Windows

### تازه‌وارد به SNI Spoofing؟

اگر با نحوه‌ی کارکرد SNI spoofing آشنا نیستید، به [**SNI_SPOOFING_EXPLAINED_FA.md**](./SNI_SPOOFING_EXPLAINED_FA.md) مراجعه کنید برای یک توضیح ساده برای تازه‌واردها.

---

## نحوه کارکرد

۱. کلاینت به آدرس محلی پروکسی وصل می‌شود.  
۲. پروکسی یک اتصال TCP به سرور مقصد باز می‌کند.  
۳. در طول مذاکره، یک پکت ClientHello جعلی با SNI فریب‌کار (مثلاً `speedtest.net`) با شماره‌ی ترتیب (sequence number) اشتباه تزریق می‌شود.  
۴. فایروال DPI، SNI فریب‌کار را می‌بیند و اجازه عبور می‌دهد.  
۵. سرور واقعی پکت جعلی را نادیده می‌گیرد (چون seq number اشتباه است) و به پکت اصلی پاسخ می‌دهد.  
۶. پروکسی داده را بین کلاینت و سرور دوطرفه ریلی می‌کند.

---

## پیش‌نیازها

| سیستم‌عامل | نیاز |
|------------|------|
| Linux      | `sudo` یا قابلیت `CAP_NET_RAW` |
| macOS      | `sudo` (دسترسی به BPF device) |
| Windows    | اجرا به عنوان **Administrator** (WinDivert) |

**ابزار ساخت:** [Rust](https://rustup.rs) نسخه ۱.۷۰ یا بالاتر

---

## ساخت (Build)

```bash
# ساخت debug (برای توسعه)
cargo build

# ساخت release (بهینه‌شده، توصیه‌شده برای استفاده واقعی)
cargo build --release
```

مسیر فایل اجرایی پس از release build:
- Linux/macOS: `target/release/sni-spoof`
- Windows: `target\release\sni-spoof.exe`

---

## پیکربندی (Configuration)

فایل `config.json` را در پوشه‌ی پروژه ایجاد یا ویرایش کنید:

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
      "connect": "172.67.139.236:443",
      "fake_sni": "security.vercel.com",
      "gaming_mode": false
    }
  ]
}
```

### توضیح فیلدهای تنظیمات

#### سطح بالا

| فیلد | نوع | پیش‌فرض | توضیح |
|------|-----|---------|-------|
| `debounce_logs` | bool | `false` | پیام‌های تکراری را در بازه ۵ ثانیه‌ای حذف می‌کند. برای محیط تولید مفید است تا لاگ‌ها flood نشوند. در حین دیباگ روی `false` نگه دارید. |
| `jitter.min_ms` | عدد | `1` | حداقل تأخیر تصادفی (میلی‌ثانیه) قبل از ارسال ClientHello جعلی. |
| `jitter.max_ms` | عدد | `8` | حداکثر تأخیر تصادفی (میلی‌ثانیه). برای غیرفعال کردن جیتر، مقدار `0` بگذارید. جیتر به‌صورت پیش‌فرض فعال است چون تحلیل زمانی DPI را ناکام می‌کند. |
| `timeouts.handshake_timeout_ms` | عدد | `5000` | مهلت TCP handshake بر حسب میلی‌ثانیه. اگر سرور دور یا کند است، مقدار را افزایش دهید؛ برای تشخیص سریع شکست، کاهش دهید. |
| `timeouts.confirmation_timeout_ms` | عدد | `2000` | مهلت انتظار برای تأیید تزریق پکت جعلی توسط sniffer (میلی‌ثانیه). |
| `listeners` | آرایه | — | یک یا چند تعریف listener (ببینید پایین). |

#### هر Listener

| فیلد | نوع | پیش‌فرض | توضیح |
|------|-----|---------|-------|
| `listen` | string | — | آدرس و پورت محلی که پروکسی روی آن گوش می‌دهد. از `0.0.0.0` برای پذیرش از همه‌ی اینترفیس‌ها استفاده کنید. |
| `connect` | string | — | آی‌پی و پورت سرور مقصد (سرور واقعی که می‌خواهید به آن وصل شوید). |
| `fake_sni` | string | — | نام دامنه‌ی جعلی که در ClientHello تزریق می‌شود. باید یک دامنه‌ی معروف و مجاز باشد (مثل `speedtest.net`، `www.google.com`). |
| `gaming_mode` | bool | `false` | وقتی `true` است: بافرهای کوچک (۳۲ کیلوبایت) برای تأخیر کمتر استفاده می‌شود. وقتی `false` است: بافرهای بزرگ (۲۵۶ کیلوبایت) برای throughput بالاتر. برای بازی آنلاین و اپلیکیشن‌های real-time فعال کنید؛ برای دانلود و استریم خاموش بگذارید. |

### تأخیر تصادفی (Inject Jitter)

جیتر یک تأخیر تصادفی کوچک (پیش‌فرض ۱ تا ۸ میلی‌ثانیه) بین تشخیص پایان handshake TCP و ارسال ClientHello جعلی اضافه می‌کند. این تصادفی‌بودن باعث می‌شود سیستم‌های DPI نتوانند از روی زمان‌بندی، روش جعل را شناسایی کنند.

برای غیرفعال کردن جیتر (توصیه نمی‌شود):
```json
"jitter": { "min_ms": 0, "max_ms": 0 }
```

---

## آمار اتصالات

وقتی با `RUST_LOG=info` اجرا می‌کنید، هر اتصال هنگام باز و بسته شدن آمار خود را لاگ می‌کند:

```
INFO connection opened  upstream=172.67.139.236:443 active=3 total=47
INFO connection closed  upstream=172.67.139.236:443 active=2 total=47
```

- **active** — اتصالات در حال ریلی داده
- **total** — تعداد کل اتصالات از زمان راه‌اندازی

---

### چند Listener همزمان

می‌توانید چند listener تعریف کنید تا پورت‌های مختلف را فوروارد کنید:

```json
{
  "debounce_logs": false,
  "jitter": { "min_ms": 1, "max_ms": 8 },
  "listeners": [
    {
      "listen": "0.0.0.0:40443",
      "connect": "172.67.139.236:443",
      "fake_sni": "security.vercel.com",
      "gaming_mode": false
    },
    {
      "listen": "0.0.0.0:40080",
      "connect": "172.67.139.236:80",
      "fake_sni": "speedtest.net",
      "gaming_mode": false
    }
  ]
}
```

---

## اجرا

```bash
# اجرا با config.json پیش‌فرض در پوشه‌ی جاری
sudo ./target/release/sni-spoof

# اجرا با فایل کانفیگ مشخص
sudo ./target/release/sni-spoof /path/to/config.json

# کنترل سطح لاگ (error / warn / info / debug / trace)
RUST_LOG=info sudo ./target/release/sni-spoof config.json
```

**ویندوز:** Command Prompt را به عنوان Administrator باز کنید، سپس:
```cmd
.\target\release\sni-spoof.exe config.json
```

### انتخاب fake_sni مناسب

دامنه‌ی جعلی باید:
- معروف و قابل دسترس (فیلتر نشده) باشد
- در عمل از HTTPS (پورت ۴۴۳) استفاده کند
- مثال‌ها: `www.speedtest.net`، `www.google.com`، `security.vercel.com`، `cdn.cloudflare.com`

---

## لاگ‌ها

سطح لاگ‌ها با متغیر محیطی `RUST_LOG` کنترل می‌شود:

| سطح | چه می‌بینید |
|-----|------------|
| `error` | فقط خطاهای مهلک |
| `warn` (پیش‌فرض) | خطاها + هشدارهای اتصال |
| `info` | اطلاعات راه‌اندازی + رویدادهای هر اتصال |
| `debug` | ردیابی دقیق هر پکت |

```bash
RUST_LOG=info sudo ./target/release/sni-spoof config.json
```

وقتی `debounce_logs: true` باشد، پیام‌های تکراری `warn`/`error` برای یک نوع رویداد، حداکثر هر ۵ ثانیه یک‌بار چاپ می‌شوند و تعداد پیام‌های حذف‌شده هم نشان داده می‌شود.

---

## ساختار پروژه

```
sni-spoofing-unified/
├── src/
│   ├── main.rs          # نقطه ورود: بارگذاری کانفیگ، راه‌اندازی sniffer و listener ها
│   ├── config.rs        # خواندن و اعتبارسنجی کانفیگ
│   ├── debounce.rs      # ماژول لاگ با محدودیت نرخ
│   ├── handler.rs       # منطق هر اتصال: تزریق جعلی + ریلی
│   ├── listener.rs      # حلقه accept اتصال TCP
│   ├── relay.rs         # ریلی دوطرفه‌ی داده
│   ├── shutdown.rs      # مدیریت سیگنال‌ها (Ctrl+C / SIGTERM)
│   ├── error.rs         # تعریف خطاهای typed
│   ├── proto.rs         # انواع پیام کانال داخلی
│   ├── packet/          # پارس پکت خام (Ethernet, IP, TCP, TLS)
│   └── sniffer/         # پشتیبان‌های capture پکت مخصوص هر پلتفرم
│       ├── linux.rs     # raw socket AF_PACKET
│       ├── macos.rs     # BPF device
│       ├── windows.rs   # WinDivert
│       └── mod.rs       # state machine مشترک sniffer
├── config.json          # تنظیمات نمونه
├── Cargo.toml
└── .gitignore
```
