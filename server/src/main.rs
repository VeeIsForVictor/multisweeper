mod game;

use game::*;

#[derive(Debug)]
enum Command {
    Reveal { x: u8, y: u8 },
    Flag { x: u8, y: u8 },
    Quit
}

#[derive(Debug)]
struct CommandError; 

impl Command {
    fn to_game_action(&self) -> Result<GameAction, CommandError> {
        match self {
            Self::Reveal { x, y } => Ok(GameAction::REVEAL { x: x - 1, y: y - 1 }),
            Self::Flag { x, y } => Ok(GameAction::FLAG { x: x - 1, y: y - 1 }),
            _  => Err(CommandError)
        }
    }
}

fn main() {
    let mut game = Game::new(GameDifficulty::EASY);

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

        let Ok(phase) = result else {
            continue
        };

        match phase {
            GamePhase::WON => {
                println!("you won!\n{}", game);
                break;
            },
            GamePhase::LOST => {
                game.lose_game();
                println!("you lost!\n{}", game);
                break;
            },
            GamePhase::PLAYING => (),
            GamePhase::STALLED => {
                println!("invalid move");
            }
        }
    }
}

fn read_command() -> Command {
    println!("'r [x] [y]' to reveal a tile\n'f [x] [y]' to flag a tile\n'q' to quit\nNote that (x, y) input is 1-indexed from top-left");
    loop {
        let mut input = String::new();
        let _ = std::io::stdin().read_line(&mut input);
        
        let Some(char) = input.get(0..1) else {
            continue;
        };

        let args: Vec<&str> = input.split(" ").map(|str| str.trim()).collect();
        return match char {
            "r" => {
                if args.len() != 3 {
                    continue;
                }
                return Command::Reveal { x: args[1].parse().unwrap(), y: args[2].parse().unwrap() }
            },
            "f" => {
                if args.len() != 3 {
                    continue;
                }
                return Command::Flag { x: args[1].parse().unwrap(), y: args[2].parse().unwrap() }
            },
            "q" => Command::Quit,
            _ => continue
        }
    }
}