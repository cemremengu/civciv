use std::{
    error::Error,
    io::{self},
};

use app::{App, InputMode};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use duckdb::Connection;
use ratatui::{prelude::*, widgets::*};

mod app;

fn main() -> Result<(), Box<dyn Error>> {
    let conn = Connection::open_in_memory()?;

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let app = App::new(&conn);
    let res = run_app(&mut terminal, app);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            match app.input_mode {
                InputMode::Normal => match key.code {
                    KeyCode::Char('e') => {
                        app.input_mode = InputMode::Editing;
                    }
                    KeyCode::Char('q') => {
                        return Ok(());
                    }
                    KeyCode::Down => {
                        app.vertical_scroll = app.vertical_scroll.saturating_add(1);
                        app.vertical_scroll_state =
                            app.vertical_scroll_state.position(app.vertical_scroll);
                    }
                    KeyCode::Up => {
                        if app.vertical_scroll >= 1 {
                            app.vertical_scroll = app.vertical_scroll.saturating_sub(1);
                            app.vertical_scroll_state =
                                app.vertical_scroll_state.position(app.vertical_scroll);
                        }
                    }
                    _ => {}
                },
                InputMode::Editing if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Enter => app.submit_sql(),
                    KeyCode::Char(to_insert) => {
                        app.enter_char(to_insert);
                    }
                    KeyCode::Backspace => {
                        app.delete_char();
                    }
                    KeyCode::Left => {
                        app.move_cursor_left();
                    }
                    KeyCode::Right => {
                        app.move_cursor_right();
                    }

                    KeyCode::Esc => {
                        app.input_mode = InputMode::Normal;
                    }
                    _ => {}
                },
                InputMode::Editing => {}
            }
        }
    }
}

fn ui(frame: &mut Frame, app: &mut App) {
    let vertical = Layout::vertical([Constraint::Length(3), Constraint::Min(1)]);

    let [sql_area, chart_area] = vertical.areas(frame.size());

    let input = Paragraph::new(app.input.as_str())
        .style(match app.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => Style::default().fg(Color::Yellow),
        })
        .block(Block::default().borders(Borders::ALL).title("SQL"));

    frame.render_widget(input, sql_area);

    let table = app.data_to_table().unwrap().to_string();

    app.vertical_scroll_state = app.vertical_scroll_state.content_length(table.len());

    let pretty_table = Paragraph::new(table)
        .scroll((app.vertical_scroll as u16, 0))
        .block(Block::default().borders(Borders::ALL).title("Result"));

    frame.render_widget(pretty_table, chart_area);
    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓")),
        chart_area,
        &mut app.vertical_scroll_state,
    )
}
