mod exportlatex;

#[allow(unused_imports)]
use exportlatex::ExportLatex;
use serde::{Deserialize, Serialize};
use serde_xml_rs::from_str;
use serde_xml_rs::ser::Serializer;
use std::collections::{HashMap, HashSet};
use std::fs;
use uuid::Uuid;
use xml::writer::EmitterConfig;

// text ids
// ion 111-119
// medea 120-128
// lysias 133-137
// xenophon 129-132
// phaedrus 228-269
// thuc2 270-295
// ajax 296-314

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum WordType {
    Word = 0,
    Punctuation = 1,
    Speaker = 2,
    Section = 4,
    VerseLine = 5, //for verse #
    ParaWithIndent = 6,
    WorkTitle = 7,
    SectionTitle = 8,
    InlineSpeaker = 9,
    ParaNoIndent = 10,
    PageBreak = 11, //not used: we now use separate table called latex_page_breaks
    Desc = 12,
    InvalidType = 13,
    InlineVerseSpeaker = 14,
    //0 word
    //1 punct
    //2 speaker
    //4 section
    //5 new line for verse #
    //6 new para with indent
    //7 work title
    //8 section title centered
    //9 inline speaker, so 2, but inline
    //10 new para without indent
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Gloss {
    #[serde(rename = "@gloss_id")]
    gloss_id: i32,
    #[serde(rename = "@uuid")]
    uuid: Uuid,
    lemma: String,
    sort_alpha: String,
    #[serde(rename = "gloss")]
    def: String,
    pos: String,
    unit: i32,
    note: String,
    updated: String,
    status: i32,
    updated_user: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Word {
    #[serde(rename = "@id")]
    word_id: i32,
    #[serde(rename = "@uuid")]
    uuid: Uuid,
    #[serde(rename = "@gloss_id")]
    gloss_id: Option<i32>,
    #[serde(rename = "@gloss_uuid")]
    gloss_uuid: Option<Uuid>,
    #[serde(rename = "@type")]
    word_type: WordType,
    #[serde(rename = "#text", default)]
    word: String,
}

//the word id where a gloss is arrowed
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GlossArrow {
    #[serde(rename = "@gloss_id")]
    gloss_id: i32,
    #[serde(rename = "@word_id")]
    word_id: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Sequence {
    sequence_id: i32,
    name: String,
    start_page: usize,
    gloss_names: Vec<String>,
    texts: Texts,
    arrowed_words: ArrowedWords,
}

impl Sequence {
    pub fn to_xml(&self) -> String {
        let mut buffer: Vec<u8> = Vec::new();
        let writer = EmitterConfig::new()
            .perform_indent(true) // Optional: for pretty-printing
            .create_writer(&mut buffer);

        let mut serializer = Serializer::new(writer);
        self.serialize(&mut serializer).unwrap();
        String::from_utf8(buffer).expect("UTF-8 error")
    }

    pub fn from_xml(s: &str) -> Result<Sequence, serde_xml_rs::Error> {
        from_str(s)
    }
}

pub struct ArrowedWordsIndex {
    gloss_lemma: String,
    gloss_sort: String,
    page_number: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ArrowedState {
    Visible,
    Arrowed,
    Invisible,
}

fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TextDescription {
    #[serde(rename = "@display", default = "default_true")]
    display: bool,
    #[serde(rename = "#text", default)]
    text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Texts {
    text: Vec<TextDescription>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ArrowedWords {
    arrow: Vec<GlossArrow>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Words {
    word: Vec<Word>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Text {
    #[serde(rename = "@text_id")]
    text_id: i32,
    #[serde(rename = "@text_name")]
    text_name: String,
    #[serde(rename = "@display", default)]
    display: bool,
    #[serde(default)]
    pages: Vec<usize>,
    words: Words,
}

impl Text {
    pub fn to_xml(&self) -> String {
        let mut buffer: Vec<u8> = Vec::new();
        let writer = EmitterConfig::new()
            .perform_indent(true) // Optional: for pretty-printing
            .create_writer(&mut buffer);

        let mut serializer = Serializer::new(writer);
        self.serialize(&mut serializer).unwrap();
        String::from_utf8(buffer).expect("UTF-8 error")
    }

    pub fn from_xml(s: &str) -> Result<Text, serde_xml_rs::Error> {
        from_str(s)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Glosses {
    #[serde(rename = "@gloss_id")]
    gloss_id: i32,
    #[serde(rename = "@gloss_name")]
    gloss_name: String,
    gloss: Vec<Gloss>,
}

impl Glosses {
    pub fn to_xml(&self) -> String {
        let mut buffer: Vec<u8> = Vec::new();
        let writer = EmitterConfig::new()
            .perform_indent(true) // Optional: for pretty-printing
            .create_writer(&mut buffer);

        let mut serializer = Serializer::new(writer);
        self.serialize(&mut serializer).unwrap();
        String::from_utf8(buffer).expect("UTF-8 error")
    }

    pub fn from_xml(s: &str) -> Result<Glosses, serde_xml_rs::Error> {
        from_str(s)
    }
}

#[derive(Clone, Debug)]
pub struct GlossOccurrance {
    //<'a> {
    //gloss_ref: &'a Gloss,
    gloss_id: i32,
    lemma: String,
    sort_alpha: String,
    gloss: String,
    arrowed_seq: Option<usize>,
    arrowed_state: ArrowedState,
}

pub trait ExportDocument {
    fn gloss_entry(&self, lemma: &str, gloss: &str, arrowed: bool) -> String;
    fn make_text(&self, words: &[Word]) -> String;
    fn page_start(&self, title: &str) -> String;
    fn page_end(&self) -> String;
    fn page_gloss_start(&self) -> String;
    fn document_end(&self) -> String;
    fn document_start(&self, title: &str, start_page: usize) -> String;
    fn make_index(&self, arrowed_words_index: &[ArrowedWordsIndex]) -> String;
    fn blank_page(&self) -> String;
}

pub fn make_page(
    words: &[Word],
    gloss_hash: &HashMap<i32, GlossOccurrance>,
    seq_offset: usize,
    export: &impl ExportDocument,
    title: &str,
    arrowed_words_index: &mut Vec<ArrowedWordsIndex>,
    page_number: usize,
) -> String {
    let mut page = export.page_start(title);
    page.push_str(&export.make_text(words));

    page.push_str(&export.page_gloss_start());

    let s = make_gloss_page(
        words,
        gloss_hash,
        seq_offset,
        arrowed_words_index,
        page_number,
    );
    page.push_str(&get_gloss_string(&s, export));

    page.push_str(&export.page_end());
    page
}

pub fn make_document(
    title: &str,
    texts: &[Text],
    gloss_hash: &HashMap<i32, GlossOccurrance>,
    export: &impl ExportDocument,
    start_page: usize,
) -> String {
    let mut arrowed_words_index: Vec<ArrowedWordsIndex> = vec![];
    let mut page_number = start_page;

    let mut doc = export.document_start(title, page_number);
    //if page_number is even, insert blank page
    if page_number % 2 == 0 {
        doc.push_str(export.blank_page().as_str());
        page_number += 1;
    }
    let mut index;
    let mut overall_index = 0;
    for t in texts {
        if !t.display {
            overall_index += t.words.word.len();
            continue;
        }
        //println!("overall index: {}", overall_index);
        //let words_per_text = t.words.word.len();
        index = 0;
        for (i, w) in t.pages.iter().enumerate() {
            if i == t.pages.len() - 1 {
                doc.push_str(
                    make_page(
                        &t.words.word[index..],
                        gloss_hash,
                        overall_index,
                        export,
                        &t.text_name,
                        &mut arrowed_words_index,
                        page_number,
                    )
                    .as_str(),
                );
                let count = t.words.word.len() - index;
                index += count;
                overall_index += count;
            } else {
                doc.push_str(
                    make_page(
                        &t.words.word[index..index + w],
                        gloss_hash,
                        overall_index,
                        export,
                        &t.text_name,
                        &mut arrowed_words_index,
                        page_number,
                    )
                    .as_str(),
                );
                index += w;
                overall_index += w;
            }
            page_number += 1;
        }
        if page_number % 2 != 0 {
            page_number += 1;
            doc.push_str(export.blank_page().as_str());
        }
        doc.push_str(export.blank_page().as_str());
        page_number += 1;
    }
    //make index
    if !arrowed_words_index.is_empty() {
        arrowed_words_index.sort_by(|a, b| {
            a.gloss_sort
                .to_lowercase()
                .cmp(&b.gloss_sort.to_lowercase())
        });

        doc.push_str(export.make_index(&arrowed_words_index).as_str());
    }

    doc.push_str(&export.document_end());
    doc
}

//sets arrowed state and makes glosses unique on page
pub fn make_gloss_page(
    words: &[Word],
    glosshash: &HashMap<i32, GlossOccurrance>,
    seq_offset: usize,
    arrowed_words_index: &mut Vec<ArrowedWordsIndex>,
    page_number: usize,
) -> Vec<GlossOccurrance> {
    let mut glosses: HashMap<i32, GlossOccurrance> = HashMap::new();

    for (seq, w) in words.iter().enumerate() {
        if let Some(gloss_id) = w.gloss_id
            && let Some(gloss) = glosshash.get(&gloss_id)
        {
            let mut g = gloss.clone();
            if gloss.arrowed_seq.is_none()
                || (gloss.arrowed_seq.is_some() && seq + seq_offset < gloss.arrowed_seq.unwrap())
            {
                g.arrowed_state = ArrowedState::Visible;
            } else if gloss.arrowed_seq.is_some() && seq + seq_offset == gloss.arrowed_seq.unwrap()
            {
                g.arrowed_state = ArrowedState::Arrowed;
                //if build_index {
                arrowed_words_index.push(ArrowedWordsIndex {
                    gloss_lemma: g.lemma.clone(),
                    gloss_sort: g.sort_alpha.to_owned(),
                    page_number,
                });
                //}
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

pub fn get_gloss_string(glosses: &[GlossOccurrance], export: &impl ExportDocument) -> String {
    let mut res = String::from("");
    for g in glosses {
        match g.arrowed_state {
            ArrowedState::Arrowed => {
                res.push_str(export.gloss_entry(&g.lemma, &g.gloss, true).as_str())
            }
            ArrowedState::Visible => {
                res.push_str(export.gloss_entry(&g.lemma, &g.gloss, false).as_str())
            }
            ArrowedState::Invisible => (),
        }
    }
    res
}

//sets figures out seq where each gloss is arrowed, arrowed_state is set to a dummy value;
//really arrowed_seq is set in make_gloss_page
pub fn make_gloss_occurrances(
    words: &[Word],
    arrowed_words: &HashMap<i32, i32>,
    glosses_hash: &HashMap<i32, Gloss>,
    seq_offset: &mut usize,
) -> Vec<GlossOccurrance> {
    //get sequence where the gloss is arrowed
    let mut glosses_seq = HashMap::new();
    for (seq, w) in words.iter().enumerate() {
        if let Some(arrowed_word_gloss) = arrowed_words.get(&w.word_id)
            && let Some(gloss) = w.gloss_id
            && *arrowed_word_gloss == gloss
        {
            glosses_seq.insert(gloss, seq + *seq_offset);
        }
    }
    *seq_offset += words.len();

    let mut r = vec![];
    for w in words {
        if let Some(gloss_id) = w.gloss_id
            && let Some(gloss) = glosses_hash.get(&gloss_id)
        {
            if let Some(gloss_seq) = glosses_seq.get(&gloss_id) {
                r.push(GlossOccurrance {
                    gloss_id,
                    lemma: gloss.lemma.clone(),
                    sort_alpha: gloss.sort_alpha.clone(),
                    gloss: gloss.def.clone(),
                    arrowed_seq: Some(*gloss_seq),
                    arrowed_state: ArrowedState::Visible, //this is actually set later
                });
            } else {
                r.push(GlossOccurrance {
                    gloss_id,
                    lemma: gloss.lemma.clone(),
                    sort_alpha: gloss.sort_alpha.clone(),
                    gloss: gloss.def.clone(),
                    arrowed_seq: None,
                    arrowed_state: ArrowedState::Visible, //this is actually set later
                });
            }
        }
    }

    r
}

pub fn load_sequence(file_path: &str, output_path: &str) -> bool {
    if let Ok(contents) = fs::read_to_string(file_path)
        && let Ok(sequence) = Sequence::from_xml(&contents)
    {
        let mut texts = vec![];
        let mut glosses = vec![];

        for g in &sequence.gloss_names {
            if let Ok(contents) = fs::read_to_string(g)
                && let Ok(gloss) = Glosses::from_xml(&contents)
            {
                glosses.push(gloss);
            }
        }

        for t in &sequence.texts.text {
            if let Ok(contents) = fs::read_to_string(&t.text)
                && let Ok(mut text) = Text::from_xml(&contents)
            {
                text.display = t.display;
                texts.push(text);
            }
        }

        if !texts.is_empty() && !glosses.is_empty() {
            let mut glosses_hash = HashMap::new();
            for ggg in glosses {
                //let mut i = 1000000;
                for g in ggg.gloss.clone() {
                    // if g.unit > 0 && g.unit < 21 {
                    //     let lemma = if let Some((before_comma, _)) = g.lemma.split_once(',') {
                    //         before_comma.to_string()
                    //     } else {
                    //         g.lemma.clone()
                    //     };

                    //     println!(
                    //         "<word id=\"{}\" uuid=\"{}\" gloss_id=\"{}\" gloss_uuid=\"{}\" type=\"Word\">{}</word>",
                    //         i,
                    //         Uuid::new_v4(),
                    //         g.gloss_id,
                    //         g.uuid,
                    //         lemma
                    //     );

                    //     println!(
                    //         "<arrowed_word gloss_id=\"{}\" word_id=\"{}\" /> <!-- {} {} -->",
                    //         g.gloss_id, i, g.unit, lemma
                    //     );
                    //     i += 1;
                    // }
                    glosses_hash.insert(g.gloss_id, g.clone());
                }
            }

            let mut aw = HashMap::new();
            for s in sequence.arrowed_words.arrow.clone() {
                aw.insert(s.word_id, s.gloss_id);
            }

            let verify_res =
                verify_arrowed_words(&texts, &aw, &glosses_hash, &sequence.arrowed_words.arrow);
            assert!(!verify_res);

            let mut glosses_occurrances: Vec<GlossOccurrance> = vec![];
            let mut offset = 0;
            for t in &texts {
                glosses_occurrances.append(&mut make_gloss_occurrances(
                    &t.words.word,
                    &aw,
                    &glosses_hash,
                    &mut offset,
                ));
            }

            let mut gloss_occurrances_hash = HashMap::new();
            for g in glosses_occurrances {
                //prevent versions without arrowed_seq from overwriting versions which do have arrowed_seq set
                // this should only contain glosses without an arrowed_seq if it is not arrowed anywhere in the sequence
                //
                // Probably we don't need gloss_occurrances at all and we could just at arrowed_seq and arrowed_state
                // to the Gloss struct, leaving those fields empty when deserializing from xml
                if g.arrowed_seq.is_some() || !gloss_occurrances_hash.contains_key(&g.gloss_id) {
                    gloss_occurrances_hash.insert(g.gloss_id, g.clone());
                }
            }

            texts[3].pages = vec![
                154, 151, 137, 72, 121, 63, 85, 107, 114, 142, 109, 79, 82, 81, 122, 99, 86, 110,
                112, 151, 140, 99, 71, 117, 114, 1,
            ];
            texts[4].pages = vec![
                142, 116, 117, 97, 81, 125, 92, 115, 84, 129, 76, 121, 142, 123, 81, 115, 109, 101,
                120, 88, 109, 1,
            ];
            //phaedrus words per page: ids 228-269
            texts[5].pages = vec![
                173, 95, 92, 125, 89, 140, 106, 74, 79, 84, 78, 107, 60, 90, 110, 148, 194, 146,
                139, 179, 126, 144, 189, 76, 149, 102, 150, 168, 102, 133, 129, 168, 143, 121, 146,
                144, 93, 97, 61, 126, 77, 129, 91, 91, 123, 114, 143, 115, 115, 159, 89, 164, 178,
                139, 93, 103, 113, 148, 178, 113, 83, 116, 100, 84, 120, 114, 86, 97, 81, 56, 90,
                105, 96, 85, 139, 86, 119, 101, 69, 75, 67, 102, 101, 91, 120, 125, 193, 60, 84,
                122, 81, 71, 96, 121, 114, 152, 90, 115, 91, 117, 123, 126, 129, 119, 202, 78, 145,
                172, 102, 125, 157, 125, 106, 144, 114, 134, 148, 129, 175, 186, 133, 102, 161, 53,
                150, 151, 193, 100, 110, 71, 126, 155, 121, 115, 119, 155, 84, 139, 187, 140, 196,
                159, 150, 177, 153, 229, 170, 161, 147, 133, 155, 111, 149, 127, 174, 152, 173,
                153, 127, 141, 92,
            ];

            //thuc2 words per page: ids 270-295
            texts[6].pages = vec![
                74, 56, 102, 125, 132, 114, 92, 145, 188, 197, 98, 86, 162, 120, 71, 112, 125, 176,
                135, 125, 115, 133, 79, 92, 117, 97, 156, 134, 115, 116, 111, 94, 130, 147, 125,
                147, 131, 93, 90, 121, 119, 121, 92, 87, 117, 108, 151, 99, 105, 126, 62, 83, 73,
                107, 101, 164, 187, 141, 110, 137, 114, 172, 150, 135, 93, 182, 126, 116, 133, 126,
                165, 144, 146, 105, 80, 95, 142, 124, 116, 133, 91, 157, 160, 148, 154, 198, 138,
                146, 146, 123, 120, 161, 141, 123, 107, 188, 135, 197, 163, 74,
            ];
            //ajax words per page; ids 296-314
            texts[7].pages = vec![
                59, 70, 55, 42, 52, 105, 70, 49, 71, 94, 109, 87, 98, 115, 74, 63, 67, 83, 55, 57,
                61, 49, 40, 49, 51, 47, 64, 49, 59, 121, 107, 91, 51, 49, 55, 67, 60, 104, 99, 62,
                77, 85, 96, 66, 65, 59, 96, 75, 85, 100, 95, 99, 105, 108, 160, 113, 107, 65, 58,
                73, 119, 47, 68, 48, 64, 74, 72, 80, 94, 104, 56, 57, 58, 59, 75, 69, 65, 82, 69,
                69, 78, 103, 93, 85, 65, 56, 73, 87, 83, 76, 52, 62, 80, 52, 76, 69, 67, 83, 91,
                107, 84, 95, 100, 97, 88, 107, 61, 54, 83, 98, 124, 105, 154, 146, 69, 96, 83, 100,
                63, 59, 64, 62, 82, 92, 109, 94, 75, 87, 69, 103, 103, 126, 157, 148, 127, 78, 76,
                70, 39,
            ];
            let p = make_document(
                &sequence.name,
                &texts,
                &gloss_occurrances_hash,
                &ExportLatex {},
                sequence.start_page,
            );
            let _ = fs::write(output_path, &p);
            //println!("testaaa: \n{p}");
        }
    } else {
        println!("no");
    }
    true
}

fn verify_arrowed_words(
    texts: &[Text],
    arrowed_words_hash: &HashMap<i32, i32>,
    glosses_hash: &HashMap<i32, Gloss>,
    arrowed_words: &[GlossArrow],
) -> bool {
    let mut has_errors = false;
    let mut seen_words = HashSet::<i32>::new();
    let mut seen_glosses = HashSet::<i32>::new();
    for s in arrowed_words {
        if !seen_words.insert(s.word_id) {
            println!("duplicate word_id in arrowed words {}", s.word_id);
            has_errors = true;
        }
        if !seen_glosses.insert(s.gloss_id) {
            println!("duplicate gloss_id in arrowed words {}", s.gloss_id);
            has_errors = true;
        }
    }

    for t in texts {
        for w in &t.words.word {
            if let Some(arrowed_gloss) = arrowed_words_hash.get(&w.word_id)
                && w.gloss_id.is_some()
                && *arrowed_gloss != w.gloss_id.unwrap()
            {
                let a = glosses_hash.get(&w.gloss_id.unwrap());
                let b = glosses_hash.get(arrowed_gloss);

                println!(
                    "arrow gloss doesn't match text's gloss {} {} g1: {} {} s1: {} g2: {} {} s2: {}",
                    w.word_id,
                    w.word,
                    a.unwrap().gloss_id,
                    a.unwrap().status,
                    a.unwrap().lemma,
                    b.unwrap().gloss_id,
                    b.unwrap().status,
                    b.unwrap().lemma,
                );
                has_errors = true;
            }
        }
    }
    has_errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let glosses = vec![
            Gloss {
                gloss_id: 1,
                uuid: Uuid::new_v4(),
                lemma: String::from("ἄγω"),
                sort_alpha: String::from("αγω"),
                def: String::from("blah gloss"),
                pos: String::from("verb"),
                unit: 8,
                note: String::from(""),
                updated: String::from(""),
                status: 1,
                updated_user: String::from(""),
            },
            Gloss {
                gloss_id: 3,
                uuid: Uuid::new_v4(),
                lemma: String::from("γαμέω"),
                sort_alpha: String::from("γαμεω"),
                def: String::from("blah gloss"),
                pos: String::from("verb"),
                unit: 8,
                note: String::from(""),
                updated: String::from(""),
                status: 1,
                updated_user: String::from(""),
            },
            Gloss {
                gloss_id: 2,
                uuid: Uuid::new_v4(),
                lemma: String::from("βλάπτω"),
                sort_alpha: String::from("βλαπτω"),
                def: String::from("blah gloss"),
                pos: String::from("verb"),
                unit: 8,
                note: String::from(""),
                updated: String::from(""),
                status: 1,
                updated_user: String::from(""),
            },
        ];

        let sequence = Sequence {
            sequence_id: 1,
            name: String::from("SGI"),
            start_page: 3,
            gloss_names: vec![String::from("H&Qplus")],
            arrowed_words: ArrowedWords {
                arrow: vec![
                    GlossArrow {
                        word_id: 5,
                        gloss_id: 1,
                    },
                    GlossArrow {
                        word_id: 1,
                        gloss_id: 2,
                    },
                    GlossArrow {
                        word_id: 10,
                        gloss_id: 3,
                    },
                ],
            },
            texts: Texts {
                text: vec![
                    TextDescription {
                        display: true,
                        text: String::from("abc.xml"),
                    },
                    TextDescription {
                        display: true,
                        text: String::from("def.xml"),
                    },
                ],
            },
        };

        let words = vec![
            Word {
                word_id: 0,
                uuid: Uuid::new_v4(),
                word: String::from("βλάπτει"),
                gloss_id: Some(2),
                gloss_uuid: None,
                word_type: WordType::Word,
            },
            Word {
                word_id: 10,
                uuid: Uuid::new_v4(),
                word: String::from("γαμεῖ"),
                gloss_id: Some(3),
                gloss_uuid: None,
                word_type: WordType::Word,
            },
            Word {
                word_id: 4,
                uuid: Uuid::new_v4(),
                word: String::from("ἄγει"),
                gloss_id: Some(1),
                gloss_uuid: None,
                word_type: WordType::Word,
            },
            Word {
                word_id: 1,
                uuid: Uuid::new_v4(),
                word: String::from("βλάπτει"),
                gloss_id: Some(2),
                gloss_uuid: None,
                word_type: WordType::Word,
            },
            Word {
                word_id: 6,
                uuid: Uuid::new_v4(),
                word: String::from("ἄγει"),
                gloss_id: Some(1),
                gloss_uuid: None,
                word_type: WordType::Word,
            },
            Word {
                word_id: 11,
                uuid: Uuid::new_v4(),
                word: String::from("γαμεῖ"),
                gloss_id: Some(3),
                gloss_uuid: None,
                word_type: WordType::Word,
            },
            Word {
                word_id: 2,
                uuid: Uuid::new_v4(),
                word: String::from("βλάπτει"),
                gloss_id: Some(2),
                gloss_uuid: None,
                word_type: WordType::Word,
            },
            Word {
                word_id: 20,
                uuid: Uuid::new_v4(),
                word: String::from("βλάπτει"),
                gloss_id: Some(2),
                gloss_uuid: None,
                word_type: WordType::Word,
            },
            Word {
                word_id: 5,
                uuid: Uuid::new_v4(),
                word: String::from("ἄγεις"),
                gloss_id: Some(1),
                gloss_uuid: None,
                word_type: WordType::Word,
            },
            Word {
                word_id: 7,
                uuid: Uuid::new_v4(),
                word: String::from("ἄγεις"),
                gloss_id: Some(1),
                gloss_uuid: None,
                word_type: WordType::Word,
            },
            Word {
                word_id: 8,
                uuid: Uuid::new_v4(),
                word: String::from("γαμεῖ"),
                gloss_id: Some(3),
                gloss_uuid: None,
                word_type: WordType::Word,
            },
            Word {
                word_id: 9,
                uuid: Uuid::new_v4(),
                word: String::from("γαμεῖ"),
                gloss_id: None,
                gloss_uuid: None,
                word_type: WordType::Word,
            },
        ];

        let mut glosses_hash = HashMap::new();
        for g in glosses.clone() {
            glosses_hash.insert(g.gloss_id, g.clone());
        }

        let mut aw = HashMap::new();
        for s in sequence.arrowed_words.arrow.clone() {
            aw.insert(s.word_id, s.gloss_id);
        }

        let glosses_occurrances = make_gloss_occurrances(&words, &aw, &glosses_hash, &mut 0);

        let mut gloss_occurrances_hash = HashMap::new();
        for g in glosses_occurrances {
            gloss_occurrances_hash.insert(g.gloss_id, g.clone());
        }

        let text = Text {
            text_id: 1,
            text_name: String::from(""),
            display: true,
            words: Words { word: words },
            pages: vec![],
        };
        let export = ExportLatex {};
        let p = make_document(&sequence.name, &[text], &gloss_occurrances_hash, &export, 1);
        println!("test: \n{p}");
    }

    #[test]
    fn load_from_file() {
        assert!(load_sequence("testsequence.xml", "output.tex"));
    }

    #[test]
    fn make() {
        let s = r#"Περὶ πολλοῦ ἂν ποιησαίμην, ὦ ἄνδρες, τὸ τοιούτους ὑμᾶς ἐμοὶ δικαστὰς περὶ τούτου τοῦ πράγματος γενέσθαι, οἷοίπερ ἂν ὑμῖν αὐτοῖς εἴητε τοιαῦτα πεπονθότες· εὖ γὰρ οἶδ' ὅτι, εἰ τὴν αὐτὴν γνώμην περὶ τῶν ἄλλων ἔχοιτε, ἥνπερ περὶ ὑμῶν αὐτῶν, οὐκ ἂν εἴη ὅστις οὐκ ἐπὶ τοῖς γεγενημένοις ἀγανακτοίη, ἀλλὰ πάντες ἂν περὶ τῶν τὰ τοιαῦτα ἐπιτηδευόντων τὰς ζημίας μικρὰς ἡγοῖσθε.\hspace{0pt}\marginsec{2} καὶ ταῦτα οὐκ ἂν εἴη μόνον παρ' ὑμῖν οὕτως ἐγνωσμένα, ἀλλ' ἐν ἁπάσῃ τῇ Ἑλλάδι· περὶ τούτου γὰρ μόνου τοῦ ἀδικήματος καὶ ἐν
            δημοκρατίᾳ καὶ ὀλιγαρχίᾳ ἡ αὐτὴ τιμωρία τοῖς ἀσθενεστάτοις πρὸς τοὺς τὰ μέγιστα δυναμένους ἀποδέδοται, ὥστε τὸν χείριστον τῶν αὐτῶν τυγχάνειν τῷ βελτίστῳ· οὕτως, ὦ ἄνδρες, ταύτην τὴν ὕβριν ἅπαντες ἄνθρωποι δεινοτάτην ἡγοῦνται.\hspace{0pt}\marginsec{3} περὶ μὲν οὖν τοῦ μεγέθους τῆς ζημίας ἅπαντας ὑμᾶς νομίζω τὴν αὐτὴν διάνοιαν ἔχειν,\hspace*{\fill}"#;
        for (i, w) in s.split(" ").enumerate() {
            println!(
                "<word id=\"{}\" gloss_id=\"1\" type=\"Word\">{}</word>",
                i, w
            );
        }
    }
}
