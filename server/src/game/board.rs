#[derive(Debug)]
pub struct Board {
    width: u8,
    height: u8,
    number_of_mines: u8,
    cells: Vec<Vec<Cell>>
}

#[derive(Debug)]
struct Cell {
    is_mine: bool,
    is_revealed: bool,
    is_flagged: bool,
    adjacent_mines: u8
}