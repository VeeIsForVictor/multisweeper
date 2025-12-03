mod game;

use game::*;

#[derive(Debug)]
enum Command {
    Reveal { x: u8, y: u8 },
    Flag { x: u8, y: u8 },
    Quit
}

fn main() {
    let mut game = Game::new(GameDifficulty::EASY);

    println!("Welcome to Multisweeper [test]!");

    loop {
        println!("{}", game);

        let command = read_command();

        println!("{:?}", command);
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
        println!("{:?}", args);
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