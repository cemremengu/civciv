use arrow::{
    array::RecordBatch,
    error::ArrowError,
    util::display::{ArrayFormatter, FormatOptions},
};
use comfy_table::{Cell, Table};
use duckdb::Connection;

use ratatui::widgets::ScrollbarState;

pub enum InputMode {
    Normal,
    Editing,
}

pub struct App<'a> {
    pub input: String,
    pub cursor_position: usize,
    pub input_mode: InputMode,
    pub data: Vec<RecordBatch>,
    pub vertical_scroll_state: ScrollbarState,
    pub vertical_scroll: usize,
    db: &'a Connection,
}

impl<'a> App<'a> {
    pub fn new(db: &'a Connection) -> Self {
        Self {
            input: String::new(),
            input_mode: InputMode::Normal,
            cursor_position: 0,
            data: vec![],
            vertical_scroll_state: ScrollbarState::default(),
            vertical_scroll: 0,
            db,
        }
    }

    pub fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.cursor_position.saturating_sub(10);
        self.cursor_position = self.clamp_cursor(cursor_moved_left);
    }

    pub fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.cursor_position.saturating_add(10);
        self.cursor_position = self.clamp_cursor(cursor_moved_right);
    }

    pub fn enter_char(&mut self, new_char: char) {
        self.input.insert(self.cursor_position, new_char);

        self.move_cursor_right();
    }

    pub fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.cursor_position != 0;
        if is_not_cursor_leftmost {
            // Method "remove" is not used on the saved text for deleting the selected char.
            // Reason: Using remove on String works on bytes instead of the chars.
            // Using remove would require special care because of char boundaries.

            let current_index = self.cursor_position;
            let from_left_to_current_index = current_index - 1;

            // Getting all characters before the selected character.
            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            // Getting all characters after selected character.
            let after_char_to_delete = self.input.chars().skip(current_index);

            // Put all characters together except the selected one.
            // By leaving the selected one out, it is forgotten and therefore deleted.
            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    pub fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.len())
    }

    pub fn reset_cursor(&mut self) {
        self.cursor_position = 0;
    }

    pub fn submit_sql(&mut self) {
        let mut stmt = self.db.prepare(self.input.as_str()).unwrap();

        self.data = stmt.query_arrow([]).unwrap().collect();

        self.input.clear();
        self.reset_cursor();
    }

    pub fn data_to_table(&self) -> Result<Table, ArrowError> {
        let options = FormatOptions::default().with_display_error(true);

        let mut table = Table::new();
        table.load_preset("||--+-++|    ++++++");

        if self.data.is_empty() {
            return Ok(table);
        }

        let schema = self.data[0].schema();

        let mut header = Vec::new();
        for field in schema.fields() {
            header.push(Cell::new(field.name()));
        }
        table.set_header(header);

        for batch in self.data.iter() {
            let formatters = batch
                .columns()
                .iter()
                .map(|c| ArrayFormatter::try_new(c.as_ref(), &options))
                .collect::<Result<Vec<_>, ArrowError>>()?;

            for row in 0..batch.num_rows() {
                let mut cells = Vec::new();
                for formatter in &formatters {
                    cells.push(Cell::new(formatter.value(row)));
                }
                table.add_row(cells);
            }
        }

        Ok(table)
    }
}
