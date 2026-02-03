# Rust TUI Development with ratatui

## Overview

ratatui is an immediate-mode TUI library. You describe what the UI should look like each frame, and ratatui handles the diff/render.

## Core Concepts

### Frame Rendering
```rust
terminal.draw(|frame| {
    // frame.area() gives you the full terminal size
    let area = frame.area();
    
    // Render widgets to specific areas
    frame.render_widget(some_widget, area);
})?;
```

### Layouts
```rust
use ratatui::prelude::*;

// Split area horizontally
let chunks = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([
        Constraint::Percentage(30),
        Constraint::Percentage(70),
    ])
    .split(area);

// chunks[0] = left panel
// chunks[1] = right panel
```

### Common Widgets

**Paragraph** — Text display
```rust
let text = Paragraph::new("Hello, world!")
    .block(Block::default().borders(Borders::ALL).title("Title"))
    .wrap(Wrap { trim: true });
```

**List** — Selectable items
```rust
let items: Vec<ListItem> = data.iter()
    .map(|d| ListItem::new(d.name.clone()))
    .collect();

let list = List::new(items)
    .block(Block::default().borders(Borders::ALL))
    .highlight_style(Style::default().fg(Color::Yellow));

// Use StatefulWidget for selection tracking
frame.render_stateful_widget(list, area, &mut list_state);
```

**Block** — Borders and titles
```rust
Block::default()
    .borders(Borders::ALL)
    .border_type(BorderType::Rounded)
    .border_style(Style::default().fg(Color::Cyan))
    .title(" My Panel ")
```

### Styling
```rust
Style::default()
    .fg(Color::Cyan)           // Foreground color
    .bg(Color::Black)          // Background color
    .add_modifier(Modifier::BOLD)
    .add_modifier(Modifier::ITALIC)
```

### Event Loop Pattern
```rust
loop {
    // 1. Render
    terminal.draw(|f| ui(f, &app))?;
    
    // 2. Handle input
    if let Event::Key(key) = event::read()? {
        if key.kind == KeyEventKind::Press {
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Down => app.next(),
                KeyCode::Up => app.previous(),
                _ => {}
            }
        }
    }
}
```

## Best Practices

1. **Separate state from rendering** — Keep UI state in a struct, render functions just read it
2. **Use constraints wisely** — `Min`, `Max`, `Percentage`, `Ratio`, `Length`
3. **Handle terminal resize** — Your layout should adapt to `frame.area()`
4. **Cleanup on exit** — Always restore terminal state in a finally block

## Common Patterns

### Popup/Modal
```rust
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

// Clear background before drawing popup
frame.render_widget(Clear, popup_area);
frame.render_widget(popup_widget, popup_area);
```

### Three-Panel Layout (like Feedo)
```rust
let main_layout = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([
        Constraint::Percentage(20),  // Sidebar
        Constraint::Percentage(30),  // List
        Constraint::Percentage(50),  // Content
    ])
    .split(area);
```

## Resources

- [ratatui book](https://ratatui.rs/)
- [ratatui examples](https://github.com/ratatui/ratatui/tree/main/examples)
