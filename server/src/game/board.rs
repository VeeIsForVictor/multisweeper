use rand::random_bool;

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
    fn new(is_mine: bool) -> Self {
        Cell {
            is_mine,
            is_revealed: false,
            is_flagged: false,
            adjacent_mines: 0
        }
    }
}