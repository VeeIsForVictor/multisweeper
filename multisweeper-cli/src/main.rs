use multisweeper_core::{Game, GameAction, GameActionResult, GameDifficulty};
use rand::{Rng, random, rng};
use thiserror::Error;

#[derive(Debug, Error)]
enum CommandError {
    #[error("cannot coerce command `{0:?}` into desired type")]
    InvalidCoerce(Command)
}

#[derive(Debug, Clone, Copy)]
enum Command {
    Reveal { x: u8, y: u8 },
    Flag { x: u8, y: u8 },
    Quit,
}

impl Command {
    fn to_game_action(&self) -> Result<GameAction, CommandError> {
        match self {
            Self::Reveal { x, y } => Ok(GameAction::Reveal { x: x - 1, y: y - 1 }),
            Self::Flag { x, y } => Ok(GameAction::Flag { x: x - 1, y: y - 1 }),
            cmd => Err(CommandError::InvalidCoerce(*cmd)),
        }
    }
}

fn render_game(game: &Game) {
    print!("\n{} x {} ({})\n", game.info().width, game.info().height, game.info().seed);
    let board = &game.snapshot().board;
    board.iter().for_each(
        |row| {
            row.iter().for_each(
                |col| {
                    match col {
                        multisweeper_core::GameCell::HiddenCell => print!("*"),
                        multisweeper_core::GameCell::VisibleCell(adj) => print!("{}", adj),
                        multisweeper_core::GameCell::FlaggedCell => print!("F"),
                        multisweeper_core::GameCell::MinedCell => print!("X"),
                    }
                }
            );
            print!("\n");
        }
    );
}

pub fn play_game(seed: u64) -> anyhow::Result<()> {
    let mut game = Game::new(GameDifficulty::TEST, random())?;

    println!("Welcome to Multisweeper [test]!");

    loop {
        render_game(&game);

        let command = read_command();

        println!("{:?}", command);

        if let Command::Quit = command {
            return Ok(());
        }

        let result = game.handle_action(command.to_game_action().unwrap());
        print!("{}[2J", 27 as char);

        let Ok(phase) = result else { continue };

        match phase.action_result {
            GameActionResult::Won => {
                println!("you won!\n");
                render_game(&game);
                break;
            }
            GameActionResult::Eliminated => {
                game.lose_game();
                println!("you lost!\n");
                render_game(&game);
                break;
            }
            GameActionResult::Stalled => {
                println!("invalid move");
            }
            GameActionResult::Applied | GameActionResult::Started => (),
        }
    }
    Ok(())
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

fn main() -> anyhow::Result<()> {
    Ok(play_game(rng().next_u64())?)
}