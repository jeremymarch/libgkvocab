mod exportlatex;

#[allow(unused_imports)]
use exportlatex::ExportLatex;
use serde::{Deserialize, Serialize};
use serde_xml_rs::from_str;
use serde_xml_rs::ser::Serializer;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use uuid::Uuid;
use xml::writer::EmitterConfig;

#[derive(Debug, PartialEq)]
pub enum GlosserError {
    NotFound(String),
    InvalidInput(String),
    Other(String),
}

impl fmt::Display for GlosserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GlosserError::NotFound(msg) => write!(f, "Not found: {}", msg),
            GlosserError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            GlosserError::Other(msg) => write!(f, "Other error: {}", msg),
        }
    }
}

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
    #[serde(rename = "@uuid")]
    uuid: Uuid,
    #[serde(rename = "@parent_uuid")]
    parent_id: Option<Uuid>,
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
    #[serde(rename = "@uuid")]
    uuid: Uuid,
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
    #[serde(rename = "@gloss_uuid")]
    gloss_uuid: Uuid,
    #[serde(rename = "@word_uuid")]
    word_uuid: Uuid,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Sequence {
    sequence_id: i32,
    name: String,
    start_page: usize,
    gloss_names: Vec<String>,
    texts: TextsContainer,
    arrowed_words: ArrowedWordsContainer,
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

//for the index of arrowed words at the back of the book
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
pub struct TextsContainer {
    text: Vec<TextDescription>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ArrowedWordsContainer {
    #[serde(rename = "arrow")]
    arrowed_words: Vec<GlossArrow>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Words {
    word: Vec<Word>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AppCrit {
    #[serde(rename = "@word_uuid", default)]
    word_uuid: Uuid,
    #[serde(rename = "#text")]
    entry: String,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AppCritsContainer {
    #[serde(rename = "appcrit")]
    appcrits: Vec<AppCrit>,
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
    appcrits: Option<AppCritsContainer>,
    #[serde(default)]
    words_per_page: String,
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
    gloss_id: Uuid,
    lemma: String,
    sort_alpha: String,
    gloss: String,
    arrowed_seq: Option<usize>,
    arrowed_state: ArrowedState,
}

pub trait ExportDocument {
    fn gloss_entry(&self, lemma: &str, gloss: &str, arrowed: bool) -> String;
    fn make_text(&self, words: &[Word], appcrit_hash: &HashMap<Uuid, String>) -> String;
    fn page_start(&self, title: &str) -> String;
    fn page_end(&self) -> String;
    fn page_gloss_start(&self) -> String;
    fn document_end(&self) -> String;
    fn document_start(&self, title: &str, start_page: usize) -> String;
    fn make_index(&self, arrowed_words_index: &[ArrowedWordsIndex]) -> String;
    fn blank_page(&self) -> String;
}

#[allow(clippy::too_many_arguments)]
pub fn make_page(
    words: &[Word],
    gloss_hash: &HashMap<Uuid, GlossOccurrance>,
    appcrit_hash: &HashMap<Uuid, String>,
    seq_offset: usize,
    export: &impl ExportDocument,
    title: &str,
    arrowed_words_index: &mut Vec<ArrowedWordsIndex>,
    page_number: usize,
) -> String {
    let mut page = export.page_start(title);
    page.push_str(&export.make_text(words, appcrit_hash));

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
    gloss_hash: &HashMap<Uuid, GlossOccurrance>,
    appcrit_hash: &HashMap<Uuid, String>,
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
                        appcrit_hash,
                        overall_index,
                        export,
                        if i == 0 { "" } else { &t.text_name },
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
                        appcrit_hash,
                        overall_index,
                        export,
                        if i == 0 { "" } else { &t.text_name },
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

pub fn sanitize_greek(s: &str) -> String {
    s.replace('\u{1F71}', "\u{03AC}") //acute -> tonos, etc...
        .replace('\u{1FBB}', "\u{0386}")
        .replace('\u{1F73}', "\u{03AD}")
        .replace('\u{1FC9}', "\u{0388}")
        .replace('\u{1F75}', "\u{03AE}")
        .replace('\u{1FCB}', "\u{0389}")
        .replace('\u{1F77}', "\u{03AF}")
        .replace('\u{1FDB}', "\u{038A}")
        .replace('\u{1F79}', "\u{03CC}")
        .replace('\u{1FF9}', "\u{038C}")
        .replace('\u{1F7B}', "\u{03CD}")
        .replace('\u{1FEB}', "\u{038E}")
        .replace('\u{1F7D}', "\u{03CE}")
        .replace('\u{1FFB}', "\u{038F}")
        .replace('\u{1FD3}', "\u{0390}") //iota + diaeresis + acute
        .replace('\u{1FE3}', "\u{03B0}") //upsilon + diaeresis + acute
        .replace('\u{037E}', "\u{003B}") //semicolon
        .replace('\u{0387}', "\u{00B7}") //middle dot
        .replace('\u{0344}', "\u{0308}\u{0301}")
}

//sets arrowed state and makes glosses unique on page
pub fn make_gloss_page(
    words: &[Word],
    glosshash: &HashMap<Uuid, GlossOccurrance>,
    seq_offset: usize,
    arrowed_words_index: &mut Vec<ArrowedWordsIndex>,
    page_number: usize,
) -> Vec<GlossOccurrance> {
    let mut glosses: HashMap<Uuid, GlossOccurrance> = HashMap::new();

    for (seq, w) in words.iter().enumerate() {
        if let Some(gloss_uuid) = w.gloss_uuid
            && let Some(gloss) = glosshash.get(&gloss_uuid)
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
            if g.arrowed_state == ArrowedState::Arrowed || !glosses.contains_key(&gloss_uuid) {
                glosses.insert(gloss_uuid, g);
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
            ArrowedState::Arrowed => res.push_str(
                export
                    .gloss_entry(&sanitize_greek(&g.lemma), &g.gloss, true)
                    .as_str(),
            ),
            ArrowedState::Visible => res.push_str(
                export
                    .gloss_entry(&sanitize_greek(&g.lemma), &g.gloss, false)
                    .as_str(),
            ),
            ArrowedState::Invisible => (),
        }
    }
    res
}

//sets figures out seq where each gloss is arrowed, arrowed_state is set to a dummy value;
//really arrowed_seq is set in make_gloss_page
pub fn make_gloss_occurrances(
    words: &[Word],
    arrowed_words: &HashMap<Uuid, Uuid>,
    glosses_hash: &HashMap<Uuid, Gloss>,
    seq_offset: &mut usize,
) -> Vec<GlossOccurrance> {
    //get sequence where the gloss is arrowed
    let mut glosses_seq = HashMap::new();
    for (seq, w) in words.iter().enumerate() {
        if let Some(arrowed_word_gloss) = arrowed_words.get(&w.uuid)
            && let Some(gloss) = w.gloss_uuid
            && *arrowed_word_gloss == gloss
        {
            glosses_seq.insert(gloss, seq + *seq_offset);
        }
    }
    *seq_offset += words.len();

    let mut r = vec![];
    for w in words {
        if let Some(gloss_uuid) = w.gloss_uuid
            && let Some(gloss) = glosses_hash.get(&gloss_uuid)
        {
            if let Some(gloss_seq) = glosses_seq.get(&gloss_uuid) {
                r.push(GlossOccurrance {
                    gloss_id: gloss_uuid,
                    lemma: gloss.lemma.clone(),
                    sort_alpha: gloss.sort_alpha.clone(),
                    gloss: gloss.def.clone(),
                    arrowed_seq: Some(*gloss_seq),
                    arrowed_state: ArrowedState::Visible, //this is actually set later
                });
            } else {
                r.push(GlossOccurrance {
                    gloss_id: gloss_uuid,
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

pub fn load_sequence(file_path: &str, output_path: &str) -> Result<(), GlosserError> {
    if let Ok(contents) = fs::read_to_string(file_path)
        && let Ok(sequence) = Sequence::from_xml(&contents)
    {
        let seq_dir = if let Some(last_slash_index) = file_path.rfind('/') {
            file_path[..last_slash_index].to_string()
        } else {
            String::from("")
        };

        let mut texts = vec![];
        let mut glosses = vec![];
        let mut appcrit_hash = HashMap::new();

        for g in &sequence.gloss_names {
            let gloss_path = format!("{}/{}", seq_dir, g);
            if let Ok(contents) = fs::read_to_string(&gloss_path)
                && let Ok(gloss) = Glosses::from_xml(&contents)
            {
                glosses.push(gloss);
            } else {
                println!("Error reading gloss");
                return Err(GlosserError::NotFound(format!(
                    "Gloss not found: {}",
                    gloss_path
                )));
            }
        }

        for t in &sequence.texts.text {
            let text_path = format!("{}/{}", seq_dir, t.text);
            if let Ok(contents) = fs::read_to_string(&text_path)
                && let Ok(mut text) = Text::from_xml(&contents)
            {
                text.display = t.display;
                texts.push(text);
            } else {
                println!("Error reading text");
                return Err(GlosserError::NotFound(format!(
                    "Text not found: {}",
                    text_path
                )));
            }
        }

        if !texts.is_empty() && !glosses.is_empty() {
            let mut glosses_hash = HashMap::new();
            for ggg in glosses {
                for g in ggg.gloss.clone() {
                    glosses_hash.insert(g.uuid, g.clone());
                }
            }

            let mut aw = HashMap::new();
            for s in sequence.arrowed_words.arrowed_words.clone() {
                aw.insert(s.word_uuid, s.gloss_uuid);
            }

            if verify_arrowed_words(
                &texts,
                &aw,
                &glosses_hash,
                &sequence.arrowed_words.arrowed_words,
            ) {
                return Err(GlosserError::InvalidInput(String::from(
                    "Invalid arrowed words",
                )));
            }

            let mut glosses_occurrances: Vec<GlossOccurrance> = vec![];
            let mut offset = 0;
            for t in &texts {
                if let Some(appcrits) = &t.appcrits {
                    for ap in &appcrits.appcrits {
                        appcrit_hash.insert(ap.word_uuid, ap.entry.clone());
                    }
                }
                // if t.appcrits.is_some() {
                //     for ap in &t.appcrits.as_ref().unwrap().appcrits {
                //         appcrit_hash.insert(ap.word_uuid, ap.entry.clone());
                //     }
                // }
                glosses_occurrances.append(&mut make_gloss_occurrances(
                    &t.words.word,
                    &aw,
                    &glosses_hash,
                    &mut offset,
                ));
            }
            //println!("app: {}", appcrit_hash.len());

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

            for t in &mut texts {
                if !t.words_per_page.is_empty() {
                    t.pages = t
                        .words_per_page
                        .split(',')
                        .filter_map(|s| s.trim().parse::<usize>().ok())
                        .collect();
                }
            }

            let p = make_document(
                &sequence.name,
                &texts,
                &gloss_occurrances_hash,
                &appcrit_hash,
                &ExportLatex {},
                sequence.start_page,
            );
            let _ = fs::write(output_path, &p);
            //println!("testaaa: \n{p}");
        }
    } else {
        return Err(GlosserError::NotFound(String::from(
            "Gloss or texts not found",
        )));
    }
    Ok(())
}

// arrowed words:
// 1. check that word_ids are not arrowed twice
// 2. check that gloss_ids are not arrowed twice
// 3. check that arrowed word_ids actually appear in the text words
// 4. check that arrowed gloss_ids actually appear in the gloss
// 5ab. check that gloss_id for arrowed word is not None (a) AND is the same (b) gloss_id assigned to that word in the text
// 6. check that the gloss has a status which does not equal 0
//
// gloss
// check that each gloss_id only appears once
// To do: be sure gloss's parent_id, if set, exists in gloss and its status is not 0
//
// text
// 7. check that each word_id only appears once
// 8. check that the gloss_id associated with each word exists in the gloss and that its status is not 0
fn verify_arrowed_words(
    texts: &[Text],
    arrowed_words_hash: &HashMap<Uuid, Uuid>,
    glosses_hash: &HashMap<Uuid, Gloss>,
    arrowed_words: &[GlossArrow],
) -> bool {
    let mut has_errors = false;

    let mut seen_arrowed_words = HashSet::<Uuid>::new();
    let mut seen_arrowed_glosses = HashSet::<Uuid>::new();
    // check that arrowed word_ids and gloss_ids are unique:
    // a word should not be arrowed twice
    // and a gloss should not be arrowed twice
    for s in arrowed_words {
        if !seen_arrowed_words.insert(s.word_uuid) {
            println!("duplicate word_id in arrowed words {}", s.word_uuid);
            // 1
            has_errors = true;
        }
        if !seen_arrowed_glosses.insert(s.gloss_uuid) {
            println!("duplicate gloss_uuid in arrowed words {}", s.gloss_uuid);
            // 2
            has_errors = true;
        }
    }

    let mut seen_words = HashSet::<Uuid>::new();
    let count_arrowed_words = arrowed_words_hash.len();
    let mut found_arrowed_words = 0;

    for t in texts {
        for w in &t.words.word {
            if !seen_words.insert(w.uuid) {
                println!("duplicate word uuid found in words {}", w.uuid);
                // 7
                has_errors = true;
            }
            if let Some(g) = w.gloss_uuid {
                let gloss = glosses_hash.get(&g);
                if gloss.is_none() {
                    println!(
                        "gloss {} set for word {} does not exist in gloss",
                        g, w.uuid
                    );
                    // 8
                    has_errors = true;
                } else if gloss.unwrap().status == 0 {
                    println!("gloss {} set for word {} has status == 0", g, w.uuid);
                    // 8
                    has_errors = true;
                }
            }
            // go through every word in sequence, if it is arrowed
            // compare the gloss_id in arrowed list to the gloss_id assigned to the arrowed word
            if let Some(arrowed_gloss) = arrowed_words_hash.get(&w.uuid) {
                found_arrowed_words += 1;
                if w.gloss_uuid.is_none() {
                    // 5a : arrowed gloss is not set on word in text
                    has_errors = true;
                    println!("arrowed word has a gloss which is not set: {}", w.uuid);
                } else if *arrowed_gloss != w.gloss_uuid.unwrap() {
                    let a = glosses_hash.get(&w.gloss_uuid.unwrap());
                    let b = glosses_hash.get(arrowed_gloss);

                    println!(
                        "arrow gloss doesn't match text's gloss {} g1: {} s1: {} g2: {} s2: {}",
                        w.word,
                        a.unwrap().status,
                        a.unwrap().lemma,
                        b.unwrap().status,
                        b.unwrap().lemma,
                    );
                    // 5b
                    has_errors = true;
                } else if glosses_hash.get(arrowed_gloss).is_none() {
                    // 4 : arrowed gloss exists in gloss
                    has_errors = true;
                    println!(
                        "arrowed gloss id does not exist in gloss: {}",
                        arrowed_gloss
                    );
                } else if let Some(g) = glosses_hash.get(arrowed_gloss)
                    && g.status == 0
                {
                    // 6 :  status != 0
                    has_errors = true;
                    println!("gloss with status 0 is arrowed: {}", arrowed_gloss);
                }
            }
        }
    }

    if count_arrowed_words != found_arrowed_words {
        // 3 number of arrowed words does not match number found in words
        has_errors = true;
        println!(
            "didn't find correct number of arrowed words; arrowed: {}, found in texts: {}",
            count_arrowed_words, found_arrowed_words
        );
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
                uuid: Uuid::new_v4(),
                parent_id: None,
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
                uuid: Uuid::new_v4(),
                parent_id: None,
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
                uuid: Uuid::new_v4(),
                parent_id: None,
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
            arrowed_words: ArrowedWordsContainer {
                arrowed_words: vec![
                    GlossArrow {
                        word_uuid: Uuid::new_v4(),
                        gloss_uuid: Uuid::new_v4(),
                    },
                    GlossArrow {
                        word_uuid: Uuid::new_v4(),
                        gloss_uuid: Uuid::new_v4(),
                    },
                    GlossArrow {
                        word_uuid: Uuid::new_v4(),
                        gloss_uuid: Uuid::new_v4(),
                    },
                ],
            },
            texts: TextsContainer {
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
                uuid: Uuid::new_v4(),
                word: String::from("βλάπτει"),
                gloss_uuid: None,
                word_type: WordType::Word,
            },
            Word {
                uuid: Uuid::new_v4(),
                word: String::from("γαμεῖ"),
                gloss_uuid: None,
                word_type: WordType::Word,
            },
            Word {
                uuid: Uuid::new_v4(),
                word: String::from("ἄγει"),
                gloss_uuid: None,
                word_type: WordType::Word,
            },
            Word {
                uuid: Uuid::new_v4(),
                word: String::from("βλάπτει"),
                gloss_uuid: None,
                word_type: WordType::Word,
            },
            Word {
                uuid: Uuid::new_v4(),
                word: String::from("ἄγει"),
                gloss_uuid: None,
                word_type: WordType::Word,
            },
            Word {
                uuid: Uuid::new_v4(),
                word: String::from("γαμεῖ"),
                gloss_uuid: None,
                word_type: WordType::Word,
            },
            Word {
                uuid: Uuid::new_v4(),
                word: String::from("βλάπτει"),
                gloss_uuid: None,
                word_type: WordType::Word,
            },
            Word {
                uuid: Uuid::new_v4(),
                word: String::from("βλάπτει"),
                gloss_uuid: None,
                word_type: WordType::Word,
            },
            Word {
                uuid: Uuid::new_v4(),
                word: String::from("ἄγεις"),
                gloss_uuid: None,
                word_type: WordType::Word,
            },
            Word {
                uuid: Uuid::new_v4(),
                word: String::from("ἄγεις"),
                gloss_uuid: None,
                word_type: WordType::Word,
            },
            Word {
                uuid: Uuid::new_v4(),
                word: String::from("γαμεῖ"),
                gloss_uuid: None,
                word_type: WordType::Word,
            },
            Word {
                uuid: Uuid::new_v4(),
                word: String::from("γαμεῖ"),
                gloss_uuid: None,
                word_type: WordType::Word,
            },
        ];

        let mut glosses_hash = HashMap::new();
        for g in glosses.clone() {
            glosses_hash.insert(g.uuid, g.clone());
        }

        let mut aw = HashMap::new();
        for s in sequence.arrowed_words.arrowed_words.clone() {
            aw.insert(s.word_uuid, s.gloss_uuid);
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
            appcrits: Some(AppCritsContainer { appcrits: vec![] }),
            words_per_page: String::from(""),
        };
        let export = ExportLatex {};
        let appcrit_hash = HashMap::new();
        let p = make_document(
            &sequence.name,
            &[text],
            &gloss_occurrances_hash,
            &appcrit_hash,
            &export,
            1,
        );
        println!("test: \n{p}");
    }

    #[test]
    fn load_from_file() {
        assert_eq!(
            load_sequence(
                "../gkvocab_data/testsequence.xml",
                "../gkvocab_data/ulg.tex"
            ),
            Ok(())
        );
    }

    /*
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

    #[test]
    fn write_gloss_uuids() {
        let seq_path = "../gkvocab_data/testsequence.xml";

        let seq_dir = if let Some(last_slash_index) = seq_path.rfind('/') {
            seq_path[..last_slash_index].to_string()
        } else {
            String::from("")
        };

        if let Ok(contents) = fs::read_to_string(seq_path)
            && let Ok(sequence) = Sequence::from_xml(&contents)
        {
            let mut texts = vec![];
            let mut glosses = vec![];

            for g in &sequence.gloss_names {
                let gloss_path = format!("{}/{}", seq_dir, g);
                if let Ok(contents) = fs::read_to_string(gloss_path)
                    && let Ok(gloss) = Glosses::from_xml(&contents)
                {
                    glosses.push(gloss);
                } else {
                    println!("Error reading gloss");
                    return;
                }
            }

            for t in &sequence.texts.text {
                let text_path = format!("{}/{}", seq_dir, t.text);
                if let Ok(contents) = fs::read_to_string(text_path)
                    && let Ok(mut text) = Text::from_xml(&contents)
                {
                    text.display = t.display;
                    texts.push(text);
                } else {
                    println!("Error reading text");
                    return;
                }
            }

            let mut glosses_hash = HashMap::new();
            for ggg in glosses {
                for g in ggg.gloss.clone() {
                    glosses_hash.insert(g.gloss_id, g.clone());
                }
            }

            let mut word_id_hash = HashMap::new();
            for t in &mut texts {
                for w in &mut t.words.word {
                    word_id_hash.insert(w.word_id, w.uuid);
                }
            }

            let mut i = 0;
            for t in &mut texts {
                // for w in &mut t.words.word {
                //     if w.gloss_id.is_some()
                //         && let Some(g) = glosses_hash.get(&w.gloss_id.unwrap())
                //     {
                //         w.gloss_uuid = Some(g.uuid);
                //     }
                // }

                if t.appcrits.is_some() {
                    for ap in &mut t.appcrits.as_mut().unwrap().appcrit {
                        ap.word_uuid = *word_id_hash.get(&ap.word_id).unwrap();
                    }
                }

                let s = t.to_xml();
                let _ = fs::write(
                    format!("../gkvocab_data/{}", sequence.texts.text[i].text),
                    s,
                );
                i += 1;
            }
        }
    }

    #[test]
    fn write_seq_uuids() {
        let seq_path = "../gkvocab_data/testsequence.xml";

        let seq_dir = if let Some(last_slash_index) = seq_path.rfind('/') {
            seq_path[..last_slash_index].to_string()
        } else {
            String::from("")
        };

        if let Ok(contents) = fs::read_to_string(seq_path)
            && let Ok(sequence) = Sequence::from_xml(&contents)
        {
            let mut texts = vec![];
            let mut glosses = vec![];

            for g in &sequence.gloss_names {
                let gloss_path = format!("{}/{}", seq_dir, g);
                if let Ok(contents) = fs::read_to_string(gloss_path)
                    && let Ok(gloss) = Glosses::from_xml(&contents)
                {
                    glosses.push(gloss);
                } else {
                    println!("Error reading gloss");
                    return;
                }
            }

            for t in &sequence.texts.text {
                let text_path = format!("{}/{}", seq_dir, t.text);
                if let Ok(contents) = fs::read_to_string(text_path)
                    && let Ok(mut text) = Text::from_xml(&contents)
                {
                    text.display = t.display;
                    texts.push(text);
                } else {
                    println!("Error reading text");
                    return;
                }
            }

            let mut glosses_hash = HashMap::new();
            for ggg in glosses {
                for g in ggg.gloss.clone() {
                    glosses_hash.insert(g.gloss_id, g.clone());
                }
            }

            let mut word_id_hash = HashMap::new();
            let mut word_string_hash = HashMap::new();

            for (i, mut t) in &mut texts.into_iter().enumerate() {
                for w in &mut t.words.word {
                    if w.gloss_id.is_some()
                        && let Some(g) = glosses_hash.get(&w.gloss_id.unwrap())
                    {
                        //w.gloss_uuid = Some(g.uuid);
                        word_id_hash.insert(w.word_id, w.uuid);
                        word_string_hash.insert(w.word_id, w.word.clone());
                    }
                }
                // let s = t.to_xml();
                // let _ = fs::write(
                //     format!("../gkvocab_data/{}", sequence.texts.text[i].text),
                //     s,
                // );
            }

            for s in sequence.arrowed_words.arrow.clone() {
                let word_uuid = word_id_hash.get(&s.word_id).unwrap();
                let gloss_uuid = glosses_hash.get(&s.gloss_id).unwrap().uuid;
                let word_word = word_string_hash.get(&s.word_id).unwrap();

                println!(
                    "<arrow gloss_id=\"{}\" gloss_uuid=\"{}\" word_id=\"{}\" word_uuid=\"{}\" /> <!-- {} -->",
                    s.gloss_id, gloss_uuid, s.word_id, word_uuid, word_word
                );
            }
        }
    }
    */
}
