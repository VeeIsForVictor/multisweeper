mod game;

use game::*;

#[derive(Debug)]
enum Command {
    Reveal { x: u8, y: u8 },
    Flag { x: u8, y: u8 },
    Quit,
}

#[derive(Debug)]
struct CommandError;

impl Command {
    fn to_game_action(&self) -> Result<GameAction, CommandError> {
        match self {
            Self::Reveal { x, y } => Ok(GameAction::REVEAL { x: x - 1, y: y - 1 }),
            Self::Flag { x, y } => Ok(GameAction::FLAG { x: x - 1, y: y - 1 }),
            _ => Err(CommandError),
        }
    }
}

#[tracing::instrument]
fn main() {
    tracing_forest::init();

    let mut game = Game::new(GameDifficulty::TEST);

    println!("Welcome to Multisweeper [test]!");

    loop {
        println!("{}", game);

        let command = read_command();

        println!("{:?}", command);

        if let Command::Quit = command {
            return;
        }

        let result = game.handle_action(command.to_game_action().unwrap());
        print!("{}[2J", 27 as char);

        let Ok(phase) = result else { continue };

        match phase {
            GamePhase::WON => {
                println!("you won!\n{}", game);
                break;
            }
            GamePhase::LOST => {
                game.lose_game();
                println!("you lost!\n{}", game);
                break;
            }
            GamePhase::PLAYING => (),
            GamePhase::STALLED => {
                println!("invalid move");
            }
        }
    }
}

fn read_command() -> Command {
    println!(
        "'r [x] [y]' to reveal a tile\n'f [x] [y]' to flag a tile\n'q' to quit\nNote that (x, y) input is 1-indexed from top-left"
    );
    loop {
        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() {
            continue;
        }

        let mut parts = input.split_whitespace();
        let cmd = match parts.next() {
            Some(c) => c,
            None => continue,
        };

        match cmd {
            "q" => return Command::Quit,
            "r" | "f" => {
                let x = match parts.next().and_then(|s| s.parse::<u8>().ok()) {
                    Some(v) => v,
                    None => continue,
                };
                let y = match parts.next().and_then(|s| s.parse::<u8>().ok()) {
                    Some(v) => v,
                    None => continue,
                };
                return if cmd == "r" {
                    Command::Reveal { x, y }
                } else {
                    Command::Flag { x, y }
                };
            }
            _ => continue,
        }
    }
}
