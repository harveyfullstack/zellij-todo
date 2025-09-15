# zellij-todo

> **⚠️ Development Status**: This plugin is currently under active development and may not be fully functional yet.

A dead-simple [Zellij](https://zellij.dev) plugin for managing a persistent floating todo list with intuitive flow.

## Features

- **Smart Ordering**: Items remember their positions when toggled between todo/done
- **Visual Separation**: Todo items stay at top, completed items at bottom
- **Grab & Reorder**: Move items with `g` + arrow keys, auto-skips completed items
- **Quick Editing**: Add, edit, and delete with simple keyboard shortcuts
- **Floating Design**: Works as an overlay without disrupting your workflow
- **Auto-persistence**: Saves automatically to filesystem

## Requirements

Tested with Zellij `v0.43.x`

### Zellij Plugin Permissions

| Permission               | Why                                    |
| ------------------------ | -------------------------------------- |
| `ReadApplicationState`   | Subscribe to key events                |
| `ChangeApplicationState` | Hide/show plugin and set pane name    |

### Host Filesystem Access

The plugin saves todos to a configurable location for persistence across sessions. By default, it saves to `.zellij_todos.json` in the directory where Zellij was launched. For global todos across all sessions, configure the plugin with a specific directory path.

## Install

### Download Binary

Download `zellij-todo.wasm` from the [releases page](https://github.com/your-repo/zellij-todo/releases) and copy to your Zellij plugins directory:

```bash
mv zellij-todo.wasm ~/.config/zellij/plugins/
```

### Build from Source

> Requires Rust with `wasm32-wasip1` target: `rustup target add wasm32-wasip1`

```bash
git clone https://github.com/your-repo/zellij-todo.git
cd zellij-todo
cargo build --release
mv target/wasm32-wasip1/release/zellij-todo.wasm ~/.config/zellij/plugins/
```

## Usage

### Launch Plugin

```bash
# As floating pane (recommended)
zellij plugin --floating -- "file:$HOME/.config/zellij/plugins/zellij-todo.wasm"

# In current pane
zellij plugin -- "file:$HOME/.config/zellij/plugins/zellij-todo.wasm"
```

### Controls

#### Normal Mode

| Key       | Action                                    |
| --------- | ----------------------------------------- |
| `↑` / `↓` | Navigate up/down                          |
| `k` / `j` | Navigate up/down (vim-style)              |
| `Space`   | Toggle Todo (•) ⟷ Done (✓)               |
| `g`       | Grab/release item for reordering          |
| `a`       | Add new todo above current position       |
| `Enter`   | Edit current item                         |
| `Delete`  | Delete current item                       |
| `q`       | Quit plugin                               |
| `Esc`     | Exit grab mode or quit plugin             |

#### Edit Mode

| Key     | Action                              |
| ------- | ----------------------------------- |
| Type    | Replace/add text                    |
| `Enter` | Save changes and return to Normal   |
| `Esc`   | Cancel changes and return to Normal |

#### Grab Mode

| Key       | Action                                      |
| --------- | ------------------------------------------- |
| `↑` / `↓` | Move grabbed item (auto-skips completed)    |
| `k` / `j` | Move grabbed item (vim-style)               |
| `g`       | Release grabbed item                        |
| Any other | Exit grab mode                              |

## How It Works

Items maintain their original order even when marked as done. When you toggle an item back to todo, it returns to its intended position. Reordering works intuitively - grab an item with `g` and move it with arrow keys. The plugin automatically skips over completed items so every keypress produces visible movement.

## Configuration

### Global Todo File

For a single global todo file shared across all Zellij sessions, configure the plugin with an absolute path:

```kdl
shared_except "locked" {
    bind "Ctrl t" {
        LaunchOrFocusPlugin "file:~/.config/zellij/plugins/zellij-todo.wasm" {
            floating true
            cwd "/home/username"  # or any absolute path you prefer
            filename ".zellij_todos.json"
        }
    }
}
```

**Configuration Options:**
- `cwd`: Directory where the todo file will be saved (default: `/host` - current directory)
- `filename`: Name of the todo file (default: `.zellij_todos.json`)

**Examples:**
- Global todos in home directory: `cwd "/home/username"`
- Project-specific todos: `cwd "/home/username/projects"`
- System-wide todos: `cwd "/etc/zellij"` (requires permissions)

### Keybinding Setup

Add to your Zellij configuration (`~/.config/zellij/config.kdl`):

```kdl
shared_except "locked" {
    bind "Ctrl t" {
        LaunchOrFocusPlugin "file:~/.config/zellij/plugins/zellij-todo.wasm" {
            floating true
        }
    }
}
```

### Layout Integration

```kdl
floating_panes {
    pane {
        name "todo"
        x "10%"
        y "10%"
        width "80%"
        height "80%"
        plugin location="file:~/.config/zellij/plugins/zellij-todo.wasm"
    }
}
```

## Development

```bash
# Build plugin
cargo build --release

# Development with auto-reload
zellij -l zellij.kdl
```

## Troubleshooting

**Plugin doesn't load:** Ensure `wasm32-wasip1` target with `rustup target add wasm32-wasip1`

**Items don't persist:** Plugin saves to `.zellij_todos.json` in the current directory - check write permissions

**Movement feels off:** Use grab mode (`g`) for reordering, arrow keys for navigation
