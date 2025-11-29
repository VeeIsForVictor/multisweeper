mod game;

use game::*;

fn main() {
    let mut game = Game::new(
        GameDifficulty::EASY
    );
    println!("{}", game);
    game.reveal(0, 0).unwrap();
    println!("{}", game);
}
