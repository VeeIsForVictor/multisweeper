use rand::{random_bool, random_range};

#[derive(Debug)]
pub struct Board {
    width: u8,
    height: u8,
    created_mines: Vec<(u8, u8)>,
    cells: Vec<Vec<Cell>>
}

impl Board {
    fn evaluate_cells(&mut self) {
        let width = self.width;
        let height = self.height;
        let evaluate_neighbors = |cells: &Vec<Vec<Cell>>, row: isize, col: isize| {
            for dy in [-1, 0, 1] {
                for dx in [-1, 0, 1] {
                    let target_col = dy + col;
                    let target_row = dx + row;
                    if (dx, dy) != (0, 0) 
                        && (self.height.into() > target_col && target_col > 0) 
                        && (self.width.into() > target_row && target_row > 0) {
                            cells[target_col as usize][target_row as usize].adjacent_mines += 1;
                    }
                }
            }
        };

        for row_idx in 0..self.height {
            for col_idx in 0..self.width {
                let adjacent = evaluate_neighbors(&self.cells, row_idx as isize, col_idx as isize);
                let cell = &mut self.cells[row_idx as usize][col_idx as usize];
                cell.adjacent_mines = adjacent;
            }
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