use rand::random_bool;

#[derive(Debug)]
pub struct Board {
    width: u8,
    height: u8,
    number_of_mines: u8,
    cells: Vec<Vec<Cell>>
}

impl Board {
    fn evaluate_cells(&mut self) {
        let width = self.width;
        let height = self.height;
        let evaluate_cell = |cells: &Vec<Vec<Cell>>, row: isize, col: isize| {
            let mut count = 0;
            for check_row in [-1, 0, 1] {
                for check_col in [-1, 0, 1] {
                    let nr = row + check_row;
                    let nc = col + check_col;
                    if nr >= 0 && nc >= 0 && (nr < height.into()) && (nc < width.into()) {
                        count += cells
                            .get(nr as usize).unwrap()
                            .get(nc as usize).unwrap()
                            .is_mine as u8;
                    }
                }
            }
            return count;
        };

        for row_idx in 0..self.height {
            for col_idx in 0..self.width {
                let adjacent = evaluate_cell(&self.cells, row_idx as isize, col_idx as isize);
                let cell = &mut self.cells[row_idx as usize][col_idx as usize];
                cell.adjacent_mines = adjacent;
            }
        }
    }

    pub fn new(width: u8, height: u8, number_of_mines: u8) -> Self {
        let mut cells = vec![];
        let mut created_mines = 0;
        for _i in 0..height {
            let mut row = vec![];
            for _j in 0..width {
                let is_mine = if created_mines < number_of_mines { 
                    created_mines += 1;
                    random_bool(0.5)
                } else {
                    false
                };
                row.push(Cell::new(is_mine));
            }
            cells.push(row);
        }
        
        let mut board= Board {
            width,
            height,
            number_of_mines,
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