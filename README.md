# VPL Systems - Optimized Native Compiler (v1.9.0)

VPL (Visual Programming Language) is a high-performance, statically compiled programming language designed for Linux environments. It features a modern syntax, an extensive standard library, and a professional-grade compiler that produces tiny, standalone native binaries.

## 🚀 Key Features

- **Industrial Build Pipeline:** Animated progress logs with microsecond precision.
- **Intelligent Library System:** Automatic dependency resolution for `.vplib` files.
- **Advanced TUI Engine:** Create professional terminal interfaces with shadows, windows, and custom boxes.
- **Modern GUI Library:** Native desktop dialogs via built-in Zenity wrappers.
- **Tree-Shaking Optimizer:** Final binaries only include the modules you actually use.
- **Cross-Platform Building:** Easily compile Linux binaries or Windows `.exe` files.
- **Scientific Math & Logic:** Full suite of trigonometric, scientific, and bitwise functions.

## 📚 Intelligent Library System

VPL automatically manages your dependencies. You don't need `import` or `include` statements.
- Save your reusable code in `.vplib` files.
- VPL searches in the **current directory**, a local **`.vpl/`** or **`.vplib/`** folder, and the global **`~/.vpl/lib/`** path.
- If you call a function that is defined in any of these locations, the compiler will automatically link the necessary library.
- **Zero Configuration:** Just write code and call functions; the compiler handles the rest.

## 📥 Installation

VPL requires the **Rust compiler** and **Curl** to be installed on your system.

### 1. Install Dependencies

Depending on your Linux distribution, run:

**Debian / Ubuntu / Mint / Pop!_OS:**
```bash
sudo apt update && sudo apt install rustc curl zenity
```

**Fedora / Red Hat / CentOS:**
```bash
sudo dnf install rust curl zenity
```

**Arch Linux / Manjaro:**
```bash
sudo pacman -S rust curl zenity
```

**Solus Linux:**
```bash
sudo eopkg install rust curl zenity
```

### 2. Get VPL
Clone this repository or download the binary, then move it to your path:
```bash
chmod +x vpl
sudo mv vpl /usr/local/bin/
```

## 🛠️ Usage

### Quick Run (JIT-like)
Execute any `.vpl` file immediately without keeping the binary:
```bash
vpl run examples/hello.vpl
```

### Build Native Binary
```bash
vpl build program.vpl -o my_app
```

### Build for Windows
```bash
vpl build program.vpl -w
```

### Interactive Selector (TUI)
Launch the professional file selector and builder:
```bash
vpl tui
```
*Inside TUI, use **W** to toggle Windows mode, **R** for Release, **S** for Strip, and **V** for Verbose logging.*

## 📝 Syntax at a Glance

```vpl
// Example: Modern VPL Script v1.8.0
func diagnose() {
    set ip = net_ip()
    set host = net_hostname()
    say "Host: " + host + " (IP: " + ip + ")"
    
    set cores = sys_cores()
    say "Processing on " + cores + " CPU cores."
}

diagnose()

set items = ["Apple", "Orange", "Banana"]
for fruit in items {
    say "I like " + fruit
}
```

## 📜 Full Documentation
See [lang.txt](lang.txt) for a complete list of all built-in functions and their usage.
