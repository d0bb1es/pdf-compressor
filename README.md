# PDF Compressor 👾

A tiny, local-first PDF compressor with a simple GUI. Drop a PDF, pick a quality, click Compress.
No uploads, no accounts, no tracking. Your files never leave your machine.

Built in Rust with [egui](https://github.com/emilk/egui), powered by [Ghostscript](https://ghostscript.com/).
Part of **H11_factory**: small tools that run locally.

<!-- Add a screenshot: put it at assets/screenshot.png and uncomment the next line -->
<!-- ![screenshot](assets/screenshot.png) -->

## Features

- Drag and drop or file picker
- Five quality presets: `screen`, `ebook`, `printer`, `prepress`, `default`
- Shows original size, compressed size and percent saved
- Never overwrites your original file
- Runs on low-end hardware (OpenGL backend, no GPU needed)

## Install

Requires Ghostscript:

```bash
sudo apt install ghostscript
```

**Download a binary:** grab the latest from [Releases](https://github.com/d0bb1es/pdf-compressor/releases), then:

```bash
chmod +x pdf-compressor-gui
./pdf-compressor-gui
```

**Or build from source:**

```bash
sudo apt install build-essential pkg-config libgtk-3-dev
git clone https://github.com/d0bb1es/pdf-compressor.git
cd pdf-compressor
cargo build --release
./target/release/pdf-compressor-gui
```

## Presets

| Preset     | Result                              |
|------------|-------------------------------------|
| `screen`   | Smallest file, low quality (72 dpi) |
| `ebook`    | Good balance (150 dpi), default     |
| `printer`  | High quality (300 dpi)              |
| `prepress` | Highest quality, color preserving   |
| `default`  | Ghostscript default                 |

## Support

If this saved you time, you can tip in crypto. Always double-check the network before sending.

| Asset | Network | Address |
|-------|---------|---------|
| BTC | Bitcoin | `YOUR_BTC_ADDRESS` |
| BTC | Lightning | `YOUR_LIGHTNING_ADDRESS` |
| ETH | Ethereum / Base / Arbitrum | `YOUR_ETH_ADDRESS` |
| USDC / USDT | `NETWORK` | `YOUR_STABLECOIN_ADDRESS` |

## License

MIT. Ghostscript is a separate program under AGPL and is not bundled with this tool.
