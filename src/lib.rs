use std::collections::HashMap;

pub fn build_gloss_map() {}

pub struct Word {
    word_id: u32,
    word: String,
    gloss_id: Option<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ArrowedState {
    Visible,
    Arrowed,
    Invisible,
}

#[derive(Clone, Debug)]
pub struct Gloss {
    gloss_id: u32,
    lemma: String,
    sort_alpha: String,
    gloss: String,
    arrowed_seq: Option<usize>,
    arrowed_state: ArrowedState,
}

pub fn make_gloss_page(words: Vec<Word>, glosshash: HashMap<u32, Gloss>) -> Vec<Gloss> {
    let mut glosses: HashMap<u32, Gloss> = HashMap::new();

    //let mut glosses: Vec<Gloss> = vec![];
    for (seq, w) in words.iter().enumerate() {
        if let Some(gloss_id) = w.gloss_id
            && let Some(gloss) = glosshash.get(&gloss_id)
        {
            let mut g = gloss.clone();
            if gloss.arrowed_seq.is_none()
                || (gloss.arrowed_seq.is_some() && seq < gloss.arrowed_seq.unwrap())
            {
                g.arrowed_state = ArrowedState::Visible;
            } else if gloss.arrowed_seq.is_some() && seq == gloss.arrowed_seq.unwrap() {
                g.arrowed_state = ArrowedState::Arrowed;
            } else {
                g.arrowed_state = ArrowedState::Invisible;
            }

            //if arrowed insert it, or if it's not already inserted
            //we want to avoid replacing an arrowed version with a non-arrowed version
            if g.arrowed_state == ArrowedState::Arrowed || !glosses.contains_key(&gloss_id) {
                glosses.insert(gloss_id, g);
            }
        }
    }

    let mut sorted_glosses: Vec<Gloss> = glosses.values().cloned().collect();
    sorted_glosses.sort_by(|a, b| {
        a.sort_alpha
            .to_lowercase()
            .cmp(&b.sort_alpha.to_lowercase())
    });

    sorted_glosses
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let glosses = [
            Gloss {
                gloss_id: 1,
                lemma: String::from("ἄγω"),
                sort_alpha: String::from("αγω"),
                gloss: String::from("blah gloss"),
                arrowed_seq: None,
                arrowed_state: ArrowedState::Visible,
            },
            Gloss {
                gloss_id: 2,
                lemma: String::from("βλάπτω"),
                sort_alpha: String::from("βλαπτω"),
                gloss: String::from("blah gloss2"),
                arrowed_seq: Some(1),
                arrowed_state: ArrowedState::Visible,
            },
        ];

        let words = vec![
            Word {
                word_id: 0,
                word: String::from("βλάπτει"),
                gloss_id: None,
            },
            Word {
                word_id: 1,
                word: String::from("βλάπτει"),
                gloss_id: Some(2),
            },
            Word {
                word_id: 2,
                word: String::from("ἄγει"),
                gloss_id: Some(1),
            },
            Word {
                word_id: 3,
                word: String::from("ἄγεις"),
                gloss_id: Some(1),
            },
        ];

        let mut gloss_hash = HashMap::new();
        for g in glosses {
            gloss_hash.insert(g.gloss_id, g.clone());
        }

        let s = make_gloss_page(words, gloss_hash);
        println!("test: {s:?}");
    }
}
