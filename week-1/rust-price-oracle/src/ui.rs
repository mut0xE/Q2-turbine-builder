use colored::*;
pub fn welcome() {
    println!("{}", "=== SOL PRICE ORACLE ===".bold().cyan());
    println!(
        "Guess the price: {} – {}\n",
        "$50".yellow(),
        "$300".yellow()
    );
}
