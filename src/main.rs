use rand::Rng;
use roaring::RoaringBitmap;
use std::array;
use std::io::Write;

#[derive(PartialEq, Eq)]
enum Letter {
    Incorrect,
    Partial,
    Correct,
}

struct Wordle<'a, const N: usize> {
    words: Vec<&'a [u8]>,
    pos: [[RoaringBitmap; 26]; N],
    at_least: [[RoaringBitmap; 26]; N],
}

impl<'a, const N: usize> Wordle<'a, N> {
    pub fn new(words_raw: &'a [u8]) -> Self {
        let words = words_raw
            .chunks_exact(N + 1)
            .map(|ch| &ch[0..N])
            .collect::<Vec<_>>();

        // TODO: optimize... maybe
        // precompute pos bitset
        let pos: [[RoaringBitmap; 26]; N] = array::from_fn(|slot| {
            array::from_fn(|letter| {
                let mut bitmap = RoaringBitmap::new();
                for (i, word) in words.iter().enumerate() {
                    if word[slot] - b'a' == letter as u8 {
                        bitmap.insert(i as u32);
                    }
                }
                bitmap
            })
        });

        // precompute atleast bitset
        let at_least: [[RoaringBitmap; 26]; N] = array::from_fn(|freq| {
            array::from_fn(|letter| {
                let mut bitmap = RoaringBitmap::new();
                for (i, word) in words.iter().enumerate() {
                    let occurrences = word.iter().filter(|&&l| l - b'a' == letter as u8).count();
                    if occurrences >= freq + 1 {
                        bitmap.insert(i as u32);
                    }
                }
                bitmap
            })
        });

        Self {
            words,
            pos,
            at_least,
        }
    }

    pub fn new_bitmap(&self) -> RoaringBitmap {
        let mut bitmap = RoaringBitmap::new();
        bitmap.insert_range(0..(self.words.len() as u32));
        bitmap
    }

    pub fn guess(guess: &[u8], goal: &[u8]) -> [(Letter, u8); N] {
        array::from_fn(|slot| {
            if guess[slot] == goal[slot] {
                return (Letter::Correct, guess[slot]);
            }

            let prev_seen = guess[0..slot].iter().filter(|&&c| c == guess[slot]).count();
            let num_in_goal = goal.iter().filter(|&&c| c == guess[slot]).count();
            if goal.contains(&guess[slot]) && prev_seen <= num_in_goal {
                return (Letter::Partial, guess[slot]);
            }

            (Letter::Incorrect, guess[slot])
        })
    }

    pub fn apply_guess(&self, bitmap: &mut RoaringBitmap, feedback: &[(Letter, u8); N]) {
        for (slot, (state, ch)) in feedback.iter().enumerate() {
            let char_index = (ch - b'a') as usize;
            match state {
                Letter::Correct => {
                    *bitmap &= &self.pos[slot][char_index];
                }
                Letter::Partial => {
                    *bitmap &= &self.at_least[0][char_index];
                    *bitmap -= &self.pos[slot][char_index];
                }
                Letter::Incorrect => {
                    let m = feedback
                        .iter()
                        .filter(|(state, l)| {
                            l == ch && matches!(state, Letter::Correct | Letter::Partial)
                        })
                        .count();

                    if m > 0 {
                        *bitmap -=
                            &self.at_least[m /* all words with m+1 occurrences */][char_index];
                        *bitmap -= &self.pos[slot][char_index];
                    } else {
                        *bitmap -= &self.at_least[0][char_index];
                    }
                }
            }
        }
    }
}

fn guess<'a, const N: usize>(wordle: &'a Wordle<N>, goal: &[u8]) -> Option<Vec<&'a [u8]>> {
    let mut c = wordle.new_bitmap();
    let mut guesses = vec![];
    loop {
        let len = c.len();
        match len {
            1 => {
                let my_guess = wordle.words[c.select(0).unwrap() as usize];
                if my_guess == goal {
                    if !guesses.last().is_some_and(|&g| g == goal) {
                        guesses.push(my_guess);
                    }
                    return Some(guesses);
                } else {
                    return None;
                }
            }
            0 => return None,
            _ => (),
        }

        let idx = rand::rng().random_range(0..(len as u32));
        let word = wordle.words[c.select(idx).unwrap() as usize];
        let guess = Wordle::guess(word, goal);
        wordle.apply_guess(&mut c, &guess);
        guesses.push(word);
    }
}

const WORD_LENGTH: usize = 5;
// const MAX_GUESSES: usize = 6;

fn main() {
    let words_raw = std::fs::read("words").unwrap();
    let wordle: Wordle<WORD_LENGTH> = Wordle::new(&words_raw);
    // let iterations = 1000;

    let mut l = String::new();
    loop {
        print!("goal word? ");

        l.clear();
        std::io::stdout().flush().unwrap();
        std::io::stdin().read_line(&mut l).unwrap();

        let guesses = guess(&wordle, &l.as_bytes()[0..WORD_LENGTH]);
        match guesses {
            Some(guesses) => {
                println!("took {} guesses:", guesses.len());
                guesses
                    .iter()
                    .for_each(|guess| println!("- {}", str::from_utf8(&guess).unwrap()));
            }
            None => println!("unable to arrive at solution"),
        }

        // let avg = (0..iterations)
        //     .into_par_iter()
        //     .map(|_| {
        //         guess(&wordle, &l.as_bytes()[0..WORD_LENGTH])
        //             .map(|v| v.len().min(MAX_GUESSES))
        //             .unwrap_or(MAX_GUESSES)
        //     })
        //     .sum::<usize>() as f32
        //     / iterations as f32;
        //
        // println!("over {iterations} iterations, average {avg} guesses");
    }
}
