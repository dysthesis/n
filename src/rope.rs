use ropey::Rope;
use tower_lsp::lsp_types::Position;

pub trait RopeLspExt {
    fn lsp_pos_to_char(&self, pos: Position) -> usize;
    fn char_to_lsp_pos(&self, char_idx: usize) -> Position;
}

impl RopeLspExt for Rope {
    /// Convert an LSP Position (UTF-16 based) into a Rope char index.
    fn lsp_pos_to_char(&self, pos: Position) -> usize {
        // Get the index (in chars) of the start of the given line.
        let line_start_char = self.line_to_char(pos.line as usize);
        // Iterate over the line’s chars, accumulating UTF-16 length.
        let mut utf16_units = 0;
        let line = self.line(pos.line as usize);
        for (i, ch) in line.chars().enumerate() {
            if utf16_units == pos.character as usize {
                return line_start_char + i;
            }
            utf16_units += ch.len_utf16();
        }
        // If the requested character is past EOL, clamp to line end.
        line_start_char + line.len_chars()
    }
    // Convert a Rope char index to an LSP `Position` (UTF-16 code units).
    fn char_to_lsp_pos(&self, char_idx: usize) -> Position {
        // Which line is this?
        let line = self.char_to_line(char_idx);
        // What char index is the start of that line?
        let line_start_char = self.line_to_char(line);
        // How many UTF-16 units up to the offset and line start?
        let utf16_offset = self.char_to_utf16_cu(char_idx);
        let utf16_line = self.char_to_utf16_cu(line_start_char);

        Position {
            line: line as u32,
            character: (utf16_offset - utf16_line) as u32,
        }
    }
}
