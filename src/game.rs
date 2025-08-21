use rand::Rng;
use roaring::RoaringBitmap;
use std::array;
use std::fmt::{Display, Formatter, Write};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Letter {
    Incorrect,
    Partial,
    Correct,
}

impl Display for Letter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Incorrect => f.write_char('⬛'),
            Self::Partial => f.write_char('🟨'),
            Self::Correct => f.write_char('🟩'),
        }
    }
}

pub struct Feedback<const N: usize>(pub [Letter; N]);

impl<const N: usize> Display for Feedback<N> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.iter().try_for_each(|l| l.fmt(f))
    }
}

impl<const N: usize> Feedback<N> {
    fn correct() -> Self {
        Self([Letter::Correct; N])
    }
}

pub struct Wordle<'a, const N: usize> {
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

    pub fn guess(guess: &[u8], goal: &[u8]) -> Feedback<N> {
        Feedback(array::from_fn(|slot| {
            if guess[slot] == goal[slot] {
                return Letter::Correct;
            }

            let prev_seen = guess[0..slot].iter().filter(|&&c| c == guess[slot]).count();
            let num_in_goal = goal.iter().filter(|&&c| c == guess[slot]).count();
            if goal.contains(&guess[slot]) && prev_seen <= num_in_goal {
                return Letter::Partial;
            }

            Letter::Incorrect
        }))
    }

    pub fn apply_guess(&self, bitmap: &mut RoaringBitmap, guess: &[u8], feedback: &Feedback<N>) {
        for (slot, (state, ch)) in feedback.0.iter().zip(guess.iter()).enumerate() {
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
                        .0
                        .iter()
                        .zip(guess.iter())
                        .filter(|(state, l)| {
                            *l == ch && matches!(state, Letter::Correct | Letter::Partial)
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

pub fn guess<'a, const N: usize>(
    wordle: &'a Wordle<N>,
    goal: &[u8],
) -> Option<Vec<(&'a [u8], Feedback<N>, u64)>> {
    let mut c = wordle.new_bitmap();
    let mut guesses = vec![];
    loop {
        let len = c.len();
        match len {
            1 => {
                let my_guess = wordle.words[c.select(0).unwrap() as usize];
                if my_guess == goal {
                    // add final guess if we didn't "guess" it before
                    // TODO: why does this sometimes happen?
                    if !guesses.last().is_some_and(|(g, _, _)| *g == goal) {
                        guesses.push((my_guess, Feedback::correct(), len));
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
        let feedback = Wordle::guess(word, goal);
        wordle.apply_guess(&mut c, word, &feedback);
        guesses.push((word, feedback, len));
    }
}

pub const WORD_LENGTH: usize = 5;
