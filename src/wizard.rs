/// Interactive first-run wizard and named preset generator.
/// Invoked via `--wizard` or `--preset <name>` command-line flags.
use std::io::{self, BufRead, Write};

const HCAPTCHA_POOL: &[&str] = &[
    "hcaptcha.com",
    "newassets.hcaptcha.com",
    "js.hcaptcha.com",
    "imgs.hcaptcha.com",
    "api.hcaptcha.com",
    "analytics.hcaptcha.com",
];

const CLOUDFLARE_POOL: &[&str] = &[
    "cloudflare.com",
    "www.cloudflare.com",
    "cdn.cloudflare.com",
    "api.cloudflare.com",
];

fn prompt(msg: &str, default: &str) -> String {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    if default.is_empty() {
        write!(out, "{}: ", msg).unwrap();
    } else {
        write!(out, "{} [{}]: ", msg, default).unwrap();
    }
    out.flush().unwrap();

    let stdin = io::stdin();
    let line = stdin.lock().lines().next()
        .and_then(|l| l.ok())
        .unwrap_or_default();

    if line.trim().is_empty() {
        default.to_string()
    } else {
        line.trim().to_string()
    }
}

fn prompt_yn(msg: &str, default: bool) -> bool {
    let default_str = if default { "Y/n" } else { "y/N" };
    let answer = prompt(msg, default_str);
    match answer.to_lowercase().as_str() {
        "y" | "yes" => true,
        "n" | "no" => false,
        _ => default,
    }
}

fn write_config(path: &str, json: &str) -> Result<(), String> {
    std::fs::write(path, json).map_err(|e| format!("failed to write {}: {}", path, e))
}

/// Run the interactive wizard. Generates config.json in the current directory.
pub fn run_wizard() -> Result<(), String> {
    println!();
    println!("=== SNI Spoofing Setup Wizard ===");
    println!("This will create a config.json for you.");
    println!("Press Enter to accept defaults shown in [brackets].");
    println!();

    let connect = prompt("Upstream server IP:port to protect (e.g. 1.2.3.4:443)", "");
    if connect.is_empty() {
        return Err("upstream address is required".into());
    }
    // Basic validation
    if connect.parse::<std::net::SocketAddr>().is_err() {
        return Err(format!("'{}' is not a valid IP:port address", connect));
    }

    let listen = prompt("Local port to listen on", "40443");
    let listen_addr = format!("0.0.0.0:{}", listen.trim_start_matches("0.0.0.0:"));

    println!();
    println!("Choose a fake SNI pool (used to fool DPI inspection):");
    println!("  1. hCaptcha pool — 6 SNIs (recommended)");
    println!("  2. Cloudflare pool — 4 SNIs");
    println!("  3. Enter a custom SNI");
    let choice = prompt("Choice", "1");

    let sni_pool: Vec<String> = match choice.trim() {
        "2" => CLOUDFLARE_POOL.iter().map(|s| s.to_string()).collect(),
        "3" => {
            let custom = prompt("Enter your custom SNI (e.g. example.com)", "");
            if custom.is_empty() {
                return Err("SNI cannot be empty".into());
            }
            vec![custom]
        }
        _ => HCAPTCHA_POOL.iter().map(|s| s.to_string()).collect(),
    };

    println!();
    let gaming_mode = prompt_yn("Enable gaming mode (lower latency, less throughput)?", false);
    let dpi_evasion = prompt_yn("Enable advanced DPI evasion (fragmentation + padding)?", false);

    let advanced_section = if dpi_evasion {
        r#",
  "advanced": {
    "payload_padding": { "min_extra_bytes": 0, "max_extra_bytes": 128 },
    "fragmentation": { "enabled": true, "fragments": 2, "delay_ms": 1 }
  }"#
    } else {
        ""
    };

    let pool_json: Vec<String> = sni_pool.iter().map(|s| format!("\"{}\"", s)).collect();
    let pool_str = pool_json.join(", ");

    let json = format!(
        r#"{{
  "listeners": [
    {{
      "listen": "{}",
      "connect": "{}",
      "fake_sni_pool": [{}],
      "gaming_mode": {}
    }}
  ]{}
}}
"#,
        listen_addr, connect, pool_str, gaming_mode, advanced_section
    );

    let output_path = "config.json";
    write_config(output_path, &json)?;

    println!();
    println!("Config saved to {}:", output_path);
    println!("{}", json);
    println!("Run the program now (with sudo/admin) to start.");
    Ok(())
}

/// Apply a named preset, writing config.json to the current directory.
/// Presets: hcaptcha | cloudflare | stealth
pub fn apply_preset(name: &str) -> Result<(), String> {
    let (pool, advanced) = match name {
        "hcaptcha" => (HCAPTCHA_POOL, false),
        "cloudflare" => (CLOUDFLARE_POOL, false),
        "stealth" => (HCAPTCHA_POOL, true),
        _ => {
            return Err(format!(
                "unknown preset '{}'. Available presets: hcaptcha, cloudflare, stealth",
                name
            ));
        }
    };

    let pool_json: Vec<String> = pool.iter().map(|s| format!("\"{}\"", s)).collect();
    let pool_str = pool_json.join(", ");

    let advanced_section = if advanced {
        r#",
  "advanced": {
    "payload_padding": { "min_extra_bytes": 0, "max_extra_bytes": 128 },
    "fragmentation": { "enabled": true, "fragments": 2, "delay_ms": 1 }
  }"#
    } else {
        ""
    };

    // Presets require the user to fill in the upstream address themselves
    // (we don't know it at preset-generation time).
    let json = format!(
        r#"{{
  "listeners": [
    {{
      "listen": "0.0.0.0:40443",
      "connect": "UPSTREAM_IP:443",
      "fake_sni_pool": [{}]
    }}
  ]{}
}}
"#,
        pool_str, advanced_section
    );

    let output_path = "config.json";
    write_config(output_path, &json)?;

    println!("Preset '{}' written to {}.", name, output_path);
    println!("Edit the file and replace UPSTREAM_IP with your actual upstream server IP.");
    Ok(())
}
