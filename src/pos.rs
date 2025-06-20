use std::{fs, num::TryFromIntError, ops::Range, path::PathBuf};

use serde::Serialize;
use tower_lsp::lsp_types::{Position, PositionEncodingKind};

// We define strong type aliases here to prevent mixups
// https://stackoverflow.com/a/69443823

/// The offset of the element from the start of the file in terms of bytes
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct Offset(usize);
impl From<Offset> for usize {
    fn from(value: Offset) -> Self {
        let Offset(val) = value;
        val
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct Row(usize);
impl From<Row> for usize {
    fn from(value: Row) -> Self {
        let Row(val) = value;
        val
    }
}
impl From<Position> for Row {
    fn from(value: Position) -> Self {
        Row(value.line as usize)
    }
}

impl From<Position> for Col {
    fn from(value: Position) -> Self {
        Col(value.character as usize)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct Col(usize);
impl From<Col> for usize {
    fn from(value: Col) -> Self {
        let Col(val) = value;
        val
    }
}

/// Helper to map between byte ranges and row/col ranges
pub struct PosMapper {
    text: String,
    /// Bytes where new lines begin
    line_starts: Vec<usize>,
    encoding: PositionEncodingKind,
}

impl PosMapper {
    pub fn new(text: String, encoding: PositionEncodingKind) -> Self {
        let line_starts = text
            .as_bytes()
            .iter()
            .enumerate()
            .filter(|&(_, &b)| b == b'\n')
            // Get the character after the newline
            .map(|(i, _)| i + 1)
            .collect();

        Self {
            text,
            line_starts,
            encoding,
        }
    }
    /// Converts a `Position` to a byte offset (`usize`).
    ///
    /// Returns `None` if the position's line is out of bounds.
    /// Correctly clamps character offsets that are beyond the end of a line.
    #[allow(unused)]
    pub fn position_to_offset(&self, pos: &Position) -> Result<usize, PositionError> {
        let line = pos.line.try_into().map_err(|e: TryFromIntError| {
            PositionError::ConversionFromU32ToUSizeFailed {
                value: pos.line,
                reason: e.to_string(),
            }
        })?;
        let line_start = *self
            .line_starts
            .get(line)
            .ok_or(PositionError::LineNotFound { line })?;

        // The LSP spec says if the character offset is greater than the line
        // length, it defaults to the line length. We find the line's text
        // and then perform the conversion, which naturally handles this.
        let line_end = self
            .line_starts
            .get(pos.line as usize + 1)
            .map_or(self.text.len(), |&end| end);

        let line_text = &self.text[line_start..line_end];

        let char_offset_bytes = if self.encoding == PositionEncodingKind::UTF16 {
            let mut utf16_offset = 0;
            let mut byte_offset = 0;
            for ch in line_text.chars() {
                if utf16_offset >= pos.character as usize {
                    break;
                }
                // A single `char` can be one or two UTF-16 code units.
                utf16_offset += ch.encode_utf16(&mut [0; 2]).len();
                byte_offset += ch.len_utf8();
            }
            byte_offset
        }
        //  The LSP spec for UTF-8 and UTF-32 defines the `character` offset as a count of
        //  Unicode scalar values (`char` in Rust). It is NOT a byte offset for UTF-8.
        else if self.encoding == PositionEncodingKind::UTF8
            || self.encoding == PositionEncodingKind::UTF32
        {
            line_text
                .chars()
                .take(pos.character as usize)
                .map(|c| c.len_utf8())
                .sum::<usize>()
        } else {
            return Err(PositionError::UnknownEncoding {
                encoding: self.encoding.clone(),
            });
        };

        // The final offset is the start of the line plus the calculated byte
        // offset within that line. We clamp to `line_end` just in case,
        // although the logic above should prevent exceeding it.
        Ok((line_start + char_offset_bytes).min(line_end))
    }

    /// Converts a byte offset (`usize`) to an LSP `Position`.
    ///
    /// Returns `None` if the offset is out of bounds of the document text.
    pub fn offset_to_position(&self, offset: usize) -> Result<(Row, Col), PositionError> {
        if offset > self.text.len() {
            return Err(PositionError::OffsetOutOfRange {
                offset,
                length: self.text.len(),
            });
        }

        // `partition_point` is a highly efficient way to find the line number of the offset. It's
        // a binary search for the last line start <= offset.
        // Find the line number the cursor is at...
        let line_start_idx = self.line_starts.partition_point(|&start| start <= offset) /* convert from 1-index to 0-index */ - 1;
        // ...and the first character of that line.
        let line_start = self.line_starts[line_start_idx];

        // The text from the start of the line up to the target offset. We will use it to calculate
        // the column, as it may depend on the UTF encoding.
        let text_before_offset_in_line = &self.text[line_start..offset];

        let character = if self.encoding == PositionEncodingKind::UTF16 {
            text_before_offset_in_line.encode_utf16().count()
        }
        // For both UTF-8 and UTF-32, the `character` field
        // is the number of Rust `char`s (Unicode scalar values).
        else if self.encoding == PositionEncodingKind::UTF8
            || self.encoding == PositionEncodingKind::UTF32
        {
            text_before_offset_in_line.chars().count()
        } else {
            return Err(PositionError::UnknownEncoding {
                encoding: self.encoding.clone(),
            });
        };

        // TODO: Figure out why I can only adjust the row index here, and not above where
        // `line_start_idx` was defined.
        Ok((Row(line_start_idx + 1), Col(character)))
    }
}

/// The position of a text element
#[derive(Debug, Serialize, Clone, PartialEq, Hash, Eq)]
pub struct Pos {
    offset_range: Range<Offset>,
    row_range: Range<Row>,
    col_range: Range<Col>,
}

#[derive(thiserror::Error, Debug)]
pub enum PositionError {
    #[error("cannot open file `{}` because {}", path.to_string_lossy(), reason)]
    CannotOpenFile { path: PathBuf, reason: String },
    #[error("unknown encoding: `{}`", encoding.as_str())]
    UnknownEncoding { encoding: PositionEncodingKind },
    #[error("the offset `{offset}` is out of range; the document has a length of {length}")]
    OffsetOutOfRange { offset: usize, length: usize },
    #[error("the line `{line}` does not exist")]
    LineNotFound { line: usize },
    #[error("failed to convert from the u32 value {value} to usize because {reason}")]
    ConversionFromU32ToUSizeFailed { value: u32, reason: String },
}

impl Pos {
    pub fn new(
        offset_range: Range<usize>,
        path: &PathBuf,
        encoding: PositionEncodingKind,
    ) -> Result<Self, PositionError> {
        let text = fs::read_to_string(path).map_err(|e| PositionError::CannotOpenFile {
            path: path.clone(),
            reason: e.to_string(),
        })?;
        let mapper = PosMapper::new(text, encoding);
        let position_range = mapper.offset_to_position(offset_range.start)?
            ..mapper.offset_to_position(offset_range.end)?;

        let row_start = position_range.start.0.into();
        let row_end = position_range.end.0.into();
        let row_range = Row(row_start)..Row(row_end);

        let col_start = position_range.start.1.into();
        let col_end = position_range.end.1.into();
        let col_range = Col(col_start)..Col(col_end);
        let offset_range = Offset(offset_range.start)..Offset(offset_range.end);
        Ok(Self {
            offset_range,
            row_range,
            col_range,
        })
    }

    #[allow(unused)]
    pub fn offset_range(&self) -> Range<Offset> {
        self.offset_range.clone()
    }

    pub fn col_range(&self) -> Range<Col> {
        self.col_range.clone()
    }
    pub fn row_range(&self) -> Range<Row> {
        self.row_range.clone()
    }
}
