// The main logic (minimax with alpha-beta pruning as well as the score function) was mostly implemented by ChatGPT.
// I don't regard that as cheating because I'm not super interested in algorithms. However, what I manually want to do
// is to improve the performance of the game by adding multicore support. However, this first requires multicore support
// in the kernel. I will leave the game for now as it is.

mod game_board;

use std::io::{Write, stdout};

use game_board::{GameBoard, Player};
use userspace::util::read_line;

extern crate alloc;
extern crate userspace;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Welcome to connect four!");
    print!("Choose the search depth: ");
    stdout().flush()?;

    let depth: u8 = loop {
        let line = read_line();
        if let Ok(depth) = line.parse() {
            break depth;
        }
        println!("\nYou didn't entered a number! Try again.");
    };

    print!("Who should start? (c)omputer or (h)uman? ");
    stdout().flush()?;
    let mut current_player = loop {
        let line = read_line();
        if line == "c" {
            break Player::C;
        }
        if line == "h" {
            break Player::H;
        }
        println!("Wrong input. Try again.");
    };

    println!("\nAlright, let's play!");

    let mut board = GameBoard::new();
    board.print();

    loop {
        if let Some(winner) = board.is_game_over() {
            println!("Hoooorayyy! Player {winner:?} won!");
            break;
        }
        match current_player {
            Player::C => computer(&mut board, depth),
            Player::H => human(&mut board),
        }
        current_player.switch();
    }

    Ok(())
}

fn human(board: &mut GameBoard) {
    println!("Enter a number between 1 and 7");
    loop {
        let line = read_line();
        if let Ok(column) = line.parse::<u8>()
            && (1..=7).contains(&column)
            && board.put(Player::H, column - 1).is_ok()
        {
            break;
        }
        println!("You entered an invalid move");
    }
    board.print();
}

fn computer(board: &mut GameBoard, depth: u8) {
    println!("Calculating moves... ");
    let mut counter = 0;
    let best_move = board
        .find_best_move(depth, Player::C, &mut counter)
        .expect("Computer should always find a move - otherwise it is a draw.");
    board.put(Player::C, best_move).unwrap();
    board.print();
    println!(
        "Computer put into column {} (calculated {counter} positions)",
        best_move + 1
    );
}
