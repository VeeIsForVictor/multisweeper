mod game;

use game::*;

fn main() {
    let game = Game::new(
        GameDifficulty::EASY
    );
    println!("{}", game);
}
