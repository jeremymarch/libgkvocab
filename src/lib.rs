#[allow(dead_code)]
mod exporthtml;
mod exportlatex;

//https://www.reddit.com/r/rust/comments/1ggl7am/how_to_use_typst_as_programmatically_using_rust/
#[allow(unused_imports)]
use exporthtml::ExportHTML;
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

type WordUuid = Uuid;
type GlossUuid = Uuid;

//build a vec of these in one pass:
// we need a hash of the glosses with a gloss_id key and pointer to the gloss value in the gloss vec as value
// we need a hash of the arrowed words: word_id as key, gloss_id as value
//
// create a hash with gloss_id as key and total_count and arrowed_seq in a struct as the value
// as we build GlossOccurrance, query the arrowed_words hash which we built ahead of time
// if found add current seq value to the gloss-count-seq hash table.
// we also keep track of the running count of each gloss there which then serves as the total count at the end.
#[derive(Debug, Clone)]
pub struct GlossOccurrance<'a> {
    word: &'a Word,
    gloss: Option<&'a Gloss>,
    running_count: Option<usize>,
    total_count: Option<usize>,
    arrowed_state: ArrowedState,
}

pub struct GlossSeqCount {
    count: usize,
    arrowed_seq: Option<usize>,
}

#[derive(Debug, PartialEq)]
pub enum GlosserError {
    NotFound(String),
    InvalidInput(String),
    Other(String),
    ArrowedWordTwice(String),
    ArrowedGlossTwice(String),
    ArrowedWordNotFound(String),
    ArrowedGlossNotFound(String),
    ArrowedWordsGlossDoesNotMatchText(String), // (None or different)
    ArrowedGlossIsInvalid(String),
    DuplicateWordIdInTexts(String),
    ReferencedGlossIdDoesNotExistInGlossOrInvalid(String),
    //
    GlossParentDoesNotExistOrInvalid(String),
    NonWordTypeIsArrowed(String), // (only WordType::Word should be arrowed)
    NonWordTypeIsGlossed(String), // (glosses should only be assigned for WordType::Word)
}

impl fmt::Display for GlosserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GlosserError::NotFound(msg) => write!(f, "Not found: {}", msg),
            GlosserError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            GlosserError::Other(msg) => write!(f, "Other error: {}", msg),

            GlosserError::ArrowedWordTwice(msg) => write!(f, "Not found: {}", msg), /* 1 */
            GlosserError::ArrowedGlossTwice(msg) => write!(f, "Not found: {}", msg), /* 2 */
            GlosserError::ArrowedWordNotFound(msg) => write!(f, "Not found: {}", msg), /* 3 */
            GlosserError::ArrowedGlossNotFound(msg) => write!(f, "Not found: {}", msg), /* 4 */
            GlosserError::ArrowedWordsGlossDoesNotMatchText(msg) => write!(f, "Not found: {}", msg), /* 5 */ //(None or different)
            GlosserError::ArrowedGlossIsInvalid(msg) => write!(f, "Not found: {}", msg), /* 6 */
            GlosserError::DuplicateWordIdInTexts(msg) => write!(f, "Not found: {}", msg), /* 7 */
            GlosserError::ReferencedGlossIdDoesNotExistInGlossOrInvalid(msg) => {
                write!(f, "Not found: {}", msg)
            } /* 8 */
            GlosserError::GlossParentDoesNotExistOrInvalid(msg) => write!(f, "Not found: {}", msg), /* 9 */
            GlosserError::NonWordTypeIsArrowed(msg) => write!(f, "Not found: {}", msg), /* 10 */ //(only WordType::Word should be arrowed)
            GlosserError::NonWordTypeIsGlossed(msg) => write!(f, "Not found: {}", msg), /* 11 */ //(glosses should only be assigned for WordType::Word)
        }
    }
}

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
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Gloss {
    #[serde(rename = "@uuid")]
    uuid: GlossUuid,
    #[serde(rename = "@parent_uuid")]
    parent_id: Option<GlossUuid>,
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
    uuid: WordUuid,
    #[serde(rename = "@gloss_uuid")]
    gloss_uuid: Option<GlossUuid>,
    #[serde(rename = "@type")]
    word_type: WordType,
    #[serde(rename = "#text", default)]
    word: String,
    #[serde(skip, default)]
    running_count: usize,
}

//the word id where a gloss is arrowed
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GlossArrow {
    #[serde(rename = "@gloss_uuid")]
    gloss_uuid: GlossUuid,
    #[serde(rename = "@word_uuid")]
    word_uuid: WordUuid,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SequenceDescription {
    sequence_id: i32,
    name: String,
    start_page: usize,
    gloss_names: Vec<String>,
    texts: TextsContainer,
    arrowed_words: ArrowedWordsContainer,
}

impl SequenceDescription {
    pub fn to_xml(&self) -> String {
        let mut buffer: Vec<u8> = Vec::new();
        let writer = EmitterConfig::new()
            .perform_indent(true) // Optional: for pretty-printing
            .create_writer(&mut buffer);

        let mut serializer = Serializer::new(writer);
        self.serialize(&mut serializer).unwrap();
        String::from_utf8(buffer).expect("UTF-8 error")
    }

    pub fn from_xml(s: &str) -> Result<SequenceDescription, serde_xml_rs::Error> {
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
    word_uuid: WordUuid,
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
    #[serde(skip, default)]
    display: bool,
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
pub struct Sequence {
    sequence_description: SequenceDescription,
    glosses: Vec<Glosses>,
    texts: Vec<Text>,
}

pub trait ExportDocument {
    fn gloss_entry(&self, gloss_occurrance: &GlossOccurrance, lemma: Option<&str>) -> String;
    fn make_text(
        &self,
        gloss_occurrances: &[GlossOccurrance],
        appcrit_hash: &HashMap<WordUuid, String>,
    ) -> String;
    fn page_start(&self, title: &str, page_number: usize) -> String;
    fn page_end(&self) -> String;
    fn page_gloss_start(&self) -> String;
    fn document_end(&self) -> String;
    fn document_start(&self, title: &str, start_page: usize) -> String;
    fn make_index(&self, arrowed_words_index: &[ArrowedWordsIndex]) -> String;
    fn blank_page(&self) -> String;
}

pub fn filter_and_sort_glosses<'a>(
    gloss_occurrances: &'a [GlossOccurrance],
    arrowed_words_index: &mut Vec<ArrowedWordsIndex>,
    page_number: usize,
    filter_unique: bool,
    filter_invisible: bool,
    sort_alpha: bool,
) -> Vec<GlossOccurrance<'a>> {
    let mut unique: HashMap<GlossUuid, GlossOccurrance> = HashMap::new();
    let mut sorted_glosses: Vec<GlossOccurrance> = vec![];
    for g in gloss_occurrances {
        if g.word.word_type == WordType::Word {
            if filter_invisible && g.arrowed_state == ArrowedState::Invisible {
                continue;
            }
            if let Some(gg) = &g.gloss {
                if g.arrowed_state == ArrowedState::Arrowed {
                    arrowed_words_index.push(ArrowedWordsIndex {
                        gloss_lemma: gg.lemma.clone(),
                        gloss_sort: gg.sort_alpha.to_owned(),
                        page_number,
                    });
                }
                if filter_unique {
                    if g.arrowed_state == ArrowedState::Arrowed || !unique.contains_key(&gg.uuid) {
                        unique.insert(gg.uuid, g.clone());
                    }
                } else {
                    sorted_glosses.push(g.clone());
                }
            } else if !filter_invisible {
                sorted_glosses.push(g.clone());
            }
        }
    }

    if filter_unique {
        sorted_glosses = unique.values().cloned().collect();
    }
    if sort_alpha {
        sorted_glosses.sort_by(|a, b| {
            a.gloss
                .as_ref()
                .unwrap()
                .sort_alpha
                .to_lowercase()
                .cmp(&b.gloss.as_ref().unwrap().sort_alpha.to_lowercase())
        });
    }

    sorted_glosses
}

#[allow(clippy::too_many_arguments)]
pub fn make_page(
    gloss_occurrances: &[GlossOccurrance],
    appcrit_hash: &HashMap<WordUuid, String>,
    export: &impl ExportDocument,
    title: &str,
    arrowed_words_index: &mut Vec<ArrowedWordsIndex>,
    page_number: usize,
    filter_unique: bool,
    filter_invisible: bool,
    sort_alpha: bool,
) -> String {
    let mut page = export.page_start(title, page_number);
    page.push_str(&export.make_text(gloss_occurrances, appcrit_hash));

    page.push_str(&export.page_gloss_start());

    let v = filter_and_sort_glosses(
        gloss_occurrances,
        arrowed_words_index,
        page_number,
        filter_unique,
        filter_invisible,
        sort_alpha,
    );

    page.push_str(&get_gloss_string(&v, export));

    page.push_str(&export.page_end());
    page
}

pub fn make_document(
    seq: &Sequence,
    gloss_occurrances: &[Vec<GlossOccurrance>],
    export: &impl ExportDocument,
    filter_unique: bool,
    filter_invisible: bool,
    sort_alpha: bool,
) -> String {
    let mut arrowed_words_index: Vec<ArrowedWordsIndex> = vec![];
    let mut page_number = seq.sequence_description.start_page;

    let mut appcrit_hash = HashMap::new();
    for t in &seq.texts {
        if let Some(appcrits) = &t.appcrits {
            for ap in &appcrits.appcrits {
                appcrit_hash.insert(ap.word_uuid, ap.entry.clone());
            }
        }
    }

    let mut doc = export.document_start(&seq.sequence_description.name, page_number);
    //if page_number is even, insert blank page
    if page_number.is_multiple_of(2) {
        doc.push_str(export.blank_page().as_str());
        page_number += 1;
    }
    let mut text_index = 0;
    for t in &seq.texts {
        //set pages vector from comma separated string
        let mut pages: Vec<usize> = vec![];
        if !t.words_per_page.is_empty() {
            pages = t
                .words_per_page
                .split(',')
                .filter_map(|s| s.trim().parse::<usize>().ok())
                .collect();
        }

        let mut index = 0;
        if !t.display {
            text_index += 1;
            continue;
        }

        for (i, w) in pages.iter().enumerate() {
            if i == pages.len() - 1 {
                doc.push_str(
                    make_page(
                        &gloss_occurrances[text_index][index..],
                        &appcrit_hash,
                        export,
                        if i == 0 { "" } else { &t.text_name },
                        &mut arrowed_words_index,
                        page_number,
                        filter_unique,
                        filter_invisible,
                        sort_alpha,
                    )
                    .as_str(),
                );
                let count = gloss_occurrances[text_index].len() - index;
                index += count;
            } else {
                if gloss_occurrances[text_index].len() < index + w {
                    println!(
                        "go out of range text: {}, len: {}, range: {}",
                        text_index,
                        gloss_occurrances[text_index].len(),
                        index + w
                    );
                    //return String::from("");
                    continue;
                }
                // else {
                //     println!("go in range text: {}", text_index);
                // }
                doc.push_str(
                    make_page(
                        &gloss_occurrances[text_index][index..index + w],
                        &appcrit_hash,
                        export,
                        if i == 0 { "" } else { &t.text_name },
                        &mut arrowed_words_index,
                        page_number,
                        filter_unique,
                        filter_invisible,
                        sort_alpha,
                    )
                    .as_str(),
                );
                index += w;
            }
            page_number += 1;
        }
        if !page_number.is_multiple_of(2) {
            page_number += 1;
            doc.push_str(export.blank_page().as_str());
        }
        doc.push_str(export.blank_page().as_str());
        page_number += 1;
        text_index += 1;
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

pub fn get_gloss_string(glosses: &[GlossOccurrance], export: &impl ExportDocument) -> String {
    let mut res = String::from("");
    for g in glosses {
        if let Some(some_gloss) = g.gloss {
            res.push_str(
                export
                    .gloss_entry(g, Some(&sanitize_greek(&some_gloss.lemma)))
                    .as_str(),
            );
        } else {
            res.push_str(export.gloss_entry(g, None).as_str());
        }
    }
    res
}

pub fn load_sequence(file_path: &str) -> Result<Sequence, GlosserError> {
    if let Ok(contents) = fs::read_to_string(file_path)
        && let Ok(sequence) = SequenceDescription::from_xml(&contents)
    {
        let mut seq = Sequence {
            sequence_description: sequence,
            texts: vec![],
            glosses: vec![],
        };

        let seq_dir = if let Some(last_slash_index) = file_path.rfind('/') {
            file_path[..last_slash_index].to_string()
        } else {
            String::from("")
        };

        for g in &seq.sequence_description.gloss_names {
            let gloss_path = format!("{}/{}", seq_dir, g);
            if let Ok(contents) = fs::read_to_string(&gloss_path)
                && let Ok(gloss) = Glosses::from_xml(&contents)
            {
                seq.glosses.push(gloss);
            } else {
                println!("Error reading gloss");
                return Err(GlosserError::NotFound(format!(
                    "Gloss not found: {}",
                    gloss_path
                )));
            }
        }

        for t in &seq.sequence_description.texts.text {
            let text_path = format!("{}/{}", seq_dir, t.text);
            if let Ok(contents) = fs::read_to_string(&text_path)
                && let Ok(mut text) = Text::from_xml(&contents)
            {
                text.display = t.display;
                seq.texts.push(text);
            } else {
                println!("Error reading text");
                return Err(GlosserError::NotFound(format!(
                    "Text not found: {}",
                    text_path
                )));
            }
        }

        if seq.texts.is_empty() || seq.glosses.is_empty() {
            return Err(GlosserError::NotFound(String::from(
                "text or gloss not found",
            )));
        }
        Ok(seq)
    } else {
        Err(GlosserError::NotFound(String::from("sequence not found")))
    }
}

pub fn sequence_to_xml(seq: &Sequence, path: &str) {
    let seq_xml = seq.sequence_description.to_xml();
    let _ = fs::write(
        format!("{}/{}", path, seq.sequence_description.name),
        seq_xml,
    );
    for (i, g) in seq.glosses.iter().enumerate() {
        let gloss_xml = g.to_xml();
        let _ = fs::write(
            format!("{}/{}", path, seq.sequence_description.gloss_names[i]),
            gloss_xml,
        );
    }
    for (i, t) in seq.texts.iter().enumerate() {
        let text_xml = t.to_xml();
        let _ = fs::write(
            format!("{}/{}", path, seq.sequence_description.texts.text[i].text),
            text_xml,
        );
    }
}

pub fn process_seq<'a>(seq: &'a Sequence) -> Result<Vec<Vec<GlossOccurrance<'a>>>, GlosserError> {
    if !seq.texts.is_empty() && !seq.glosses.is_empty() {
        let mut glosses_hash = HashMap::new();
        for ggg in &seq.glosses {
            for g in &ggg.gloss {
                glosses_hash.insert(g.uuid, g);
            }
        }

        let mut arrowed_words_hash: HashMap<WordUuid, GlossUuid> = HashMap::new();
        for s in &seq.sequence_description.arrowed_words.arrowed_words {
            arrowed_words_hash.insert(s.word_uuid, s.gloss_uuid);
        }

        if verify_arrowed_words(seq, &arrowed_words_hash, &glosses_hash).is_err() {
            return Err(GlosserError::InvalidInput(String::from(
                "Invalid input: Has errors",
            )));
        }

        let mut gloss_seq_count: HashMap<GlossUuid, GlossSeqCount> = HashMap::new();

        let mut res: Vec<Vec<GlossOccurrance>> = vec![];
        let mut i = 0;
        for t in &seq.texts {
            let mut text_vec = vec![];
            for w in &t.words.word {
                let mut gloss: Option<&Gloss> = None;
                let gloss_seq = if let Some(g) = w.gloss_uuid {
                    if let Some(temp_gloss_ref) = glosses_hash.get(&g) {
                        gloss = Some(temp_gloss_ref);
                    }
                    if let Some(arrowed_gloss_uuid) = arrowed_words_hash.get(&w.uuid)
                        && gloss.is_some()
                        && *arrowed_gloss_uuid == gloss.unwrap().uuid
                    {
                        Some(i)
                    } else {
                        None
                    }
                } else {
                    None
                };

                let mut running_count: Option<usize> = None;
                let mut real_gloss_seq: Option<usize> = None;
                if let Some(g) = gloss {
                    if let Some(gsc) = gloss_seq_count.get_mut(&g.uuid) {
                        running_count = Some(gsc.count);
                        real_gloss_seq = gsc.arrowed_seq;
                        gsc.count += 1;
                        gsc.arrowed_seq = if gsc.arrowed_seq.is_some() {
                            gsc.arrowed_seq
                        } else {
                            gloss_seq
                        };
                    } else {
                        running_count = Some(1);
                        real_gloss_seq = gloss_seq;
                        gloss_seq_count.insert(
                            g.uuid,
                            GlossSeqCount {
                                count: 1,
                                arrowed_seq: gloss_seq,
                            },
                        );
                    }
                }

                text_vec.push(GlossOccurrance {
                    word: w,
                    gloss, //gloss or None
                    arrowed_state: if real_gloss_seq.is_some() && real_gloss_seq.unwrap() < i {
                        ArrowedState::Invisible
                    } else if gloss_seq.is_some() && gloss_seq.unwrap() == i {
                        ArrowedState::Arrowed
                    } else {
                        ArrowedState::Visible
                    },
                    running_count,
                    total_count: None, //for now, we won't know total count until the end of this loop, so set it then
                });
                i += 1;
            }

            //now we can set gloss total counts, since we've gone through the whole sequence of words
            for w in &mut text_vec {
                if w.gloss.is_some()
                    && let Some(gsc) = gloss_seq_count.get(&w.gloss.as_ref().unwrap().uuid)
                {
                    w.total_count = Some(gsc.count);
                }
            }
            res.push(text_vec);
        }

        Ok(res)
    } else {
        Err(GlosserError::NotFound(String::from(
            "Gloss or texts not found",
        )))
    }
}

//1 ArrowedWordTwice
//2 ArrowedGlossTwice
//3 ArrowedWordNotFound
//4 ArrowedGlossNotFound
//5 ArrowedWordsGlossDoesNotMatchText (None or different)
//6 ArrowedGlossIsInvalid
//7 DuplicateWordIdInTexts
//8 ReferencedGlossIdDoesNotExistInGlossOrInvalid
//
//9 GlossParentDoesNotExistOrInvalid
//10 NonWordTypeIsArrowed (only WordType::Word should be arrowed)
//11 NonWordTypeIsGlossed (glosses should only be assigned for WordType::Word)
//
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
// To do9: be sure gloss's parent_id, if set, exists in gloss and its status is not 0
//
// text
// 7. check that each word_id only appears once
// 8. check that the gloss_id associated with each word exists in the gloss and that its status is not 0
//
// To do10: add check that only WordType::Words are glossed and that all arrowed words are of type WordType::Word
fn verify_arrowed_words(
    seq: &Sequence,
    arrowed_words_hash: &HashMap<WordUuid, GlossUuid>,
    glosses_hash: &HashMap<GlossUuid, &Gloss>,
) -> Result<(), GlosserError> {
    //let mut has_errors = false;

    let mut seen_arrowed_words = HashSet::<WordUuid>::new();
    let mut seen_arrowed_glosses = HashSet::<GlossUuid>::new();
    // check that arrowed word_ids and gloss_ids are unique:
    // a word should not be arrowed twice
    // and a gloss should not be arrowed twice
    for s in &seq.sequence_description.arrowed_words.arrowed_words {
        if !seen_arrowed_words.insert(s.word_uuid) {
            println!("duplicate word_id in arrowed words {}", s.word_uuid);
            // 1
            //has_errors = true;
            return Err(GlosserError::ArrowedWordTwice(format!(
                "duplicate word_id in arrowed words {}",
                s.word_uuid
            )));
        }
        if !seen_arrowed_glosses.insert(s.gloss_uuid) {
            println!("duplicate gloss_uuid in arrowed words {}", s.gloss_uuid);
            // 2
            //has_errors = true;
            return Err(GlosserError::ArrowedGlossTwice(format!(
                "duplicate gloss_uuid in arrowed words {}",
                s.gloss_uuid
            )));
        }
    }

    let mut seen_words = HashSet::<WordUuid>::new();
    let count_arrowed_words = arrowed_words_hash.len();
    let mut found_arrowed_words = 0;

    for t in &seq.texts {
        for w in &t.words.word {
            if !seen_words.insert(w.uuid) {
                println!(
                    "duplicate word uuid found in text {}, word {}",
                    t.text_name, w.uuid
                );
                // 7
                //has_errors = true;
                return Err(GlosserError::DuplicateWordIdInTexts(format!(
                    "duplicate word uuid found in text {}, word {}",
                    t.text_name, w.uuid
                )));
            }
            if let Some(g) = w.gloss_uuid {
                if w.word_type != WordType::Word {
                    println!(
                        "non-word type is glossed: text: {}, word: {}",
                        t.text_name, w.uuid
                    );
                    return Err(GlosserError::NonWordTypeIsGlossed(format!(
                        "non-word type is glossed: text: {}, word: {}",
                        t.text_name, w.uuid
                    )));
                }
                if let Some(gloss) = glosses_hash.get(&g) {
                    if gloss.status == 0 {
                        println!("gloss {} set for word {} has status == 0", g, w.uuid);
                        // 8
                        //has_errors = true;
                        return Err(GlosserError::ReferencedGlossIdDoesNotExistInGlossOrInvalid(
                            format!("gloss {} set for word {} has status == 0", g, w.uuid),
                        ));
                    }
                } else {
                    println!(
                        "gloss {} set for word {} does not exist in gloss",
                        g, w.uuid
                    );
                    // 8
                    //has_errors = true;
                    return Err(GlosserError::ReferencedGlossIdDoesNotExistInGlossOrInvalid(
                        format!(
                            "gloss {} set for word {} does not exist in gloss",
                            g, w.uuid
                        ),
                    ));
                }
            }
            // go through every word in sequence, if it is arrowed
            // compare the gloss_id in arrowed list to the gloss_id assigned to the arrowed word
            if let Some(arrowed_gloss) = arrowed_words_hash.get(&w.uuid) {
                found_arrowed_words += 1;
                if w.word_type != WordType::Word {
                    println!("non-word type is arrowed: {}", w.uuid);
                    return Err(GlosserError::NonWordTypeIsArrowed(format!(
                        "non-word type is arrowed: {}",
                        w.uuid
                    )));
                } else if w.gloss_uuid.is_none() {
                    // 5a : arrowed gloss is not set on word in text
                    //has_errors = true;
                    println!("arrowed word has a gloss which is not set: {}", w.uuid);
                    return Err(GlosserError::ArrowedWordsGlossDoesNotMatchText(format!(
                        "arrowed word has a gloss which is not set: {}",
                        w.uuid
                    )));
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
                    //has_errors = true;
                    return Err(GlosserError::ArrowedWordsGlossDoesNotMatchText(format!(
                        "arrow gloss doesn't match text's gloss {} g1: {} s1: {} g2: {} s2: {}",
                        w.word,
                        a.unwrap().status,
                        a.unwrap().lemma,
                        b.unwrap().status,
                        b.unwrap().lemma,
                    )));
                } else if glosses_hash.get(arrowed_gloss).is_none() {
                    // 4 : arrowed gloss exists in gloss
                    //has_errors = true;
                    println!(
                        "arrowed gloss id does not exist in gloss: {}",
                        arrowed_gloss
                    );

                    return Err(GlosserError::ArrowedGlossNotFound(format!(
                        "arrowed gloss id does not exist in gloss: {}",
                        arrowed_gloss
                    )));
                } else if let Some(g) = glosses_hash.get(arrowed_gloss)
                    && g.status == 0
                {
                    // 6 :  status != 0
                    //has_errors = true;
                    println!("gloss with status 0 is arrowed: {}", arrowed_gloss);
                    return Err(GlosserError::ArrowedGlossIsInvalid(format!(
                        "gloss with status 0 is arrowed: {}",
                        arrowed_gloss
                    )));
                }
            }
        }
    }

    if count_arrowed_words != found_arrowed_words {
        // 3 number of arrowed words does not match number found in words
        //has_errors = true;
        println!(
            "didn't find correct number of arrowed words; arrowed: {}, found in texts: {}",
            count_arrowed_words, found_arrowed_words
        );
        return Err(GlosserError::ArrowedWordNotFound(format!(
            "didn't find correct number of arrowed words; arrowed: {}, found in texts: {}",
            count_arrowed_words, found_arrowed_words
        )));
    }
    //has_errors
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_from_file() {
        let seq = load_sequence("../gkvocab_data/testsequence.xml");
        assert!(seq.is_ok());

        let gloss_occurrances = process_seq(seq.as_ref().unwrap());
        assert!(gloss_occurrances.is_ok());

        let do_html = false;
        if do_html {
            let filter_unique = false;
            let filter_invisible = false;
            let sort_alpha = false;
            let doc = make_document(
                seq.as_ref().unwrap(),
                &gloss_occurrances.unwrap(),
                &ExportHTML {},
                filter_unique,
                filter_invisible,
                sort_alpha,
            );
            let output_path = "../gkvocab_data/ulgv3.html";
            let _ = fs::write(output_path, &doc);
        } else {
            let filter_unique = true;
            let filter_invisible = true;
            let sort_alpha = true;
            let doc = make_document(
                seq.as_ref().unwrap(),
                &gloss_occurrances.unwrap(),
                &ExportLatex {},
                filter_unique,
                filter_invisible,
                sort_alpha,
            );
            let output_path = "../gkvocab_data/ulgv3.tex";
            let _ = fs::write(output_path, &doc);
        }
    }

    #[test]
    fn test_data() {
        let glosses = vec![
            Gloss {
                uuid: Uuid::parse_str("67e55044-10b1-426f-9247-bb680e5fe0c8").unwrap(),
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
                uuid: Uuid::parse_str("7cb7721c-c992-4178-84ce-8660d0d0e355").unwrap(),
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
                uuid: Uuid::parse_str("0a2151b4-39a0-4b37-8ac8-72ea6252a1ab").unwrap(),
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

        let words = vec![
            Word {
                uuid: Uuid::parse_str("8b8eb16b-5d74-4dc7-bce1-9d561e40d60f").unwrap(),
                word: String::from("βλάπτει"),
                gloss_uuid: Some(Uuid::parse_str("67e55044-10b1-426f-9247-bb680e5fe0c8").unwrap()),
                word_type: WordType::Word,
                running_count: 0,
            },
            Word {
                uuid: Uuid::parse_str("7b6e9cf3-288f-4d40-b026-13f9544a9434").unwrap(),
                word: String::from("γαμεῖ"),
                gloss_uuid: Some(Uuid::parse_str("7cb7721c-c992-4178-84ce-8660d0d0e355").unwrap()),
                word_type: WordType::Word,
                running_count: 0,
            },
            Word {
                uuid: Uuid::parse_str("f0d558ba-af7a-4224-867f-bc126f5ab9c7").unwrap(),
                word: String::from("ἄγει"),
                gloss_uuid: Some(Uuid::parse_str("0a2151b4-39a0-4b37-8ac8-72ea6252a1ab").unwrap()),
                word_type: WordType::Word,
                running_count: 0,
            },
        ];

        let sequence = SequenceDescription {
            sequence_id: 1,
            name: String::from("SGI"),
            start_page: 3,
            gloss_names: vec![String::from("H&Qplus")],
            arrowed_words: ArrowedWordsContainer {
                arrowed_words: vec![
                    GlossArrow {
                        word_uuid: Uuid::parse_str("8b8eb16b-5d74-4dc7-bce1-9d561e40d60f").unwrap(),
                        gloss_uuid: Uuid::parse_str("67e55044-10b1-426f-9247-bb680e5fe0c8")
                            .unwrap(),
                    },
                    GlossArrow {
                        word_uuid: Uuid::parse_str("7b6e9cf3-288f-4d40-b026-13f9544a9434").unwrap(),
                        gloss_uuid: Uuid::parse_str("7cb7721c-c992-4178-84ce-8660d0d0e355")
                            .unwrap(),
                    },
                    GlossArrow {
                        word_uuid: Uuid::parse_str("f0d558ba-af7a-4224-867f-bc126f5ab9c7").unwrap(),
                        gloss_uuid: Uuid::parse_str("0a2151b4-39a0-4b37-8ac8-72ea6252a1ab")
                            .unwrap(),
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

        let mut glosses_hash = HashMap::new();
        for g in &glosses {
            glosses_hash.insert(g.uuid, g);
        }

        let mut arrowed_words_hash = HashMap::new();
        for s in sequence.arrowed_words.arrowed_words.clone() {
            arrowed_words_hash.insert(s.word_uuid, s.gloss_uuid);
        }

        let text = Text {
            text_id: 1,
            text_name: String::from(""),
            display: true,
            words: Words { word: words },
            appcrits: Some(AppCritsContainer { appcrits: vec![] }),
            words_per_page: String::from(""),
        };

        let seq = Sequence {
            sequence_description: sequence,
            texts: vec![text],
            glosses: vec![],
        };

        let v = verify_arrowed_words(&seq, &arrowed_words_hash, &glosses_hash);
        assert!(v.is_ok());
    }

    #[test]
    fn test_data_dup_arrowed_word() {
        let glosses = vec![
            Gloss {
                uuid: Uuid::parse_str("67e55044-10b1-426f-9247-bb680e5fe0c8").unwrap(),
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
                uuid: Uuid::parse_str("7cb7721c-c992-4178-84ce-8660d0d0e355").unwrap(),
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
                uuid: Uuid::parse_str("0a2151b4-39a0-4b37-8ac8-72ea6252a1ab").unwrap(),
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

        let words = vec![
            Word {
                uuid: Uuid::parse_str("8b8eb16b-5d74-4dc7-bce1-9d561e40d60f").unwrap(),
                word: String::from("βλάπτει"),
                gloss_uuid: Some(Uuid::parse_str("67e55044-10b1-426f-9247-bb680e5fe0c8").unwrap()),
                word_type: WordType::Word,
                running_count: 0,
            },
            Word {
                uuid: Uuid::parse_str("7b6e9cf3-288f-4d40-b026-13f9544a9434").unwrap(),
                word: String::from("γαμεῖ"),
                gloss_uuid: Some(Uuid::parse_str("7cb7721c-c992-4178-84ce-8660d0d0e355").unwrap()),
                word_type: WordType::Word,
                running_count: 0,
            },
            Word {
                uuid: Uuid::parse_str("f0d558ba-af7a-4224-867f-bc126f5ab9c7").unwrap(),
                word: String::from("ἄγει"),
                gloss_uuid: Some(Uuid::parse_str("0a2151b4-39a0-4b37-8ac8-72ea6252a1ab").unwrap()),
                word_type: WordType::Word,
                running_count: 0,
            },
        ];

        let sequence = SequenceDescription {
            sequence_id: 1,
            name: String::from("SGI"),
            start_page: 3,
            gloss_names: vec![String::from("H&Qplus")],
            arrowed_words: ArrowedWordsContainer {
                arrowed_words: vec![
                    GlossArrow {
                        word_uuid: Uuid::parse_str("8b8eb16b-5d74-4dc7-bce1-9d561e40d60f").unwrap(),
                        gloss_uuid: Uuid::parse_str("67e55044-10b1-426f-9247-bb680e5fe0c8")
                            .unwrap(),
                    },
                    GlossArrow {
                        word_uuid: Uuid::parse_str("8b8eb16b-5d74-4dc7-bce1-9d561e40d60f").unwrap(),
                        gloss_uuid: Uuid::parse_str("7cb7721c-c992-4178-84ce-8660d0d0e355")
                            .unwrap(),
                    },
                    GlossArrow {
                        word_uuid: Uuid::parse_str("f0d558ba-af7a-4224-867f-bc126f5ab9c7").unwrap(),
                        gloss_uuid: Uuid::parse_str("0a2151b4-39a0-4b37-8ac8-72ea6252a1ab")
                            .unwrap(),
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

        let mut glosses_hash = HashMap::new();
        for g in &glosses {
            glosses_hash.insert(g.uuid, g);
        }

        let mut arrowed_words_hash = HashMap::new();
        for s in sequence.arrowed_words.arrowed_words.clone() {
            arrowed_words_hash.insert(s.word_uuid, s.gloss_uuid);
        }

        let text = Text {
            text_id: 1,
            text_name: String::from(""),
            display: true,
            words: Words { word: words },
            appcrits: Some(AppCritsContainer { appcrits: vec![] }),
            words_per_page: String::from(""),
        };

        let seq = Sequence {
            sequence_description: sequence,
            texts: vec![text],
            glosses: vec![],
        };

        let v = verify_arrowed_words(&seq, &arrowed_words_hash, &glosses_hash);
        assert_eq!(
            v,
            Err(GlosserError::ArrowedWordTwice(String::from(
                "duplicate word_id in arrowed words 8b8eb16b-5d74-4dc7-bce1-9d561e40d60f"
            )))
        );
    }
}
