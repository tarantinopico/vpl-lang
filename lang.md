# 📘 VPL Language Specification (v2.2.0)

VPL (Visual Programming Language) is a high-performance language with a rich **Standard Library**.

## 🏗️ 1. Basic Syntax
| Feature | Example |
| :--- | :--- |
| **Variables** | `set x = 10` |
| **Dictionaries** | `set m = map_new()` |
| **Functions** | `func name(a) { ... }` |

## 📦 2. Standard Library Highlights
### [Data & Storage]
- `db_load(file)`, `db_save(file, data)`: Simple JSON DB.
- `map_copy(m)`, `arr_copy(a)`: Memory management.

### [Graphics & UI]
- `gui_window()`, `gui_button()`, `gui_input_draw()`
- `gui_sprite_draw(x, y, scale, data)`: Pixel art engine.
- `gui_grid_x()`, `gui_grid_y()`: Layout managers.

### [System & Tasks]
- `task_new_timer(ms)`: Non-blocking timers.
- `sys_clipboard_set(text)`: OS Clipboard access.

### [Testing]
- `test_assert(name, condition)`: Built-in testing.

## 🚀 3. Compiler usage
`./vpl build app.vpl`
