# 📘 VPL Language Specification (v2.1.0)

VPL (Visual Programming Language) is a high-performance, statically compiled language for Linux, compiling directly to native machine code via **LLVM**.

## 🏗️ 1. Basic Syntax
| Feature | Syntax | Example |
| :--- | :--- | :--- |
| **Variables** | `set name = value` | `set x = 10` |
| **Output** | `say expr` | `say "Hello " + x` |
| **Functions** | `func name(a, b) { ... }` | `func add(x, y) { return x + y }` |
| **Conditionals** | `if cond { ... } else { ... }` | `if x > 5 { say "Big" }` |
| **Loops** | `while cond { ... }` | `while i < 10 { set i = i + 1 }` |
| **For Loop** | `for item in array { ... }` | `for x in [1,2] { say x }` |

## 📦 2. Library System
VPL includes an **Intelligent Auto-Include** system.
- Store `.vplib` files in `./.vplib/` or `~/.vpl/lib/`.
- Calling a function like `gui_window()` automatically links the providing library.

## 🛠️ 3. Core Functions
### [TUI Engine]
- `tui_init()`, `tui_clear()`, `tui_print(x, y, text)`
- `tui_draw_box(x, y, w, h)`, `tui_read_key()`
- `tui_progress_bar(x, y, w, %, label)` *(via tui_widgets.vplib)*

### [Native GUI]
- `gfx_open(w, h, title)`, `gfx_close()`, `gfx_clear(color)`
- `gfx_rect(x, y, w, h, color)`, `gfx_text(x, y, text, color)`
- `gfx_poll()`: Returns a Map `{ "type": "click/key", "x": n, "y": n, "key": "s" }`

### [Security & Crypto]
- `crypto_xor_encrypt(data, key)`: Symmetric encryption.
- `crypto_simple_hash(data)`: Fast string hashing.

### [File System]
- `fs_read(path)`, `fs_write(path, content)`, `fs_list(dir)`
- `fs_exists(path)`, `fs_is_dir(path)`, `fs_delete(path)`

## 🚀 4. Compiler Commands
```bash
./vpl run app.vpl          # Quick execution
./vpl build app.vpl -o my  # Build standalone binary
./vpl build app.vpl -silent # Production silent build
```
