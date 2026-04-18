# SNI Spoof — Unified Rust Proxy

> **English** | [فارسی](#فارسی)

---

A cross-platform, high-performance SNI spoofing proxy written in Rust. Bypasses DPI-based censorship by injecting a fake TLS ClientHello with a decoy SNI during the TCP handshake, then relaying traffic transparently.

**Supported platforms:** Linux · macOS · Windows

---

## How It Works

1. A client connects to the proxy's local listen address.
2. The proxy opens a TCP connection to the upstream server.
3. During the handshake, a forged ClientHello packet with a fake SNI (e.g. `speedtest.net`) is injected at a deliberately wrong sequence number.
4. The DPI firewall sees the decoy SNI and allows the connection through.
5. The server discards the forged packet (wrong seq) and responds to the real one.
6. The proxy relays data bidirectionally between the client and the server.

---

## Requirements

| Platform | Requirement |
|----------|-------------|
| Linux    | `sudo` or `CAP_NET_RAW` capability |
| macOS    | `sudo` (BPF device access) |
| Windows  | Run as **Administrator** (WinDivert) |

**Build tools:** [Rust](https://rustup.rs) ≥ 1.70

---

## Build

```bash
# Debug build (for development)
cargo build

# Release build (optimized, recommended for production)
cargo build --release
```

Binary location after release build:
- Linux/macOS: `target/release/sni-spoof`
- Windows: `target\release\sni-spoof.exe`

---

## Configuration

Create or edit `config.json` in the project directory:

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

### Config Fields

#### Top-level

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `debounce_logs` | bool | `false` | Suppress repeated identical log messages within a 5-second window. Useful in production to avoid log flooding when many connections fail. Keep `false` during debugging. |
| `jitter.min_ms` | number | `1` | Minimum random delay (ms) injected before sending the fake ClientHello. |
| `jitter.max_ms` | number | `8` | Maximum random delay (ms). Set to `0` to disable jitter entirely. Jitter is enabled by default because it defeats timing-based DPI fingerprinting. |
| `timeouts.handshake_timeout_ms` | number | `5000` | TCP handshake timeout in milliseconds. Increase if connecting to slow/distant servers; decrease for faster failure detection. |
| `timeouts.confirmation_timeout_ms` | number | `2000` | Time to wait (ms) for the fake packet injection to be confirmed by the sniffer. |
| `listeners` | array | — | One or more listener definitions (see below). |

#### Per-listener

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `listen` | string | — | Local address and port the proxy listens on. Use `0.0.0.0` to accept from all interfaces. |
| `connect` | string | — | Upstream server IP and port to connect to (the real destination). |
| `fake_sni` | string | — | The decoy hostname injected in the forged ClientHello. Should be a popular, allowed domain (e.g. `speedtest.net`, `www.google.com`). |
| `gaming_mode` | bool | `false` | When `true`: uses small socket buffers (32 KB) for lower latency. When `false`: uses large buffers (256 KB) for higher throughput. Enable for gaming or real-time apps; leave off for downloads or streaming. |

### Inject Jitter

Jitter adds a small random delay (default 1–8 ms) between detecting the TCP handshake completion and sending the fake ClientHello. This randomness makes it much harder for DPI systems to identify the spoofing technique via timing analysis.

To disable jitter (not recommended):
```json
"jitter": { "min_ms": 0, "max_ms": 0 }
```

---

## Connection Stats

When running at `RUST_LOG=info`, each connection logs its stats on open and close:

```
INFO connection opened  upstream=172.67.139.236:443 active=3 total=47
INFO connection closed  upstream=172.67.139.236:443 active=2 total=47
```

- **active** — connections currently relaying data
- **total** — cumulative connections since startup

---

### Multiple Listeners

You can define multiple listeners in a single config to forward different ports:

```json
{
  "debounce_logs": false,
  "jitter": { "min_ms": 1, "max_ms": 8 },
  "timeouts": { "handshake_timeout_ms": 5000, "confirmation_timeout_ms": 2000 },
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

## Running

```bash
# Run with default config.json in current directory
sudo ./target/release/sni-spoof

# Run with a specific config file
sudo ./target/release/sni-spoof /path/to/config.json

# Control log verbosity (error / warn / info / debug / trace)
RUST_LOG=info sudo ./target/release/sni-spoof config.json
```

**Windows:** Open Command Prompt as Administrator, then:
```cmd
.\target\release\sni-spoof.exe config.json
```

### Choosing a fake_sni

The fake SNI should be a domain that:
- Is popular and known to be accessible (not blocked)
- Uses HTTPS (port 443) in practice
- Examples: `www.speedtest.net`, `www.google.com`, `security.vercel.com`, `cdn.cloudflare.com`

---

## Logging

Logging is controlled by the `RUST_LOG` environment variable:

| Level | What you see |
|-------|-------------|
| `error` | Only fatal errors |
| `warn` (default) | Errors + connection warnings |
| `info` | Startup info + per-connection events |
| `debug` | Detailed per-packet tracing |

```bash
RUST_LOG=info sudo ./target/release/sni-spoof config.json
```

When `debounce_logs: true`, repeated `warn`/`error` messages for the same event type are suppressed and printed at most once every 5 seconds, with a count of how many were skipped.

---

## Project Structure

```
sni-spoofing-unified/
├── src/
│   ├── main.rs          # Entry point: loads config, starts sniffer + listeners
│   ├── config.rs        # Config deserialization and validation
│   ├── debounce.rs      # Rate-limited logging module
│   ├── handler.rs       # Per-connection logic: fake inject + relay
│   ├── listener.rs      # TCP accept loop
│   ├── relay.rs         # Bidirectional data relay
│   ├── shutdown.rs      # Graceful signal handling (Ctrl+C / SIGTERM)
│   ├── error.rs         # Typed error definitions
│   ├── proto.rs         # Internal channel message types
│   ├── packet/          # Raw packet parsing (Ethernet, IP, TCP, TLS)
│   └── sniffer/         # Platform-specific packet capture backends
│       ├── linux.rs     # AF_PACKET raw socket
│       ├── macos.rs     # BPF device
│       ├── windows.rs   # WinDivert
│       └── mod.rs       # Shared sniffer state machine
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
