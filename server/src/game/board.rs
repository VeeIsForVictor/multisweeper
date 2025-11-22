use std::fmt::Display;

use rand::{random_bool, random_range};

#[derive(Debug)]
pub struct Board {
    width: u8,
    height: u8,
    created_mines: Vec<(u8, u8)>,
    cells: Vec<Vec<Cell>>
}

impl Display for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let _ = f.write_fmt(format_args!("{} x {}", self.width, self.height));
        for row in &self.cells {
            for col in row {
                let _ = write!(f, "{col}");
            }
            let _ = write!(f, "\n");
        }
        Ok(())
    }
}

impl Board {
    fn evaluate_cells(&mut self) {
        let mut evaluate_neighbors = |row: u8, col: u8| {
            for dy in [-1, 0, 1] {
                for dx in [-1, 0, 1] {
                    let target_col: isize = dy + col as isize;
                    let target_row: isize = dx + row as isize;
                    if (dx, dy) != (0, 0) 
                        && (self.height > target_col as u8 && target_col >= 0) 
                        && (self.width > target_row as u8 && target_row >= 0) {
                            self.cells[target_col as usize][target_row as usize].adjacent_mines += 1;
                    }
                }
            }
        };

        for (row_idx, col_idx) in &self.created_mines {
            evaluate_neighbors(*row_idx, *col_idx);
        }
    }

    pub fn new(width: u8, height: u8, number_of_mines: u8) -> Self {
        let mut cells = vec![];
        let mut created_mines: Vec<(u8, u8)> = vec![];
        for i in 0..number_of_mines {
            created_mines.push((random_range(0..width), random_range(0..height)));
        }

        for y in 0..height {
            let mut row = vec![];
            for x in 0..width {
                let is_mine = created_mines.contains(&(x, y));
                row.push(Cell::new(is_mine));
            }
            cells.push(row);
        }
        
        let mut board= Board {
            width,
            height,
            created_mines,
            cells
        };
        
        board.evaluate_cells();

        return board;
    }
}

#[derive(Debug)]
struct Cell {
    pub is_mine: bool,
    pub adjacent_mines: u8,
    is_revealed: bool,
    is_flagged: bool,
}

impl Display for Cell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_flagged {
            f.write_str("F")
        } else if self.is_mine {
            f.write_str("*")
        } else if self.adjacent_mines > 0 {
            f.write_fmt(format_args!("{}", self.adjacent_mines))
        } else {
            f.write_str(".")
        }
    }
}

impl Cell {
    fn new(is_mine: bool) -> Self {
        Cell {
            is_mine,
            is_revealed: false,
            is_flagged: false,
            adjacent_mines: 0
        }
    }
}