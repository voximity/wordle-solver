mod game;
mod nyt;

use crate::game::guess;
use crate::nyt::daily_manifest;
use game::{WORD_LENGTH, Wordle};
use rand::prelude::IndexedRandom;
use serde::Serialize;
use std::fmt::Write;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};
use std::path::Path;

fn read_lines(file: impl AsRef<Path>) -> io::Result<Vec<String>> {
    Ok(BufReader::new(File::open(file)?)
        .lines()
        .map_while(Result::ok)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>())
}

fn run(word_file: &'static str) {
    let robot_lines = read_lines("robot").unwrap();
    let words = std::fs::read(word_file).unwrap();
    let webhook_urls = read_lines("webhook").unwrap();

    let wordle: Wordle<WORD_LENGTH> = Wordle::new(&words);
    let manifest = daily_manifest().expect("could not get daily manifest");
    let Some(guesses) = guess(&wordle, manifest.solution.as_bytes()) else {
        if word_file == "words_all" {
            panic!("could not solve even with all words");
        }

        return run("words_all");
    };

    let client = reqwest::blocking::Client::new();
    let mut sent = 0;
    for webhook_url in webhook_urls.iter().map(|url| url.trim()) {
        let robot_line = robot_lines.choose(&mut rand::rng()).unwrap();

        let mut content = format!(
            "{robot_line}\n\n**Wordle {}**\n{}",
            manifest.days_since_launch,
            if word_file == "words_all" {
                "*this guess required a larger word pool!*\n"
            } else {
                ""
            }
        );

        guesses.iter().for_each(|(guess, feedback, pool)| {
            write!(
                &mut content,
                "{feedback} ||`{}` (1 in {pool})||\n",
                std::str::from_utf8(guess).unwrap()
            )
            .unwrap()
        });

        #[derive(Serialize)]
        struct Payload {
            content: String,
        }

        match client.post(webhook_url).json(&Payload { content }).send() {
            Ok(_) => sent += 1,
            Err(e) => {
                eprintln!("failed to send to {webhook_url}: {e}");
            }
        }
    }

    println!("attempt sent to {sent} webhook URLs");
}

fn main() {
    run("words");
}
