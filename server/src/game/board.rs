#[derive(Debug)]
pub struct Board {
    width: u8,
    height: u8,
    number_of_mines: u8,
    cells: Vec<Vec<Cell>>
}

impl Board {
    pub fn new(width: u8, height: u8, number_of_mines: u8) -> Self {
        let mut cells = vec![];
        for _i in 0..height {
            let mut row = vec![];
            for _j in 0..width {
                row.push(Cell::new());
            }
            cells.push(row);
        }
        
        Board {
            width,
            height,
            number_of_mines,
            cells
        }
    }
}

#[derive(Debug)]
struct Cell {
    is_mine: bool,
    is_revealed: bool,
    is_flagged: bool,
    adjacent_mines: u8
}

impl Cell {
    fn new() -> Self {
        Cell {
            is_mine: false,
            is_revealed: false,
            is_flagged: false,
            adjacent_mines: 0
        }
    }
}