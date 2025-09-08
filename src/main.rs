use zellij_tile::prelude::*;
use std::collections::BTreeMap;
use std::io::{self, Write};
use serde::{Serialize, Deserialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
struct TodoItem {
    text: String,
    done: bool,
    id: usize,
    display_order: usize,
}

#[derive(Debug, Clone, PartialEq)]
enum Mode {
    Normal,
    Edit,
}

#[derive(Default)]
struct State {
    items: Vec<TodoItem>,
    selected_index: usize,
    next_id: usize,
    next_display_order: usize,
    mode: Mode,
    edit_buffer: String,
    grabbed_item_id: Option<usize>,
    rows: usize,
    cols: usize,
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Normal
    }
}

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self, _configuration: BTreeMap<String, String>) {
        subscribe(&[EventType::Key, EventType::CustomMessage]);
        
        // Set terminal title that Zellij will use as pane name
        print!("\x1b]0;TODO\x07");
        io::stdout().flush().unwrap();
        
        // Load persisted todos from file system if available
        self.load_todos();
    }

    fn update(&mut self, event: Event) -> bool {
        let mut should_render = false;

        match event {
            Event::Key(key) => {
                should_render = self.handle_key(key);
            }
            Event::CustomMessage(message, _) => {
                // Handle plugin toggle - if we receive a message while visible, close
                if message.contains("toggle") {
                    hide_self();
                    return false;
                }
            }
            _ => {}
        }

        should_render
    }

    fn render(&mut self, rows: usize, cols: usize) {
        self.rows = rows;
        self.cols = cols;

        // Clear screen
        print!("\x1b[2J\x1b[H");

        if self.items.is_empty() && self.mode == Mode::Normal {
            self.render_empty_state();
        } else {
            self.render_todo_list();
        }
    }
}

impl State {
    fn handle_key(&mut self, key: KeyWithModifier) -> bool {
        match self.mode {
            Mode::Normal => self.handle_normal_mode_key(key),
            Mode::Edit => self.handle_edit_mode_key(key),
        }
    }

    fn handle_normal_mode_key(&mut self, key: KeyWithModifier) -> bool {
        if self.items.is_empty() {
            // Special handling when no items exist
            match key.bare_key {
                BareKey::Char('a') if key.has_no_modifiers() => {
                    self.add_new_item();
                    return true;
                }
                BareKey::Char('q') if key.has_no_modifiers() => {
                    hide_self();
                    return false;
                }
                _ => return false,
            }
        }

        match key.bare_key {
            // Navigation
            BareKey::Up if key.has_no_modifiers() => {
                if self.grabbed_item_id.is_some() {
                    self.move_grabbed_item_up();
                } else if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
                true
            }
            BareKey::Down if key.has_no_modifiers() => {
                if self.grabbed_item_id.is_some() {
                    self.move_grabbed_item_down();
                } else if self.selected_index < self.items.len().saturating_sub(1) {
                    self.selected_index += 1;
                }
                true
            }
            
            // Vim-style navigation
            BareKey::Char('k') if key.has_no_modifiers() => {
                if self.grabbed_item_id.is_some() {
                    self.move_grabbed_item_up();
                } else if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
                true
            }
            BareKey::Char('j') if key.has_no_modifiers() => {
                if self.grabbed_item_id.is_some() {
                    self.move_grabbed_item_down();
                } else if self.selected_index < self.items.len().saturating_sub(1) {
                    self.selected_index += 1;
                }
                true
            }

            // Toggle completion (or exit grab mode)
            BareKey::Char(' ') if key.has_no_modifiers() => {
                if self.grabbed_item_id.is_some() {
                    self.grabbed_item_id = None;
                } else {
                    self.toggle_current_item();
                }
                true
            }

            // Grab/release item for reordering
            BareKey::Char('g') if key.has_no_modifiers() => {
                self.toggle_grab();
                true
            }

            // Add new item (or exit grab mode)
            BareKey::Char('a') if key.has_no_modifiers() => {
                if self.grabbed_item_id.is_some() {
                    self.grabbed_item_id = None;
                } else {
                    self.add_new_item();
                }
                true
            }

            // Edit current item (or release grab if in grab mode)
            BareKey::Enter if key.has_no_modifiers() => {
                if self.grabbed_item_id.is_some() {
                    self.grabbed_item_id = None;
                } else {
                    self.start_editing_current();
                }
                true
            }

            // Delete current item (or exit grab mode)
            BareKey::Delete if key.has_no_modifiers() => {
                if self.grabbed_item_id.is_some() {
                    self.grabbed_item_id = None;
                } else {
                    self.delete_current_item();
                }
                true
            }
            BareKey::Backspace if key.has_no_modifiers() => {
                if self.grabbed_item_id.is_some() {
                    self.grabbed_item_id = None;
                } else {
                    self.delete_current_item();
                }
                true
            }

            // Quit
            BareKey::Char('q') if key.has_no_modifiers() => {
                hide_self();
                false
            }
            BareKey::Esc if key.has_no_modifiers() => {
                if self.grabbed_item_id.is_some() {
                    self.grabbed_item_id = None;
                    true
                } else {
                    hide_self();
                    false
                }
            }
            // Handle Ctrl+k to close (same as opening keybind)
            BareKey::Char('k') if key.has_modifiers(&[KeyModifier::Ctrl]) => {
                hide_self();
                false
            }

            _ => false,
        }
    }

    fn handle_edit_mode_key(&mut self, key: KeyWithModifier) -> bool {
        match key.bare_key {
            // Save and exit edit mode
            BareKey::Enter if key.has_no_modifiers() => {
                self.save_edit();
                true
            }

            // Cancel edit mode
            BareKey::Esc if key.has_no_modifiers() => {
                self.cancel_edit();
                true
            }

            // Backspace
            BareKey::Backspace if key.has_no_modifiers() => {
                self.edit_buffer.pop();
                true
            }

            // Type characters
            BareKey::Char(c) if key.has_no_modifiers() => {
                self.edit_buffer.push(c);
                true
            }

            _ => false,
        }
    }

    fn toggle_current_item(&mut self) {
        if let Some(item) = self.items.get_mut(self.selected_index) {
            let original_cursor_position = self.selected_index;
            item.done = !item.done;
            self.sort_items();
            
            // Keep cursor at the same visual position instead of following the moved item
            self.selected_index = std::cmp::min(original_cursor_position, self.items.len().saturating_sub(1));
            
            self.save_todos();
        }
    }

    fn toggle_grab(&mut self) {
        if let Some(grabbed_id) = self.grabbed_item_id {
            // Release the grabbed item
            if grabbed_id == self.get_current_item_id() {
                self.grabbed_item_id = None;
            }
        } else {
            // Grab the current item
            self.grabbed_item_id = Some(self.get_current_item_id());
        }
    }

    fn get_current_item_id(&self) -> usize {
        self.items.get(self.selected_index).map(|item| item.id).unwrap_or(0)
    }

    fn move_grabbed_item_up(&mut self) {
        if let Some(grabbed_id) = self.grabbed_item_id {
            let grabbed_item_done = self.items.iter().find(|item| item.id == grabbed_id).map(|item| item.done).unwrap_or(false);
            
            loop {
                // Work with logical ordering (by display_order) - always move by 1 position regardless of status
                let mut logical_items: Vec<_> = self.items.iter().enumerate().collect();
                logical_items.sort_by(|a, b| a.1.display_order.cmp(&b.1.display_order));
                
                if let Some(logical_pos) = logical_items.iter().position(|(_, item)| item.id == grabbed_id) {
                    if logical_pos > 0 {
                        // Always swap with the item directly above in logical order
                        let current_item_idx = logical_items[logical_pos].0;
                        let target_item_idx = logical_items[logical_pos - 1].0;
                        let target_item_done = self.items[target_item_idx].done;
                        
                        let temp_display_order = self.items[current_item_idx].display_order;
                        self.items[current_item_idx].display_order = self.items[target_item_idx].display_order;
                        self.items[target_item_idx].display_order = temp_display_order;
                        
                        // Re-sort to reflect the new logical order
                        self.sort_items();
                        
                        // If we swapped with an item of the same status, we're done (visual change occurred)
                        if grabbed_item_done == target_item_done {
                            break;
                        }
                        // Otherwise, continue moving up (crossed over a done item)
                    } else {
                        break; // Can't move up anymore
                    }
                } else {
                    break; // Item not found
                }
            }
            self.save_todos();
        }
    }

    fn move_grabbed_item_down(&mut self) {
        if let Some(grabbed_id) = self.grabbed_item_id {
            let grabbed_item_done = self.items.iter().find(|item| item.id == grabbed_id).map(|item| item.done).unwrap_or(false);
            
            loop {
                // Work with logical ordering (by display_order) - always move by 1 position regardless of status
                let mut logical_items: Vec<_> = self.items.iter().enumerate().collect();
                logical_items.sort_by(|a, b| a.1.display_order.cmp(&b.1.display_order));
                
                if let Some(logical_pos) = logical_items.iter().position(|(_, item)| item.id == grabbed_id) {
                    if logical_pos < logical_items.len() - 1 {
                        // Always swap with the item directly below in logical order
                        let current_item_idx = logical_items[logical_pos].0;
                        let target_item_idx = logical_items[logical_pos + 1].0;
                        let target_item_done = self.items[target_item_idx].done;
                        
                        let temp_display_order = self.items[current_item_idx].display_order;
                        self.items[current_item_idx].display_order = self.items[target_item_idx].display_order;
                        self.items[target_item_idx].display_order = temp_display_order;
                        
                        // Re-sort to reflect the new logical order
                        self.sort_items();
                        
                        // If we swapped with an item of the same status, we're done (visual change occurred)
                        if grabbed_item_done == target_item_done {
                            break;
                        }
                        // Otherwise, continue moving down (crossed over a done item)
                    } else {
                        break; // Can't move down anymore
                    }
                } else {
                    break; // Item not found
                }
            }
            self.save_todos();
        }
    }

    fn add_new_item(&mut self) {
        let new_item = TodoItem {
            text: String::new(),
            done: false,
            id: self.next_id,
            display_order: self.next_display_order,
        };
        self.next_id += 1;
        self.next_display_order += 1;

        if self.items.is_empty() {
            // If list is empty, just add the first item
            self.items.push(new_item);
            self.selected_index = 0;
        } else {
            // Check if cursor is currently on a completed item
            let current_item_is_done = self.items.get(self.selected_index).map(|item| item.done).unwrap_or(false);
            
            let insert_pos = if current_item_is_done {
                // Cursor is on completed item - snap to end of todo section
                self.items.iter().position(|item| item.done).unwrap_or(self.items.len())
            } else {
                // Cursor is on todo item - insert above current position
                self.selected_index
            };
            
            self.items.insert(insert_pos, new_item);
            self.selected_index = insert_pos;
        }

        self.start_editing_current();
    }

    fn start_editing_current(&mut self) {
        if let Some(_item) = self.items.get(self.selected_index) {
            self.edit_buffer = String::new(); // Start with empty buffer for overwrite mode
            self.mode = Mode::Edit;
        }
    }

    fn save_edit(&mut self) {
        if let Some(item) = self.items.get_mut(self.selected_index) {
            item.text = self.edit_buffer.trim().to_string();
            if item.text.is_empty() {
                // Remove empty items
                self.items.remove(self.selected_index);
                if self.selected_index >= self.items.len() && !self.items.is_empty() {
                    self.selected_index = self.items.len() - 1;
                }
            }
            self.save_todos();
        }
        self.mode = Mode::Normal;
        self.edit_buffer.clear();
    }

    fn cancel_edit(&mut self) {
        // If this was a new empty item, remove it
        if let Some(item) = self.items.get(self.selected_index) {
            if item.text.is_empty() {
                self.items.remove(self.selected_index);
                if self.selected_index >= self.items.len() && !self.items.is_empty() {
                    self.selected_index = self.items.len() - 1;
                }
            }
            // If editing existing item, revert any changes by doing nothing
            // The original text remains unchanged
        }
        self.mode = Mode::Normal;
        self.edit_buffer.clear();
    }

    fn delete_current_item(&mut self) {
        if !self.items.is_empty() {
            self.items.remove(self.selected_index);
            if self.selected_index >= self.items.len() && !self.items.is_empty() {
                self.selected_index = self.items.len() - 1;
            }
            self.save_todos();
        }
    }

    fn sort_items(&mut self) {
        // Simple sort: todo items first (by display_order), then done items (by display_order)
        let current_id = self.get_current_item_id();

        self.items.sort_by(|a, b| {
            match (a.done, b.done) {
                (false, true) => std::cmp::Ordering::Less,  // Todo items come first
                (true, false) => std::cmp::Ordering::Greater, // Done items come last
                _ => a.display_order.cmp(&b.display_order),   // Within same status, sort by original order
            }
        });

        // Update selected index to follow the moved item
        if let Some(new_pos) = self.items.iter().position(|item| item.id == current_id) {
            self.selected_index = new_pos;
        }
    }


    fn render_empty_state(&self) {
        let message = "Press 'a' to add a todo";
        let y = self.rows / 2;
        let x = (self.cols.saturating_sub(message.len())) / 2;

        print!("\x1b[{};{}H\x1b[2m{}\x1b[0m", y + 1, x + 1, message);
    }

    fn render_todo_list(&self) {
        let start_row = 1;
        let available_rows = self.rows;

        // Calculate visible range
        let start_idx = if self.selected_index >= available_rows {
            self.selected_index.saturating_sub(available_rows.saturating_sub(1))
        } else {
            0
        };
        let end_idx = std::cmp::min(start_idx + available_rows, self.items.len());

        // Render visible items
        for (display_row, idx) in (start_idx..end_idx).enumerate() {
            let item = &self.items[idx];
            let row = start_row + display_row;

            // Move cursor to start of line
            print!("\x1b[{};1H", row);

            // Clear line
            print!("\x1b[K");

            // Determine styling based on state
            let (bullet, style_start, style_end) = if item.done {
                ("✓", "\x1b[2m", "\x1b[0m") // Dimmed with checkmark
            } else {
                ("•", "", "")
            };

            // Highlight selected item with underline instead of background
            let (highlight_start, highlight_end) = if idx == self.selected_index {
                if self.mode == Mode::Edit {
                    ("\x1b[4;36m", "\x1b[0m") // Underlined cyan for edit mode
                } else {
                    ("\x1b[4m", "\x1b[0m") // Simple underline for selection
                }
            } else {
                ("", "")
            };

            // Show grab indicator (minimal)
            let grab_indicator = if self.grabbed_item_id == Some(item.id) {
                "▶ "
            } else {
                "  "
            };

            // Render the item
            let display_text = if self.mode == Mode::Edit && idx == self.selected_index {
                // In edit mode: show user input or ghost text if empty
                let display_content = if self.edit_buffer.is_empty() {
                    // Show original text as faded ghost placeholder
                    format!("\x1b[2m{}\x1b[0m", item.text)
                } else {
                    // Show user input normally
                    self.edit_buffer.clone()
                };
                
                format!("{}{}{} {}{}{}",
                    grab_indicator,
                    highlight_start,
                    bullet,
                    display_content,
                    style_end,
                    highlight_end
                )
            } else {
                let max_text_width = self.cols.saturating_sub(6); // Account for grab indicator and bullet
                let truncated_text = if item.text.len() > max_text_width {
                    format!("{}…", &item.text[..max_text_width.saturating_sub(1)])
                } else {
                    item.text.clone()
                };

                format!("{}{}{}{} {}{}{}",
                    grab_indicator,
                    highlight_start,
                    style_start,
                    bullet,
                    truncated_text,
                    style_end,
                    highlight_end
                )
            };

            print!("{}", display_text);
        }
    }


    fn load_todos(&mut self) {
        // Load todos from host filesystem for global persistence
        let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let todos_path = format!("{}/.config/zellij/todos.json", home_dir);
        if let Ok(data) = std::fs::read_to_string(&todos_path) {
            if let Ok(mut loaded_items) = serde_json::from_str::<Vec<TodoItem>>(&data) {
                // Simple migration: assign display_order based on current position for items that don't have it
                for (index, item) in loaded_items.iter_mut().enumerate() {
                    if item.display_order == 0 && index > 0 {
                        item.display_order = index;
                    }
                }
                
                self.items = loaded_items;
                self.next_id = self.items.iter().map(|item| item.id).max().unwrap_or(0) + 1;
                self.next_display_order = self.items.iter().map(|item| item.display_order).max().unwrap_or(0) + 1;
                self.sort_items();
            }
        }
    }

    fn save_todos(&self) {
        // Save todos to host filesystem for global persistence
        if let Ok(data) = serde_json::to_string_pretty(&self.items) {
            let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            let config_dir = format!("{}/.config/zellij", home_dir);
            let todos_path = format!("{}/todos.json", config_dir);
            
            // Ensure the directory exists
            let _ = std::fs::create_dir_all(&config_dir);
            let _ = std::fs::write(&todos_path, data);
        }
    }
}
