use std::ops::Add;

#[derive(Debug, Clone, Copy)]
pub(crate) enum RowColDiff {
    SameRow(ColOffset),
    DifferentRow(RowAndColOffset),
}

impl Default for RowColDiff {
    fn default() -> Self {
        Self::SameRow(ColOffset { col_offset: 0 })
    }
}

impl RowColDiff {
    pub fn delta_start(&self) -> u32 {
        match self {
            RowColDiff::SameRow(same_row) => same_row.col_offset as u32,
            RowColDiff::DifferentRow(diff_row) => diff_row.new_col as u32,
        }
    }

    pub fn delta_line(&self) -> u32 {
        match self {
            RowColDiff::SameRow(_) => 0,
            RowColDiff::DifferentRow(diff_row) => diff_row.row_offset as u32,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ColOffset {
    pub(crate) col_offset: usize,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RowAndColOffset {
    pub(crate) row_offset: usize,
    pub(crate) new_col: usize,
}

impl Add for RowColDiff {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (RowColDiff::SameRow(same_1), RowColDiff::SameRow(same_2)) => {
                RowColDiff::SameRow(ColOffset {
                    col_offset: same_1.col_offset + same_2.col_offset,
                })
            }
            (RowColDiff::SameRow(_), RowColDiff::DifferentRow(d)) => RowColDiff::DifferentRow(d),
            (RowColDiff::DifferentRow(diff), RowColDiff::SameRow(same)) => {
                RowColDiff::DifferentRow(RowAndColOffset {
                    row_offset: diff.row_offset,
                    new_col: diff.new_col + same.col_offset,
                })
            }
            (RowColDiff::DifferentRow(diff_1), RowColDiff::DifferentRow(diff_2)) => {
                RowColDiff::DifferentRow(RowAndColOffset {
                    row_offset: diff_1.row_offset + diff_2.row_offset,
                    new_col: diff_2.new_col,
                })
            }
        }
    }
}

pub(crate) fn diff_to_end_of_slice(source_str: &str) -> RowColDiff {
    let mut row_count = 0;
    let mut index_of_start_of_last_row = 0;
    for (index, char) in source_str.chars().enumerate() {
        // TODO we need to handle other line breaks
        if char == '\n' {
            row_count += 1;
            index_of_start_of_last_row = index;
        }
    }

    if row_count == 0 {
        RowColDiff::SameRow(ColOffset {
            col_offset: source_str.len(),
        })
    } else {
        RowColDiff::DifferentRow(RowAndColOffset {
            row_offset: row_count,
            // TODO why -1 here?
            new_col: source_str.len() - index_of_start_of_last_row - 1,
        })
    }
}

pub(crate) fn get_index_from_diff(source_str: &str, diff: RowColDiff) -> usize {
    let mut remaining_cols = diff.delta_line();
    let mut index = 0;
    if remaining_cols > 0 {
        for char in source_str.chars() {
            index += 1;
            if char == '\n' {
                remaining_cols -= 1;
                if remaining_cols == 0 {
                    break;
                }
            }
        }
    }

    let remaining_rows = diff.delta_start();

    index + remaining_rows as usize
}
