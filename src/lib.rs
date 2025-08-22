use std::{collections::HashMap, hash::Hash};

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

//the word id where a gloss is arrowed
pub struct GlossArrow {
    gloss_id: u32,
    word_id: u32,
}

#[derive(Clone, Debug)]
pub struct Gloss {
    gloss_id: u32,
    lemma: String,
    sort_alpha: String,
    gloss: String,
    unit: u32,
    pos: String,
    status: u32,
}

#[derive(Clone, Debug)]
pub struct GlossOccurrance {
    //<'a> {
    //gloss_ref: &'a Gloss,
    gloss_id: u32,
    lemma: String,
    sort_alpha: String,
    gloss: String,
    arrowed_seq: Option<usize>,
    arrowed_state: ArrowedState,
}

pub struct Sequence {
    sequence_id: u32,
    name: String,
    gloss_name: String,
    gloss: Vec<Gloss>,
    texts: Vec<String>,
    arrowed_words: Vec<GlossArrow>,
}

pub fn make_gloss_page(
    words: Vec<Word>,
    glosshash: HashMap<u32, GlossOccurrance>,
) -> Vec<GlossOccurrance> {
    let mut glosses: HashMap<u32, GlossOccurrance> = HashMap::new();

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

    let mut sorted_glosses: Vec<GlossOccurrance> = glosses.values().cloned().collect();
    sorted_glosses.sort_by(|a, b| {
        a.sort_alpha
            .to_lowercase()
            .cmp(&b.sort_alpha.to_lowercase())
    });

    sorted_glosses
}

pub fn get_gloss_string(glosses: &[GlossOccurrance]) -> String {
    let mut res = String::from("");
    for g in glosses {
        match g.arrowed_state {
            ArrowedState::Arrowed => res.push_str(&format!("-> {}  {}\n", g.lemma, g.gloss)),
            ArrowedState::Visible => res.push_str(&format!("{}  {}\n", g.lemma, g.gloss)),
            ArrowedState::Invisible => (),
        }
    }
    res
}

pub fn make_gloss_occurrances(
    words: &[Word],
    seq: Sequence,
    glosses_hash: HashMap<u32, Gloss>,
) -> Vec<GlossOccurrance> {
    //hashmap of word_ids which are arrowed
    let mut aw = HashMap::new();
    for s in seq.arrowed_words {
        aw.insert(s.word_id, s.gloss_id);
    }

    //get sequence where the gloss is arrowed
    let mut glosses_seq = HashMap::new();
    for (seq, w) in words.iter().enumerate() {
        if let Some(arrowed_word_gloss) = aw.get(&w.word_id)
            && let Some(gloss) = w.gloss_id
            && *arrowed_word_gloss == gloss
        {
            glosses_seq.insert(gloss, seq);
        }
    }

    let mut r = vec![];
    for w in words {
        if let Some(gloss_id) = w.gloss_id
            && let Some(gloss) = glosses_hash.get(&gloss_id)
            && let Some(gloss_seq) = glosses_seq.get(&gloss_id)
        {
            r.push(GlossOccurrance {
                gloss_id: gloss_id,
                lemma: gloss.lemma.clone(),
                sort_alpha: gloss.sort_alpha.clone(),
                gloss: gloss.gloss.clone(),
                arrowed_seq: Some(*gloss_seq),
                arrowed_state: ArrowedState::Visible,
            });
        }
    }

    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let glosses = vec![
            Gloss {
                gloss_id: 1,
                lemma: String::from("ἄγω"),
                sort_alpha: String::from("αγω"),
                gloss: String::from("blah gloss"),
                pos: String::from("verb"),
                unit: 8,
                status: 1,
            },
            Gloss {
                gloss_id: 2,
                lemma: String::from("βλάπτω"),
                sort_alpha: String::from("βλαπτω"),
                gloss: String::from("blah gloss"),
                pos: String::from("verb"),
                unit: 8,
                status: 1,
            },
        ];

        let sequence = Sequence {
            sequence_id: 1,
            name: String::from("SGI"),
            gloss_name: String::from("H&Qplus"),
            gloss: glosses.clone(),
            arrowed_words: vec![GlossArrow {
                word_id: 2,
                gloss_id: 1,
            }],
            texts: vec![],
        };
        // let glosses_occurrances = vec![
        //     GlossOccurrance {
        //         //gloss_ref: &glosses[0],
        //         gloss_id: 1,
        //         lemma: String::from("ἄγω"),
        //         sort_alpha: String::from("αγω"),
        //         gloss: String::from("blah gloss"),
        //         arrowed_seq: None,
        //         arrowed_state: ArrowedState::Visible,
        //     },
        //     GlossOccurrance {
        //         //gloss_ref: &glosses[0],
        //         gloss_id: 2,
        //         lemma: String::from("βλάπτω"),
        //         sort_alpha: String::from("βλαπτω"),
        //         gloss: String::from("blah gloss2"),
        //         arrowed_seq: Some(1),
        //         arrowed_state: ArrowedState::Visible,
        //     },
        // ];

        let words = vec![
            Word {
                word_id: 0,
                word: String::from("βλάπτει"),
                gloss_id: Some(2),
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

        let mut glosses_hash = HashMap::new();
        for g in glosses {
            glosses_hash.insert(g.gloss_id, g.clone());
        }
        let glosses_occurrances = make_gloss_occurrances(&words, sequence, glosses_hash);

        let mut gloss_hash = HashMap::new();
        for g in glosses_occurrances {
            gloss_hash.insert(g.gloss_id, g.clone());
        }

        let s = make_gloss_page(words, gloss_hash);
        let p = get_gloss_string(&s);
        println!("test: \n{p}");
    }
}
