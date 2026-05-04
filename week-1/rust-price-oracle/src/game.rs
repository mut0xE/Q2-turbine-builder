use colored::*;
use rand::RngExt;
use std::io;

use crate::hints;

pub fn start() {
    println!("{}", "Game starting...\n".dimmed());
    let secret_price = rand::rng().random_range(50..=300);
    println!();

    let mut tries = 5;
    loop {
        if tries <= 0 {
            println!("\n{}", "MARKET CLOSED".red().bold());

            println!("The price was ${}", format!("${}", secret_price).yellow());

            break;
        }
        println!("{}", format!("You have {} tries.\n", tries).bold());
        println!("Enter your SOL price guess:");

        let mut guess = String::new();

        io::stdin()
            .read_line(&mut guess)
            .expect("failed to read input");
        let guess = match guess.trim().parse::<u32>() {
            Ok(num) => num,
            Err(_) => {
                println!("Please enter a valid number.");
                println!();
                continue;
            }
        };
        println!("You predicted: {}", format!("${}", guess).blue());

        if guess == secret_price {
            println!("{}", "Exact price predicted.".green());
            break;
        }
        hints::price_hint(guess, secret_price);
        tries -= 1
    }
}
