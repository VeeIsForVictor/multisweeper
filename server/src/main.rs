mod game;

use game::*;

fn main() {
    let mut game = Game::new(GameDifficulty::EASY);
    println!("{}", game);
    game.handle_action(GameAction::REVEAL { x: 0, y: 0 })
        .unwrap();
    println!("{}", game);
}
