pub fn price_hint(guess: u32, secret: u32) {
    let diff = guess.abs_diff(secret);
    let dir = if guess < secret {
        "Too bearish"
    } else {
        "Too bullish"
    };

    let signal = match diff {
        0..=5 => "very close",
        6..=15 => "strong signal",
        16..=30 => "weak signal",
        _ => "no alpha",
    };
    println!("{dir} — {signal}\n");

    println!();
}
