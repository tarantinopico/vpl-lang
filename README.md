# VPL Systems - Optimized Native Compiler (v1.5.0)

VPL (Visual Programming Language) is a high-performance, statically compiled programming language designed for Linux (Solus) environments. It features a modern syntax, an extensive standard library, and a professional-grade compiler that produces tiny, standalone native binaries.

## 🚀 Key Features

- **Blazing Fast Native Code**: Compiles through Rust/LLVM with `opt-level=3` by default.
- **Smart Tree-Shaking (DCE)**: Only includes used runtime modules (Core, GUI, Network, etc.) in the final binary to keep file sizes minimal.
- **Dual Graphics Engine**:
  - **TUI**: Advanced terminal interfaces using ANSI escape sequences (windows, buttons, menus).
  - **GUI**: Modern system dialogs (calendars, sliders, lists, color pickers, notifications) via Zenity integration.
- **Industrial Compiler Logger**: Animated, threaded, and detailed build logs with microsecond-precision diagnostics.
- **Cross-Platform Readiness**: Native support for building Windows `.exe` files from Linux using the `-w` flag.
- **Rich Standard Library**: Built-in support for JSON, Filesystem, Advanced Networking (HTTP/TCP/IP), Logic/Bitwise operations, and specialized Math (GCD, LCM, Trig).
- **Color Tools**: Built-in functions for RGB/HEX manipulation and color blending.

## 🛠 Prerequisites

To use the VPL compiler, your system needs:

1. **Rust Toolchain**: `rustc` is required as the primary backend for LLVM linking.
   - *Solus Linux:* `sudo eopkg install rust`
2. **Zenity**: Required for rendering GUI dialogs.
   - *Solus Linux:* `sudo eopkg install zenity`
3. **Mingw-w64 (Optional)**: Required only if you intend to cross-compile for Windows.
   - *Solus Linux:* `sudo eopkg install mingw-w64`

## 📂 Project Structure

- `/vpl` - The main compiler binary.
- `/src` - Rust source code for the Lexer, Parser, and Compiler.
- `/examples` - A rich suite of demo scripts (.vpl).
- `lang.txt` - Complete language specification and function reference.

## 💻 Usage

### Interactive Selector (Build & Run)
Launch the TUI file explorer to browse, build, and run your scripts visually:
```bash
./vpl tui
```

### Manual Build
```bash
# Basic build (produces a native binary)
./vpl build examples/hello.vpl

# Build with custom output name
./vpl build examples/game.vpl -o my_game

# Cross-compile for Windows (.exe)
./vpl build examples/gui_demo.vpl -o windows_app -w
```

## 📝 Syntax at a Glance

```vpl
// Example: Modern VPL Script v1.5.0
func diagnose() {
    set ip = net_ip()
    set host = net_hostname()
    say "Host: " + host + " (IP: " + ip + ")"
}

set ping_time = net_ping_ms("8.8.8.8")
if logic_and(ping_time > 0, ping_time < 100) {
    gui_notify("Network is fast!", "VPL Status")
}

diagnose()
```

## ⚖️ License
VPL is an industrial-grade tool provided as-is for high-efficiency systems development.
