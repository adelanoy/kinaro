# Kinaro

A fast, native desktop application for testing and exploring REST APIs — written in Rust and built with [GPUI](https://github.com/zed-industries/zed), the GPU-accelerated UI framework from the creators of Zed.

![License](https://img.shields.io/badge/license-MIT-blue)
![Rust](https://img.shields.io/badge/rust-stable-orange?logo=rust)
![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20Windows-lightgrey)

---

## 📋 Features (WIP)

- **HTTP method support** — GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS
- **Request builder** — Rest steps: headers, query params, cookies, and multi-type bodies (JSON, form-data, x-www-form-urlencoded, raw, binary)
- **Database access** — DB steps: Write fixture data, or assert directly from databases 
- **Profile system** — variable interpolation with per-profile overrides (such as dev / staging / prod...) and secret masking
- **Test organization** — organize tests into Test Suites, composed of Test Cases, composed of Test Steps, persisted locally in an open, diff-friendly format
- **Response viewer** — formatted JSON/XML/HTML highlighting, response headers, timing breakdown
- **Scripted assertions** — lightweight test scripts to validate status codes, headers, and body contents
- **Themes** — GPUI-native theming, light and dark out of the box

## 🚀 Getting Started

### Prerequisites

- Rust stable (1.90+) — install via [rustup](https://rustup.rs)
- Platform dependencies for GPUI:
    - **Linux**: `libxkbcommon-dev`, `libwayland-dev`, `libfontconfig1-dev`, and a Vulkan-capable driver
    - **Windows**: MSVC Build Tools

### Build from source

```bash
git clone https://codeberg.org/fenhryl/kinaro.git
cd kinaro
cargo build --release