#[allow(dead_code)]
pub mod exporthtml;
pub mod exportlatex;

//https://www.reddit.com/r/rust/comments/1ggl7am/how_to_use_typst_as_programmatically_using_rust/
//
use quick_xml::Reader;
use quick_xml::events::Event;
use quick_xml::name::QName;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::str::FromStr;
use uuid::Uuid;

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

pub struct GlossPageOptions {
    pub filter_unique: bool,
    pub filter_invisible: bool,
    pub sort_alpha: bool,
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

#[derive(Default, Clone, Copy, Debug, PartialEq)]
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
    #[default]
    InvalidType = 13,
    InlineVerseSpeaker = 14,
}

impl FromStr for WordType {
    type Err = String; // Define the error type for parsing failures

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Word" => Ok(WordType::Word),
            "Punctuation" => Ok(WordType::Punctuation),
            "Speaker" => Ok(WordType::Speaker),
            "Section" => Ok(WordType::Section),
            "VerseLine" => Ok(WordType::VerseLine),
            "ParaWithIndent" => Ok(WordType::ParaWithIndent),
            "WorkTitle" => Ok(WordType::WorkTitle),
            "SectionTitle" => Ok(WordType::SectionTitle),
            "InlineSpeaker" => Ok(WordType::InlineSpeaker),
            "ParaNoIndent" => Ok(WordType::ParaNoIndent),
            "PageBreak" => Ok(WordType::PageBreak),
            "Desc" => Ok(WordType::Desc),
            "InvalidType" => Ok(WordType::InvalidType),
            "InlineVerseSpeaker" => Ok(WordType::InlineVerseSpeaker),
            _ => Err(format!("'{}' is not a valid variant for WordType", s)),
        }
    }
}

impl fmt::Display for WordType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            WordType::Word => write!(f, "Word"),
            WordType::Punctuation => write!(f, "Punctuation"),
            WordType::Speaker => write!(f, "Speaker"),
            WordType::Section => write!(f, "Section"),
            WordType::VerseLine => write!(f, "VerseLine"),
            WordType::ParaWithIndent => write!(f, "ParaWithIndent"),
            WordType::WorkTitle => write!(f, "WorkTitle"),
            WordType::SectionTitle => write!(f, "SectionTitle"),
            WordType::InlineSpeaker => write!(f, "InlineSpeaker"),
            WordType::ParaNoIndent => write!(f, "ParaNoIndent"),
            WordType::PageBreak => write!(f, "PageBreak"),
            WordType::Desc => write!(f, "Desc"),
            WordType::InvalidType => write!(f, "InvalidType"),
            WordType::InlineVerseSpeaker => write!(f, "InlineVerseSpeaker"),
        }
    }
}

#[derive(Default, Clone, Debug, PartialEq)]
pub struct Gloss {
    pub uuid: GlossUuid,
    pub parent_id: Option<GlossUuid>,
    pub lemma: String,
    pub sort_alpha: String,
    pub def: String,
    pub pos: String,
    pub unit: i32,
    pub note: String,
    pub updated: String,
    pub status: i32,
    pub updated_user: String,
}

#[derive(Default, Clone, Debug, PartialEq)]
pub struct Word {
    uuid: WordUuid,
    gloss_uuid: Option<GlossUuid>,
    word_type: WordType,
    word: String,
}

//the word id where a gloss is arrowed
#[derive(Default, Clone, Debug, PartialEq)]
pub struct GlossArrow {
    gloss_uuid: GlossUuid,
    word_uuid: WordUuid,
}

#[derive(Default, Clone, Debug, PartialEq)]
pub struct SequenceDescription {
    name: String,
    start_page: usize,
    gloss_names: Vec<String>,
    texts: Vec<TextDescription>,
    arrowed_words: Vec<GlossArrow>,
}

impl SequenceDescription {
    pub fn to_xml(&self) -> Result<String, quick_xml::Error> {
        write_seq_desc_xml(self)
    }

    pub fn from_xml(s: &str) -> Result<SequenceDescription, quick_xml::Error> {
        read_seq_desc_xml(s)
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

#[derive(Default, Clone, Debug, PartialEq)]
pub struct TextDescription {
    display: bool,
    text: String,
}

#[derive(Default, Clone, Debug, PartialEq)]
pub struct AppCrit {
    word_uuid: WordUuid,
    entry: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    text_name: String,
    words: Vec<Word>,
    appcrits: Option<Vec<AppCrit>>,
    words_per_page: String,
}

impl Text {
    pub fn to_xml(&self) -> Result<String, quick_xml::Error> {
        write_text_xml(self)
    }

    pub fn from_xml(s: &str) -> Result<Text, quick_xml::Error> {
        read_text_xml(s)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Glosses {
    gloss_name: String,
    gloss: Vec<Gloss>,
}

impl Glosses {
    pub fn to_xml(&self) -> Result<String, quick_xml::Error> {
        write_gloss_xml(self)
    }

    pub fn from_xml(s: &str) -> Result<Glosses, quick_xml::Error> {
        read_gloss_xml(s)
    }
}

#[derive(Clone, Debug)]
pub struct Sequence {
    sequence_description: SequenceDescription,
    glosses: Vec<Glosses>,
    texts: Vec<Text>,
}

impl Sequence {
    pub fn from_xml(file_path: &str) -> Result<Sequence, GlosserError> {
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

            for t in &seq.sequence_description.texts {
                let text_path = format!("{}/{}", seq_dir, t.text);
                if let Ok(contents) = fs::read_to_string(&text_path)
                    && let Ok(text) = Text::from_xml(&contents)
                {
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

    pub fn to_xml(&self, output_path: &str, seq_name: &str) -> Result<(), quick_xml::Error> {
        let sx = self.sequence_description.to_xml()?;
        let _ = fs::write(format!("{}/{}", output_path, seq_name), &sx);
        for (i, g) in self.glosses.iter().enumerate() {
            let gx = g.to_xml()?;
            let _ = fs::write(
                format!(
                    "{}/{}",
                    output_path, self.sequence_description.gloss_names[i]
                ),
                &gx,
            );
        }
        for (i, t) in self.texts.iter().enumerate() {
            let tx = t.to_xml()?;
            let _ = fs::write(
                format!(
                    "{}/{}",
                    output_path, self.sequence_description.texts[i].text
                ),
                &tx,
            );
        }
        Ok(())
    }

    pub fn process(&self) -> Result<Vec<Vec<GlossOccurrance<'_>>>, GlosserError> {
        if !self.texts.is_empty() && !self.glosses.is_empty() {
            let mut glosses_hash = HashMap::new();
            for ggg in &self.glosses {
                for g in &ggg.gloss {
                    glosses_hash.insert(g.uuid, g);
                }
            }

            let mut arrowed_words_hash: HashMap<WordUuid, GlossUuid> = HashMap::new();
            for s in &self.sequence_description.arrowed_words {
                arrowed_words_hash.insert(s.word_uuid, s.gloss_uuid);
            }

            if self.verify(&arrowed_words_hash, &glosses_hash).is_err() {
                return Err(GlosserError::InvalidInput(String::from(
                    "Invalid input: Has errors",
                )));
            }

            let mut gloss_seq_count: HashMap<GlossUuid, GlossSeqCount> = HashMap::new();

            let mut res: Vec<Vec<GlossOccurrance>> = vec![];
            let mut i = 0;
            for t in &self.texts {
                let mut text_vec = vec![];
                for w in &t.words {
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
                            // if g.lemma == "περί" {
                            //     println!("{} {} {}", g.lemma, gsc.count, t.text_name);
                            // }
                            gsc.count += 1;
                            running_count = Some(gsc.count);
                            real_gloss_seq = gsc.arrowed_seq;
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

                res.push(text_vec);
            }

            //now we can set gloss total counts, since we've gone through the whole sequence of words
            for text in &mut res {
                for gloss_occurrance in text {
                    if let Some(go_g) = gloss_occurrance.gloss
                        && let Some(gsc) = gloss_seq_count.get(&go_g.uuid)
                    {
                        gloss_occurrance.total_count = Some(gsc.count);
                    }
                }
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
    fn verify(
        &self,
        arrowed_words_hash: &HashMap<WordUuid, GlossUuid>,
        glosses_hash: &HashMap<GlossUuid, &Gloss>,
    ) -> Result<(), GlosserError> {
        //let mut has_errors = false;

        let mut seen_arrowed_words = HashSet::<WordUuid>::new();
        let mut seen_arrowed_glosses = HashSet::<GlossUuid>::new();
        // check that arrowed word_ids and gloss_ids are unique:
        // a word should not be arrowed twice
        // and a gloss should not be arrowed twice
        for s in &self.sequence_description.arrowed_words {
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

        for t in &self.texts {
            for w in &t.words {
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
                            return Err(
                                GlosserError::ReferencedGlossIdDoesNotExistInGlossOrInvalid(
                                    format!("gloss {} set for word {} has status == 0", g, w.uuid),
                                ),
                            );
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

    pub fn get_glosses(&self, key: &str, num: usize) -> (Vec<Gloss>, Option<Uuid>) {
        use std::collections::BTreeMap;
        use std::ops::Bound;
        use std::ops::Bound::{Excluded, Included, Unbounded};

        let mut map: BTreeMap<String, &Gloss> = BTreeMap::new();

        for g in &self.glosses {
            for gg in &g.gloss {
                if gg.status > 0 {
                    map.insert(gg.sort_alpha.to_lowercase(), gg);
                }
            }
        }

        let mut res_before: Vec<Gloss> = map
            .range::<str, (Bound<&str>, Bound<&str>)>((Unbounded, Excluded(key)))
            // Reverse the iterator to start from the items closest to the search key
            .rev()
            .take(if num > 0 { num - 1 } else { 0 })
            .map(|(_key, value)| (*value).clone())
            .collect();

        let mut res_equal_and_after: Vec<Gloss> = map
            .range::<str, (Bound<&str>, Bound<&str>)>((Included(key), Unbounded))
            .take(num)
            .map(|(_key, value)| (*value).clone())
            .collect();

        let selected = if res_equal_and_after.is_empty() {
            None
        } else {
            Some(res_equal_and_after.first().unwrap().uuid)
        };

        res_before.reverse();
        res_before.append(&mut res_equal_and_after);
        (res_before, selected)
    }

    pub fn make_document(
        &self,
        gloss_occurrances: &[Vec<GlossOccurrance>],
        export: &impl ExportDocument,
        options: &GlossPageOptions,
    ) -> String {
        let mut arrowed_words_index: Vec<ArrowedWordsIndex> = vec![];
        let mut page_number = self.sequence_description.start_page;

        let mut appcrit_hash = HashMap::new();
        for t in &self.texts {
            if let Some(appcrits) = &t.appcrits {
                for ap in appcrits {
                    appcrit_hash.insert(ap.word_uuid, ap.entry.clone());
                }
            }
        }

        let mut doc = export.document_start(&self.sequence_description.name, page_number);
        //if page_number is even, insert blank page
        if page_number.is_multiple_of(2) {
            doc.push_str(export.blank_page().as_str());
            page_number += 1;
        }
        let mut text_index = 0;
        for (t_idx, t) in self.texts.iter().enumerate() {
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
            // should this text be displayed or not?: former used a variabld on the text itself
            if !self.sequence_description.texts[t_idx].display {
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
                            options,
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
                            options,
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

    pub fn make_single_page(
        &self,
        gloss_occurrances: &[Vec<GlossOccurrance>],
        export: &impl ExportDocument,
        options: &GlossPageOptions,
        selected_page_number: usize,
    ) -> String {
        let mut arrowed_words_index: Vec<ArrowedWordsIndex> = vec![];
        let mut page_number = self.sequence_description.start_page;

        let appcrit_hash = HashMap::new();
        // for t in &seq.texts {
        //     if let Some(appcrits) = &t.appcrits {
        //         for ap in &appcrits.appcrits {
        //             appcrit_hash.insert(ap.word_uuid, ap.entry.clone());
        //         }
        //     }
        // }

        let doc = String::from(""); //export.document_start(&seq.sequence_description.name, page_number);
        //if page_number is even, insert blank page

        let mut text_index = 0;
        for (t_idx, t) in self.texts.iter().enumerate() {
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
            // should this text be displayed or not?: former used a variabld on the text itself
            if !self.sequence_description.texts[t_idx].display {
                text_index += 1;
                continue;
            }

            for (i, w) in pages.iter().enumerate() {
                if i == pages.len() - 1 {
                    if page_number == selected_page_number {
                        return make_page(
                            &gloss_occurrances[text_index][index..],
                            &appcrit_hash,
                            export,
                            if i == 0 { "" } else { &t.text_name },
                            &mut arrowed_words_index,
                            page_number,
                            options,
                        );
                    }
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
                    if page_number == selected_page_number {
                        return make_page(
                            &gloss_occurrances[text_index][index..index + w],
                            &appcrit_hash,
                            export,
                            if i == 0 { "" } else { &t.text_name },
                            &mut arrowed_words_index,
                            page_number,
                            options,
                        );
                    }
                    index += w;
                }
                page_number += 1;
            }
            text_index += 1;
        }

        //doc.push_str(&export.document_end());
        doc //error
    }
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

//used for index
fn get_small_lemma(s: &str) -> String {
    let a = s.split(",");
    let mut res = String::from("");
    for parts in a {
        if parts.trim() != "—" {
            res = parts.trim().to_string();
            break;
        }
    }
    if res.is_empty() {
        res = s.trim().to_string();
    }
    res
}

fn filter_and_sort_glosses<'a>(
    gloss_occurrances: &'a [GlossOccurrance],
    arrowed_words_index: &mut Vec<ArrowedWordsIndex>,
    page_number: usize,
    options: &GlossPageOptions,
) -> Vec<GlossOccurrance<'a>> {
    let mut unique: HashMap<GlossUuid, GlossOccurrance> = HashMap::new();
    let mut sorted_glosses: Vec<GlossOccurrance> = vec![];
    for g in gloss_occurrances {
        if g.word.word_type == WordType::Word {
            if options.filter_invisible && g.arrowed_state == ArrowedState::Invisible {
                continue;
            }
            if let Some(gg) = &g.gloss {
                if g.arrowed_state == ArrowedState::Arrowed {
                    arrowed_words_index.push(ArrowedWordsIndex {
                        gloss_lemma: get_small_lemma(&gg.lemma),
                        gloss_sort: gg.sort_alpha.to_owned(),
                        page_number,
                    });
                }
                if options.filter_unique {
                    if g.arrowed_state == ArrowedState::Arrowed || !unique.contains_key(&gg.uuid) {
                        unique.insert(gg.uuid, g.clone());
                    }
                } else {
                    sorted_glosses.push(g.clone());
                }
            } else if !options.filter_invisible {
                sorted_glosses.push(g.clone());
            }
        }
    }

    if options.filter_unique {
        sorted_glosses = unique.values().cloned().collect();
    }
    if options.sort_alpha {
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

fn make_page(
    gloss_occurrances: &[GlossOccurrance],
    appcrit_hash: &HashMap<WordUuid, String>,
    export: &impl ExportDocument,
    title: &str,
    arrowed_words_index: &mut Vec<ArrowedWordsIndex>,
    page_number: usize,
    options: &GlossPageOptions,
) -> String {
    let mut page = export.page_start(title, page_number);
    page.push_str(&export.make_text(gloss_occurrances, appcrit_hash));

    page.push_str(&export.page_gloss_start());

    let v = filter_and_sort_glosses(gloss_occurrances, arrowed_words_index, page_number, options);

    page.push_str(&get_gloss_string(&v, export));

    page.push_str(&export.page_end());
    page
}

fn sanitize_greek(s: &str) -> String {
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

fn get_gloss_string(glosses: &[GlossOccurrance], export: &impl ExportDocument) -> String {
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

// fn count_lines(gloss_occurances: &[GlossOccurrance]) {
//     let text_lines_per_page = 50;
//     let width_of_line = 1000;
//     let width_of_lemma = 400;
//     let width_of_def = 400;

//     for go in gloss_occurances {
//         //let current_text_width += get_width(go.word);
//         //current
//     }
// }

fn read_seq_desc_xml(xml: &str) -> Result<SequenceDescription, quick_xml::Error> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut(); //.trim_text(true); // Trim whitespace from text nodes
    //reader.config_mut().trim_text(true); //we don't want this since it trims spaces around entities e.g. &lt;
    reader.config_mut().enable_all_checks(true);
    reader.config_mut().expand_empty_elements = true;

    let mut buf = Vec::new();

    //let mut glosses = vec![];
    let mut texts = vec![];
    let mut arrowed_words = vec![];

    let mut current_seq_desc: SequenceDescription = Default::default();
    let mut current_text: TextDescription = Default::default();

    let mut tags = vec![];
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                if b"SequenceDefinition" == e.name().as_ref() {
                    current_seq_desc = Default::default();
                } else if b"text" == e.name().as_ref() {
                    for attribute_result in e.attributes() {
                        match attribute_result {
                            Ok(attr) => {
                                if attr.key == QName(b"display") {
                                    let display = std::str::from_utf8(&attr.value).unwrap();
                                    current_text.display = display != "false";
                                }
                            }
                            Err(e) => eprintln!("Error reading attribute: {:?}", e),
                        }
                    }
                } else if b"arrow" == e.name().as_ref() {
                    let mut gloss_uuid: Option<Uuid> = None;
                    let mut word_uuid: Option<Uuid> = None;
                    for attribute_result in e.attributes() {
                        match attribute_result {
                            Ok(attr) => {
                                if attr.key == QName(b"gloss_uuid") {
                                    gloss_uuid = Some(
                                        Uuid::parse_str(std::str::from_utf8(&attr.value).unwrap())
                                            .unwrap(),
                                    );
                                } else if attr.key == QName(b"word_uuid") {
                                    word_uuid = Some(
                                        Uuid::parse_str(std::str::from_utf8(&attr.value).unwrap())
                                            .unwrap(),
                                    );
                                }
                            }
                            Err(e) => eprintln!("Error reading attribute: {:?}", e),
                        }
                    }
                    if let Some(g) = gloss_uuid
                        && let Some(w) = word_uuid
                    {
                        arrowed_words.push(GlossArrow {
                            gloss_uuid: g,
                            word_uuid: w,
                        });
                    } else {
                        panic!();
                    }
                }

                let name: String = std::str::from_utf8(e.name().local_name().as_ref())
                    .expect("Invalid UTF-8")
                    .to_string();
                tags.push(name);
            }
            Ok(Event::GeneralRef(e)) => {
                let text = get_entity(e.decode().unwrap());
                if let Some(this_tag) = tags.last()
                    && !text.is_empty()
                {
                    match this_tag.as_ref() {
                        "name" => current_seq_desc.name.push_str(text),
                        "start_page" => current_seq_desc.start_page = text.parse().unwrap(),
                        "gloss_name" => current_seq_desc.gloss_names.push(text.to_string()),
                        "text" => current_text.text.push_str(text),
                        _ => (), //println!("unknown tag: {}", this_tag),
                    }
                }
            }
            Ok(Event::Text(e)) => {
                if let Ok(text) = e.decode()
                    && let Some(this_tag) = tags.last()
                {
                    match this_tag.as_ref() {
                        "name" => current_seq_desc.name.push_str(&text),
                        "start_page" => current_seq_desc.start_page = text.parse().unwrap(),
                        "gloss_name" => current_seq_desc.gloss_names.push(text.to_string()),
                        "text" => current_text.text.push_str(&text),
                        _ => (), //println!("unknown tag: {}", this_tag),
                    }
                }
            }
            Ok(Event::End(e)) => {
                tags.pop();
                if b"text" == e.name().as_ref() {
                    texts.push(current_text.clone());
                    current_text = Default::default();
                }
            }
            Ok(Event::Eof) => break, // End of file
            Err(e) => panic!("Error at position {}: {:?}", reader.error_position(), e),
            _ => (), // Ignore other event types like comments, processing instructions, etc.
        }
        buf.clear(); // Clear buffer for the next event
    }
    current_seq_desc.arrowed_words = arrowed_words;
    current_seq_desc.texts = texts;
    Ok(current_seq_desc)
}

fn read_gloss_xml(xml: &str) -> Result<Glosses, quick_xml::Error> {
    let mut res: Vec<Gloss> = vec![];
    let mut reader = Reader::from_str(xml);
    reader.config_mut(); //.trim_text(true); // Trim whitespace from text nodes
    //reader.config_mut().trim_text(true); //we don't want this since it trims spaces around entities e.g. &lt;
    reader.config_mut().enable_all_checks(true);
    reader.config_mut().expand_empty_elements = true;

    let mut buf = Vec::new();

    let mut current_gloss: Gloss = Default::default();
    let mut gloss_name = String::from("");

    let mut tags = vec![];
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                if b"gloss" == e.name().as_ref() {
                    current_gloss = Default::default();
                    for attribute_result in e.attributes() {
                        match attribute_result {
                            Ok(attr) => {
                                if attr.key == QName(b"uuid") {
                                    current_gloss.uuid =
                                        Uuid::parse_str(std::str::from_utf8(&attr.value).unwrap())
                                            .unwrap();
                                }
                                // let key = String::from_utf8_lossy(attr.key.as_ref());
                                // let value = String::from_utf8_lossy(&attr.value);
                                // if key == "uuid" {
                                //     current_gloss.uuid = Uuid::parse_str(&value).unwrap()
                                // }
                            }
                            Err(e) => eprintln!("Error reading attribute: {:?}", e),
                        }
                    }
                } else if b"glosses" == e.name().as_ref() {
                    for attribute_result in e.attributes() {
                        match attribute_result {
                            Ok(attr) => {
                                if attr.key == QName(b"gloss_name") {
                                    gloss_name =
                                        std::str::from_utf8(&attr.value).unwrap().to_string();
                                }
                                // let key = String::from_utf8_lossy(attr.key.as_ref());
                                // let value = String::from_utf8_lossy(&attr.value);
                                // if key == "uuid" {
                                //     current_gloss.uuid = Uuid::parse_str(&value).unwrap()
                                // }
                            }
                            Err(e) => eprintln!("Error reading attribute: {:?}", e),
                        }
                    }
                }
                let name = String::from_utf8(e.name().as_ref().to_vec()).unwrap();
                //println!()
                tags.push(name);
            }
            Ok(Event::GeneralRef(e)) => {
                let text = get_entity(e.decode().unwrap());
                if let Some(this_tag) = tags.last()
                    && !text.is_empty()
                {
                    match this_tag.as_ref() {
                        "lemma" => current_gloss.lemma.push_str(text),
                        "sort_alpha" => current_gloss.sort_alpha.push_str(text),
                        "parent_id" => {
                            current_gloss.parent_id = if text.trim().is_empty() {
                                None
                            } else {
                                Some(Uuid::parse_str(text).unwrap())
                            };
                        }
                        "def" => current_gloss.def.push_str(text),
                        "pos" => current_gloss.pos.push_str(text),
                        "unit" => current_gloss.unit = text.parse().unwrap(),
                        "status" => current_gloss.status = text.parse().unwrap(),
                        "note" => current_gloss.note.push_str(text),
                        "updated" => current_gloss.updated.push_str(text),
                        "updated_user" => current_gloss.updated_user.push_str(text),
                        _ => (), //println!("unknown tag: {}", this_tag),
                    }
                }
            }
            Ok(Event::Text(e)) => {
                if let Ok(text) = e.decode()
                    && let Some(this_tag) = tags.last()
                {
                    //println!("this tag: {}: {}", this_tag, text);
                    match this_tag.as_ref() {
                        "lemma" => current_gloss.lemma.push_str(&text),
                        "sort_alpha" => current_gloss.sort_alpha.push_str(&text),
                        "parent_id" => {
                            current_gloss.parent_id = if text.trim().is_empty() {
                                None
                            } else {
                                Some(Uuid::parse_str(text.as_ref()).unwrap())
                            };
                        }
                        "def" => current_gloss.def.push_str(&text),
                        "pos" => current_gloss.pos.push_str(&text),
                        "unit" => current_gloss.unit = text.parse().unwrap(),
                        "status" => current_gloss.status = text.parse().unwrap(),
                        "note" => current_gloss.note.push_str(&text),
                        "updated" => current_gloss.updated.push_str(&text),
                        "updated_user" => current_gloss.updated_user.push_str(&text),
                        _ => (), //println!("unknown tag: {}", this_tag),
                    }
                }
            }
            Ok(Event::End(e)) => {
                tags.pop();
                if b"gloss" == e.name().as_ref() {
                    res.push(current_gloss.clone());
                }
            }
            Ok(Event::Eof) => break, // End of file
            Err(e) => panic!("Error at position {}: {:?}", reader.error_position(), e),
            _ => (), // Ignore other event types like comments, processing instructions, etc.
        }
        buf.clear(); // Clear buffer for the next event
    }
    Ok(Glosses {
        gloss_name,
        gloss: res,
    })
}

fn write_seq_desc_xml(seq_desc: &SequenceDescription) -> Result<String, quick_xml::Error> {
    use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
    use quick_xml::writer::Writer;
    use std::io::Cursor;

    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);

    writer.write_event(Event::Start(BytesStart::new("sequence_description")))?;
    writer
        .create_element("name")
        .write_text_content(BytesText::new(&seq_desc.name))?;
    writer
        .create_element("start_page")
        .write_text_content(BytesText::new(&seq_desc.start_page.to_string()))?;

    if !seq_desc.gloss_names.is_empty() {
        writer.write_event(Event::Start(BytesStart::new("glosses")))?;
    }
    for g in &seq_desc.gloss_names {
        writer
            .create_element("gloss_name")
            .write_text_content(BytesText::new(g))?;
    }
    if !seq_desc.gloss_names.is_empty() {
        writer.write_event(Event::End(BytesEnd::new("glosses")))?;
    }

    if !seq_desc.texts.is_empty() {
        writer.write_event(Event::Start(BytesStart::new("texts")))?;
    }

    for t in &seq_desc.texts {
        writer
            .create_element("text")
            .with_attribute(("display", t.display.to_string().as_str()))
            .write_text_content(BytesText::new(&t.text))?;
    }
    if !seq_desc.texts.is_empty() {
        writer.write_event(Event::End(BytesEnd::new("texts")))?;
    }

    if !seq_desc.arrowed_words.is_empty() {
        writer.write_event(Event::Start(BytesStart::new("arrowed_words")))?;
    }

    for a in &seq_desc.arrowed_words {
        writer
            .create_element("arrow")
            .with_attribute(("gloss_uuid", a.gloss_uuid.to_string().as_str()))
            .with_attribute(("word_uuid", a.word_uuid.to_string().as_str()))
            .write_empty()?;
    }
    if !seq_desc.arrowed_words.is_empty() {
        writer.write_event(Event::End(BytesEnd::new("arrowed_words")))?;
    }

    writer.write_event(Event::End(BytesEnd::new("sequence_description")))?;

    let result = writer.into_inner().into_inner();
    Ok(std::str::from_utf8(&result).unwrap().to_string())
}

fn write_gloss_xml(gloss: &Glosses) -> Result<String, quick_xml::Error> {
    use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
    use quick_xml::writer::Writer;
    use std::io::Cursor;

    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);

    let mut gloss_start = BytesStart::new("glosses");
    gloss_start.push_attribute(("gloss_name", gloss.gloss_name.as_str()));
    writer.write_event(Event::Start(gloss_start))?;

    for g in &gloss.gloss {
        writer
            .create_element("gloss")
            .with_attribute(("uuid", g.uuid.to_string().as_str()))
            .write_inner_content(|writer| {
                writer
                    .create_element("lemma")
                    .write_text_content(BytesText::new(&g.lemma))?;
                writer
                    .create_element("sort_alpha")
                    .write_text_content(BytesText::new(&g.sort_alpha))?;
                writer
                    .create_element("def")
                    .write_text_content(BytesText::new(&g.def))?;
                writer
                    .create_element("pos")
                    .write_text_content(BytesText::new(&g.pos))?;
                writer
                    .create_element("unit")
                    .write_text_content(BytesText::new(&g.unit.to_string()))?;
                writer
                    .create_element("note")
                    .write_text_content(BytesText::new(&g.note))?;
                writer
                    .create_element("updated")
                    .write_text_content(BytesText::new(&g.updated))?;
                writer
                    .create_element("status")
                    .write_text_content(BytesText::new(&g.status.to_string()))?;
                writer
                    .create_element("updated_user")
                    .write_text_content(BytesText::new(&g.updated_user))?;
                Ok(())
            })?;
    }

    writer.write_event(Event::End(BytesEnd::new("glosses")))?;

    let result = writer.into_inner().into_inner();
    Ok(std::str::from_utf8(&result).unwrap().to_string())
}

fn write_text_xml(text: &Text) -> Result<String, quick_xml::Error> {
    use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
    use quick_xml::writer::Writer;
    use std::io::Cursor;

    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);

    let mut gloss_start = BytesStart::new("text");
    gloss_start.push_attribute(("text_name", text.text_name.as_str()));
    writer.write_event(Event::Start(gloss_start))?;
    if !text.words.is_empty() {
        writer.write_event(Event::Start(BytesStart::new("words")))?;
    }
    for w in &text.words {
        if let Some(gloss_uuid) = w.gloss_uuid {
            writer
                .create_element("word")
                .with_attribute(("uuid", w.uuid.to_string().as_str()))
                .with_attribute(("gloss_uuid", gloss_uuid.to_string().as_str()))
                .with_attribute(("type", w.word_type.to_string().as_str()))
                .write_text_content(BytesText::new(&w.word))?;
        } else {
            writer
                .create_element("word")
                .with_attribute(("uuid", w.uuid.to_string().as_str()))
                .with_attribute(("type", w.word_type.to_string().as_str()))
                .write_text_content(BytesText::new(&w.word))?;
        }
    }
    if !text.words.is_empty() {
        writer.write_event(Event::End(BytesEnd::new("words")))?;
    }

    if let Some(appcrits) = text.appcrits.as_ref() {
        if !appcrits.is_empty() {
            writer.write_event(Event::Start(BytesStart::new("appcrits")))?;
        }
        for a in appcrits {
            writer
                .create_element("appcrit")
                .with_attribute(("word_uuid", a.word_uuid.to_string().as_str()))
                .write_text_content(BytesText::new(&a.entry))?;
        }
        if !appcrits.is_empty() {
            writer.write_event(Event::End(BytesEnd::new("appcrits")))?;
        }
    }

    writer
        .create_element("words_per_page")
        .write_text_content(BytesText::new(&text.words_per_page))?;

    writer.write_event(Event::End(BytesEnd::new("text")))?;

    let result = writer.into_inner().into_inner();
    Ok(std::str::from_utf8(&result).unwrap().to_string())
}

fn read_text_xml(xml: &str) -> Result<Text, quick_xml::Error> {
    let mut res: Vec<Word> = vec![];
    let mut appcrits: Vec<AppCrit> = vec![];
    let mut reader = Reader::from_str(xml);
    reader.config_mut(); //.trim_text(true); // Trim whitespace from text nodes
    //reader.config_mut().trim_text(true); //we don't want this since it trims spaces around entities e.g. &lt;
    reader.config_mut().enable_all_checks(true);
    reader.config_mut().expand_empty_elements = true;

    let mut buf = Vec::new();

    let mut current_word: Word = Default::default();
    let mut current_appcrit: AppCrit = Default::default();
    let mut text_name = String::from("");
    let mut words_per_page = String::from("");

    let mut tags = vec![];
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                if b"word" == e.name().as_ref() {
                    current_word = Default::default();
                    for attribute_result in e.attributes() {
                        match attribute_result {
                            Ok(attr) => {
                                if attr.key == QName(b"uuid") {
                                    current_word.uuid =
                                        Uuid::parse_str(std::str::from_utf8(&attr.value).unwrap())
                                            .unwrap();
                                } else if attr.key == QName(b"gloss_uuid") {
                                    if let Ok(gloss_uuid) =
                                        Uuid::parse_str(std::str::from_utf8(&attr.value).unwrap())
                                    {
                                        current_word.gloss_uuid = Some(gloss_uuid);
                                    } else {
                                        current_word.gloss_uuid = None;
                                    }
                                } else if attr.key == QName(b"type") {
                                    current_word.word_type =
                                        std::str::from_utf8(&attr.value).unwrap().parse().unwrap();
                                }
                                // let key = String::from_utf8_lossy(attr.key.as_ref());
                                // let value = String::from_utf8_lossy(&attr.value);
                                // if key == "uuid" {
                                //     current_gloss.uuid = Uuid::parse_str(&value).unwrap()
                                // }
                            }
                            Err(e) => eprintln!("Error reading attribute: {:?}", e),
                        }
                    }
                } else if b"appcrit" == e.name().as_ref() {
                    current_appcrit = Default::default();
                    for attribute_result in e.attributes() {
                        match attribute_result {
                            Ok(attr) => {
                                if attr.key == QName(b"word_uuid") {
                                    current_appcrit.word_uuid =
                                        Uuid::parse_str(std::str::from_utf8(&attr.value).unwrap())
                                            .unwrap();
                                }
                            }
                            Err(e) => eprintln!("Error reading attribute: {:?}", e),
                        }
                    }
                } else if b"text" == e.name().as_ref() {
                    for attribute_result in e.attributes() {
                        match attribute_result {
                            Ok(attr) => {
                                if attr.key == QName(b"text_name") {
                                    text_name =
                                        std::str::from_utf8(&attr.value).unwrap().to_string();
                                }
                                // let key = String::from_utf8_lossy(attr.key.as_ref());
                                // let value = String::from_utf8_lossy(&attr.value);
                                // if key == "uuid" {
                                //     current_gloss.uuid = Uuid::parse_str(&value).unwrap()
                                // }
                            }
                            Err(e) => eprintln!("Error reading attribute: {:?}", e),
                        }
                    }
                }
                let name = String::from_utf8(e.name().as_ref().to_vec()).unwrap();
                //println!()
                tags.push(name);
            }
            Ok(Event::GeneralRef(e)) => {
                let text = get_entity(e.decode().unwrap());
                if let Some(this_tag) = tags.last()
                    && !text.is_empty()
                {
                    match this_tag.as_ref() {
                        "word" => current_word.word.push_str(text),
                        "appcrit" => current_appcrit.entry.push_str(text),
                        "words_per_page" => words_per_page.push_str(text),
                        _ => (), //println!("unknown tag: {}", this_tag),
                    }
                }
            }
            Ok(Event::Text(e)) => {
                if let Ok(text) = e.decode()
                    && let Some(this_tag) = tags.last()
                {
                    //println!("this tag: {}: {}", this_tag, text);
                    match this_tag.as_ref() {
                        "word" => current_word
                            .word
                            .push_str(&quick_xml::escape::unescape(&text).unwrap()),
                        "appcrit" => current_appcrit
                            .entry
                            .push_str(&quick_xml::escape::unescape(&text).unwrap()),
                        "words_per_page" => {
                            words_per_page.push_str(&quick_xml::escape::unescape(&text).unwrap())
                        }
                        _ => (), //println!("unknown tag: {}", this_tag),
                    }
                }
            }
            Ok(Event::End(e)) => {
                tags.pop();
                if b"word" == e.name().as_ref() {
                    res.push(current_word.clone());
                }
                if b"appcrit" == e.name().as_ref() {
                    appcrits.push(current_appcrit.clone());
                }
            }
            Ok(Event::Eof) => break, // End of file
            Err(e) => panic!("Error at position {}: {:?}", reader.error_position(), e),
            _ => (), // Ignore other event types like comments, processing instructions, etc.
        }
        buf.clear(); // Clear buffer for the next event
    }
    Ok(Text {
        text_name,
        appcrits: if appcrits.is_empty() {
            None
        } else {
            Some(appcrits)
        },
        words: res,
        words_per_page,
    })
}

fn split_words(
    text: &str,
    in_speaker: bool,
    in_head: bool,
    in_desc: bool,
    lemmatizer: &HashMap<String, Uuid>,
) -> Vec<Word> {
    let mut words: Vec<Word> = vec![];
    let mut last = 0;
    let word_type_word = if in_desc {
        WordType::Desc
    } else {
        WordType::Word
    };
    if in_head {
        words.push(Word {
            uuid: Uuid::new_v4(),
            word: text.to_string(),
            word_type: WordType::WorkTitle,
            gloss_uuid: None,
        });
    } else if in_speaker {
        words.push(Word {
            uuid: Uuid::new_v4(),
            word: text.to_string(),
            word_type: WordType::Speaker,
            gloss_uuid: None,
        });
    } else {
        for (index, matched) in text.match_indices(|c: char| {
            !(c.is_alphanumeric() || c == '\'' || unicode_normalization::char::is_combining_mark(c))
        }) {
            //add words
            if last != index && &text[last..index] != " " {
                let gloss_uuid = lemmatizer.get(&text[last..index]).copied();
                words.push(Word {
                    uuid: Uuid::new_v4(),
                    word: text[last..index].to_string(),
                    word_type: word_type_word,
                    gloss_uuid,
                });
            }
            //add word separators
            if matched != " " {
                words.push(Word {
                    uuid: Uuid::new_v4(),
                    word: matched.to_string(),
                    word_type: WordType::Punctuation,
                    gloss_uuid: None,
                });
            }
            last = index + matched.len();
        }
        //add last word
        if last < text.len() && &text[last..] != " " {
            let gloss_uuid = lemmatizer.get(&text[last..]).copied();
            words.push(Word {
                uuid: Uuid::new_v4(),
                word: text[last..].to_string(),
                word_type: word_type_word,
                gloss_uuid,
            });
        }
    }
    words
}

fn get_entity(e: Cow<'_, str>) -> &str {
    match e {
        std::borrow::Cow::Borrowed("lt") => "<",
        std::borrow::Cow::Borrowed("gt") => ">",
        std::borrow::Cow::Borrowed("amp") => "&",
        std::borrow::Cow::Borrowed("apos") => "'",
        std::borrow::Cow::Borrowed("quot") => "\"",
        _ => "",
    }
}

pub fn import_text(
    xml_string: &str,
    lemmatizer: &HashMap<String, Uuid>,
) -> Result<Text, quick_xml::Error> {
    let mut words: Vec<Word> = Vec::new();

    let mut reader = Reader::from_str(xml_string);
    reader.config_mut().trim_text(true); //FIX ME: check docs, do we want true here?
    reader.config_mut().enable_all_checks(true);

    let mut buf = Vec::new();

    let mut in_text = false;
    let mut in_speaker = false;
    let mut in_head = false;
    let mut found_tei = false;
    let mut in_desc = false;
    let mut chapter_value: Option<String> = None;
    /*
    TEI: verse lines can either be empty <lb n="5"/>blah OR <l n="5">blah</l>
    see Perseus's Theocritus for <lb/> and Euripides for <l></l>
    */

    loop {
        match reader.read_event_into(&mut buf) {
            // for triggering namespaced events, use this instead:
            // match reader.read_namespaced_event(&mut buf) {
            Ok(Event::Start(ref e)) => {
                // for namespaced:
                // Ok((ref namespace_value, Event::Start(ref e)))
                if b"div" == e.name().as_ref() {
                    let mut subtype = None;
                    let mut n = None;

                    for attrib in e.attributes() {
                        //.next().unwrap().unwrap();
                        if attrib.as_ref().unwrap().key == QName(b"subtype") {
                            subtype = Some(
                                std::str::from_utf8(&attrib.unwrap().value)
                                    .unwrap()
                                    .to_string(),
                            );
                        } else if attrib.as_ref().unwrap().key == QName(b"n") {
                            n = Some(
                                std::str::from_utf8(&attrib.unwrap().value)
                                    .unwrap()
                                    .to_string(),
                            );
                        }
                    }

                    if let Some(subtype_unwraped) = subtype
                        && let Some(n_unwraped) = n
                    {
                        //if found both subtype and n attributes on div
                        match subtype_unwraped.as_str() {
                            "chapter" => chapter_value = Some(n_unwraped),
                            "section" => {
                                let reference = if chapter_value.is_some() {
                                    Some(format!(
                                        "{}.{}",
                                        chapter_value.as_ref().unwrap(),
                                        n_unwraped
                                    ))
                                } else {
                                    Some(n_unwraped)
                                };

                                if let Some(ref_value) = reference {
                                    words.push(Word {
                                        uuid: Uuid::new_v4(),
                                        word: ref_value,
                                        word_type: WordType::Section,
                                        gloss_uuid: None,
                                    });
                                }
                            }
                            _ => (),
                        }
                    }
                } else if b"text" == e.name().as_ref() {
                    in_text = true;
                } else if b"speaker" == e.name().as_ref() {
                    in_speaker = true;
                } else if b"head" == e.name().as_ref() {
                    in_head = true;
                } else if b"TEI.2" == e.name().as_ref() || b"TEI" == e.name().as_ref() {
                    found_tei = true;
                } else if b"desc" == e.name().as_ref() {
                    in_desc = true;
                    words.push(Word {
                        uuid: Uuid::new_v4(),
                        word: String::from(""),
                        word_type: WordType::ParaNoIndent,
                        gloss_uuid: None,
                    });
                } else if b"p" == e.name().as_ref() {
                    words.push(Word {
                        uuid: Uuid::new_v4(),
                        word: String::from(""),
                        word_type: WordType::ParaWithIndent,
                        gloss_uuid: None,
                    });
                } else if b"l" == e.name().as_ref() {
                    let mut line_num = String::from("");

                    for a in e.attributes() {
                        if a.as_ref().unwrap().key == QName(b"n") {
                            line_num = std::str::from_utf8(&a.unwrap().value).unwrap().to_string();
                        }
                    }
                    words.push(Word {
                        uuid: Uuid::new_v4(),
                        word: line_num.to_string(),
                        word_type: WordType::VerseLine,
                        gloss_uuid: None,
                    });
                }
            }
            // unescape and decode the text event using the reader encoding
            Ok(Event::GeneralRef(ref e)) => {
                let text = get_entity(e.decode().unwrap());

                if in_text && !text.is_empty() {
                    //let seperator = Regex::new(r"([ ,.;]+)").expect("Invalid regex");
                    let clean_string = sanitize_greek(text);
                    words.extend_from_slice(
                        &split_words(&clean_string, in_speaker, in_head, in_desc, lemmatizer)[..],
                    );
                }
            }
            // unescape and decode the text event using the reader encoding
            Ok(Event::Text(ref e)) => {
                if in_text && let Ok(s) = e.decode() {
                    //let seperator = Regex::new(r"([ ,.;]+)").expect("Invalid regex");
                    let clean_string = sanitize_greek(&s);
                    words.extend_from_slice(
                        &split_words(&clean_string, in_speaker, in_head, in_desc, lemmatizer)[..],
                    );
                }
            }
            Ok(Event::Empty(ref e)) => {
                if b"lb" == e.name().as_ref() {
                    //line beginning
                    let mut line_num = String::from("");

                    for a in e.attributes() {
                        //.next().unwrap().unwrap();
                        if a.as_ref().unwrap().key == QName(b"n") {
                            line_num = std::str::from_utf8(&a.unwrap().value).unwrap().to_string();
                        }
                    }
                    words.push(Word {
                        uuid: Uuid::new_v4(),
                        word: line_num.to_string(),
                        word_type: WordType::VerseLine,
                        gloss_uuid: None,
                    });
                } else if b"pb" == e.name().as_ref() {
                    //page beginning
                    words.push(Word {
                        uuid: Uuid::new_v4(),
                        word: String::from(""),
                        word_type: WordType::PageBreak,
                        gloss_uuid: None,
                    });
                }
            }
            Ok(Event::End(ref e)) => {
                if b"text" == e.name().as_ref() {
                    in_text = false;
                } else if b"speaker" == e.name().as_ref() {
                    in_speaker = false;
                } else if b"head" == e.name().as_ref() {
                    in_head = false;
                } else if b"desc" == e.name().as_ref() {
                    in_desc = false;
                    words.push(Word {
                        uuid: Uuid::new_v4(),
                        word: String::from(""),
                        word_type: WordType::ParaNoIndent,
                        gloss_uuid: None,
                    });
                }
            }
            Ok(Event::Eof) => break, // exits the loop when reaching end of file
            Err(e) => {
                words.clear();
                return Err(e);
            } //return empty vec on error //panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            _ => (), // There are several other `Event`s we do not consider here
        }

        // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
        buf.clear();
    }
    if !found_tei {
        //using this error for now, if doc does not even try to be tei
        return Err(quick_xml::Error::IllFormed(
            quick_xml::errors::IllFormedError::MissingDoctypeName,
        ));
    }

    Ok(Text {
        text_name: String::from(""),
        appcrits: None,
        words,
        words_per_page: String::from(""),
    })
}
/*
use tokio_postgres::{Error, NoTls};
use tokio_postgres::Client;

async fn create_tables(client: &Client) {
    // <gloss gloss_id="2524" uuid="bc659b58-6a1a-40e1-aeae-decdc1e92504">
    //   <lemma>ἄνθη, ἄνθης, ἡ</lemma>
    //   <sort_alpha>ανθη, ανθης, η</sort_alpha>
    //   <gloss>full bloom</gloss>
    //   <pos>noun</pos>
    //   <unit>0</unit>
    //   <note />
    //   <updated>2021-04-07 19:44:48</updated>
    //   <status>1</status>
    //   <updated_user />
    let create_table_sql = "
            CREATE TABLE IF NOT EXISTS glosses (
                uuid UUID PRIMARY KEY,
                gloss_name TEXT NOT NULL,
                lemma TEXT NOT NULL,
                sort_alpha TEXT NOT NULL,
                gloss TEXT NOT NULL,
                pos TEXT NOT NULL,
                unit TEXT NOT NULL,
                note TEXT,
                updated TIMESTAMP,
                status INT,
                updated_user TEXT
            )
        ";

    // 4. Execute the SQL statement
    client.batch_execute(create_table_sql).await.unwrap();
}

async fn insert_rows(client: &Client, gloss: Gloss) {
    // <gloss gloss_id="2524" uuid="bc659b58-6a1a-40e1-aeae-decdc1e92504">
    //   <lemma>ἄνθη, ἄνθης, ἡ</lemma>
    //   <sort_alpha>ανθη, ανθης, η</sort_alpha>
    //   <gloss>full bloom</gloss>
    //   <pos>noun</pos>
    //   <unit>0</unit>
    //   <note />
    //   <updated>2021-04-07 19:44:48</updated>
    //   <status>1</status>
    //   <updated_user />
    let rows_affected_multi = client
        .execute(
            "INSERT INTO users (name, email) VALUES ($1, $2), ($3, $4)",
            &[&name2, &email2, &name3, &email3],
        )
        .await?;
}
*/
#[cfg(test)]
mod tests {
    use super::*;
    use exporthtml::ExportHTML;
    use exportlatex::ExportLatex;

    #[test]
    fn test_import() {
        let source_xml = r#"<TEI.2>
            <text lang="greek">
                <head>Θύρσις ἢ ᾠδή</head>
                <speaker>Θύρσις</speaker>
                <div subtype="chapter" n="1">
                    <div subtype="section" n="1">
                        <lb rend="displayNum" n="5" />αἴκα &apos; &lt; &gt; &quot; &amp; δ᾽ αἶγα λάβῃ τῆνος γέρας, ἐς τὲ καταρρεῖ
                        <pb/>
                        <l n="10">ὁσίου γὰρ ἀνδρὸς ὅσιος ὢν ἐτύγχανον</l>
                        <desc>This is a test.</desc>
                    </div>
                </div>
            </text>
        </TEI.2>"#;

        let mut lemmatizer: HashMap<String, Uuid> = HashMap::new();
        lemmatizer.insert(
            String::from("δ"),
            Uuid::parse_str("d8a70e71-f04b-430e-98da-359a98b12931").unwrap(),
        );

        let text_struct = import_text(source_xml, &lemmatizer);
        assert!(text_struct.is_ok());

        let text_xml_string = text_struct.as_ref().unwrap().to_xml();
        assert!(text_xml_string.is_ok());

        println!("text: {}", text_xml_string.unwrap());
        let r = text_struct.unwrap().words;
        assert_eq!(r.len(), 35);
        assert_eq!(r[0].word_type, WordType::WorkTitle);
        assert_eq!(r[1].word_type, WordType::Speaker);
        assert_eq!(r[2].word_type, WordType::Section);
        assert_eq!(r[2].word, "1.1");
        assert_eq!(r[3].word_type, WordType::VerseLine);
        assert_eq!(r[3].word, "5");
        assert_eq!(r[4].word_type, WordType::Word);
        assert_eq!(
            r[10].gloss_uuid,
            Some(Uuid::parse_str("d8a70e71-f04b-430e-98da-359a98b12931").unwrap())
        );
        assert_eq!(r[16].word_type, WordType::Punctuation);
        assert_eq!(r[20].word_type, WordType::PageBreak);
        assert_eq!(r[21].word_type, WordType::VerseLine);
        assert_eq!(r[21].word, "10");
        assert_eq!(r[28].word, "");
        assert_eq!(r[28].word_type, WordType::ParaNoIndent);
        assert_eq!(r[29].word, "This");
        assert_eq!(r[29].word_type, WordType::Desc);
        assert_eq!(r[34].word, "");
        assert_eq!(r[34].word_type, WordType::ParaNoIndent);
    }

    #[test]
    fn test_read_write_gloss_xml_roundtrip() {
        let source_xml = r###"<glosses gloss_name="testgloss">
  <gloss uuid="f8d14d83-e5c8-4407-b3ad-d119887ea63d">
    <lemma>ψῡχρός, ψῡχρ, &apos; &lt; &gt; &quot; &amp; ψῡχρόν</lemma>
    <sort_alpha>ψυχροςψυχραψυχρον</sort_alpha>
    <def>cold, &apos; &lt; &gt; &quot; &amp; chilly</def>
    <pos>adjective</pos>
    <unit>0</unit>
    <note></note>
    <updated>2021-04-17 03:38:29</updated>
    <status>1</status>
    <updated_user></updated_user>
  </gloss>
  <gloss uuid="7b989de1-f161-46cb-8575-71762863ca45">
    <lemma>Νύμφη, Νύμφης, ἡ</lemma>
    <sort_alpha>Νυμφη, Νυμφης, η</sort_alpha>
    <def>minor goddess, especially of streams, pools and fountains</def>
    <pos>noun</pos>
    <unit>0</unit>
    <note></note>
    <updated>2021-04-07 19:44:48</updated>
    <status>1</status>
    <updated_user></updated_user>
  </gloss>
</glosses>"###;
        let gloss_struct = read_gloss_xml(source_xml);

        let expected_gloss_struct = Glosses {
            gloss_name: String::from("testgloss"),
            gloss: vec![
                Gloss {
                    uuid: Uuid::parse_str("f8d14d83-e5c8-4407-b3ad-d119887ea63d").unwrap(),
                    parent_id: None,
                    lemma: String::from("ψῡχρός, ψῡχρ\u{eb00}, ' < > \" & ψῡχρόν"),
                    sort_alpha: String::from("ψυχροςψυχραψυχρον"),
                    def: String::from("cold, ' < > \" & chilly"),
                    pos: String::from("adjective"),
                    unit: 0,
                    note: String::from(""),
                    updated: String::from("2021-04-17 03:38:29"),
                    status: 1,
                    updated_user: String::from(""),
                },
                Gloss {
                    uuid: Uuid::parse_str("7b989de1-f161-46cb-8575-71762863ca45").unwrap(),
                    parent_id: None,
                    lemma: String::from("Νύμφη, Νύμφης, ἡ"),
                    sort_alpha: String::from("Νυμφη, Νυμφης, η"),
                    def: String::from("minor goddess, especially of streams, pools and fountains"),
                    pos: String::from("noun"),
                    unit: 0,
                    note: String::from(""),
                    updated: String::from("2021-04-07 19:44:48"),
                    status: 1,
                    updated_user: String::from(""),
                },
            ],
        };

        let xml_string = write_gloss_xml(gloss_struct.as_ref().unwrap());

        assert_eq!(gloss_struct.unwrap(), expected_gloss_struct);
        assert_eq!(xml_string.unwrap(), source_xml);
    }

    #[test]
    fn test_read_write_text_xml_roundtrip() {
        let source_xml = r###"<text text_name="ΥΠΕΡ ΤΟΥ ΕΡΑΤΟΣΘΕΝΟΥΣ ΦΟΝΟΥ ΑΠΟΛΟΓΙΑ">
  <words>
    <word uuid="46bc20ad-bb8d-486f-a61e-fa783f0d558a" type="Section">1</word>
    <word uuid="d8a70e71-f04b-430e-98da-359a98b12931" gloss_uuid="565de2e3-bf50-49b0-bf71-757ccf34080f" type="Word">Περὶ &apos; &lt; &gt; &quot; &amp;</word>
  </words>
  <appcrits>
    <appcrit word_uuid="cc402eca-165d-4af0-9514-4c57aee17bb7">1.4 ἀγανακτήσειε Η; οὐκ ἀγανακτείση P$^1$ -οίη P$^c$</appcrit>
    <appcrit word_uuid="8680e45e-f6e0-4c9d-aed4-d0deb9470b4f">2.1 ἡγοῖσθε (OCT, Carey); ἡγεῖσθαι P</appcrit>
  </appcrits>
  <words_per_page>154, 151, 137, 72, 121, 63, 85, 107, 114, 142, 109, 79, 82, 81, 122, 99, 86, 110, 112, 151, 140, 99, 71, 117, 114, 1</words_per_page>
</text>"###;
        let text_struct = read_text_xml(source_xml);

        let expected_text_struct = Text {
            text_name: String::from("ΥΠΕΡ ΤΟΥ ΕΡΑΤΟΣΘΕΝΟΥΣ ΦΟΝΟΥ ΑΠΟΛΟΓΙΑ"),
            words: vec![
                Word {
                    uuid: Uuid::parse_str("46bc20ad-bb8d-486f-a61e-fa783f0d558a").unwrap(),
                    gloss_uuid: None,
                    word_type: WordType::Section,
                    word: String::from("1"),
                },
                Word {
                    uuid: Uuid::parse_str("d8a70e71-f04b-430e-98da-359a98b12931").unwrap(),
                    gloss_uuid: Some(
                        Uuid::parse_str("565de2e3-bf50-49b0-bf71-757ccf34080f").unwrap(),
                    ),
                    word_type: WordType::Word,
                    word: String::from("Περὶ ' < > \" &"),
                },
            ],
            appcrits: Some(vec![
                AppCrit {
                    word_uuid: Uuid::parse_str("cc402eca-165d-4af0-9514-4c57aee17bb7").unwrap(),
                    entry: String::from("1.4 ἀγανακτήσειε Η; οὐκ ἀγανακτείση P$^1$ -οίη P$^c$"),
                },
                AppCrit {
                    word_uuid: Uuid::parse_str("8680e45e-f6e0-4c9d-aed4-d0deb9470b4f").unwrap(),
                    entry: String::from("2.1 ἡγοῖσθε (OCT, Carey); ἡγεῖσθαι P"),
                },
            ]),
            words_per_page: String::from(
                "154, 151, 137, 72, 121, 63, 85, 107, 114, 142, 109, 79, 82, 81, 122, 99, 86, 110, 112, 151, 140, 99, 71, 117, 114, 1",
            ),
        };

        let xml_string = write_text_xml(text_struct.as_ref().unwrap());

        assert_eq!(text_struct.unwrap(), expected_text_struct);
        assert_eq!(xml_string.unwrap(), source_xml);
    }

    #[test]
    fn test_read_write_seq_desc_xml_roundtrip() {
        let source_xml = r###"<sequence_description>
  <name>LGI - UPPER LEVEL GREEK &apos; &lt; &gt; &quot; &amp;</name>
  <start_page>24</start_page>
  <glosses>
    <gloss_name>glosses.xml</gloss_name>
  </glosses>
  <texts>
    <text display="false">hq.xml &apos; &lt; &gt; &quot; &amp;</text>
    <text display="false">ion.xml</text>
    <text display="true">ajax.xml</text>
  </texts>
  <arrowed_words>
    <arrow gloss_uuid="da684ef2-94eb-4fcd-8967-b2483c9cf0fa" word_uuid="a6bd2ba8-7a42-47ed-9529-747ca37389f8"/>
    <arrow gloss_uuid="dc090991-55dd-4396-9309-1a5e4a5f59b8" word_uuid="3b9e30ba-df58-48c5-8e24-31bbdcb81d18"/>
  </arrowed_words>
</sequence_description>"###;
        let seq_desc_struct = read_seq_desc_xml(source_xml);

        let expected_seq_desc_struct = SequenceDescription {
            name: String::from("LGI - UPPER LEVEL GREEK ' < > \" &"),
            start_page: 24,
            gloss_names: vec![String::from("glosses.xml")],
            texts: vec![
                TextDescription {
                    display: false,
                    text: String::from("hq.xml ' < > \" &"),
                },
                TextDescription {
                    display: false,
                    text: String::from("ion.xml"),
                },
                TextDescription {
                    display: true,
                    text: String::from("ajax.xml"),
                },
            ],
            arrowed_words: vec![
                GlossArrow {
                    word_uuid: Uuid::parse_str("a6bd2ba8-7a42-47ed-9529-747ca37389f8").unwrap(),
                    gloss_uuid: Uuid::parse_str("da684ef2-94eb-4fcd-8967-b2483c9cf0fa").unwrap(),
                },
                GlossArrow {
                    word_uuid: Uuid::parse_str("3b9e30ba-df58-48c5-8e24-31bbdcb81d18").unwrap(),
                    gloss_uuid: Uuid::parse_str("dc090991-55dd-4396-9309-1a5e4a5f59b8").unwrap(),
                },
            ],
        };

        let xml_string = write_seq_desc_xml(seq_desc_struct.as_ref().unwrap());

        assert_eq!(seq_desc_struct.unwrap(), expected_seq_desc_struct);
        assert_eq!(xml_string.unwrap(), source_xml);
    }

    /*
    #[tokio::test]
    async fn postgres_import_test() {
        let (client, connection) =
            tokio_postgres::connect("host=localhost user=jwm password=1234 dbname=hc", NoTls)
                .await
                .unwrap();

        // The connection object handles the communication with the database.
        // It needs to be spawned on its own task to run concurrently.
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        // Execute a simple query to retrieve a string.
        let rows = client
            .query("SELECT $1::TEXT", &[&"hello world from tokio-postgres"])
            .await
            .unwrap();

        create_tables(&client);
    }
    */

    #[test]
    fn save_xml() {
        let seq = Sequence::from_xml("../gkvocab_data/testsequence.xml");
        assert!(seq.is_ok());

        let res = seq.unwrap().to_xml("../gkvocab_data", "testsequence.xml");
        assert!(res.is_ok());
    }

    #[test]
    fn get_glosses() {
        let seq = Sequence::from_xml("../gkvocab_data/testsequence.xml");
        assert!(seq.is_ok());
        let r = seq.unwrap().get_glosses("βε", 3);
        println!("res {:?}", r);
    }

    #[test]
    fn save_html_document_from_file() {
        let seq = Sequence::from_xml("../gkvocab_data/testsequence.xml");
        assert!(seq.is_ok());

        let gloss_occurrances = seq.as_ref().unwrap().process();
        assert!(gloss_occurrances.is_ok());

        let options = GlossPageOptions {
            filter_unique: false,
            filter_invisible: false,
            sort_alpha: false,
        };

        let doc = seq.as_ref().unwrap().make_document(
            &gloss_occurrances.unwrap(),
            &ExportHTML {},
            &options,
        );
        let output_path = "../gkvocab_data/ulgv3.html";
        let _ = fs::write(output_path, &doc);
    }

    #[test]
    fn save_latex_document_from_file() {
        let seq = Sequence::from_xml("../gkvocab_data/testsequence.xml");
        assert!(seq.is_ok());

        let gloss_occurrances = seq.as_ref().unwrap().process();
        assert!(gloss_occurrances.is_ok());

        let options = GlossPageOptions {
            filter_unique: true,
            filter_invisible: true,
            sort_alpha: true,
        };

        let doc = seq.as_ref().unwrap().make_document(
            &gloss_occurrances.unwrap(),
            &ExportLatex {},
            &options,
        );
        let output_path = "../gkvocab_data/ulgv3.tex";
        let _ = fs::write(output_path, &doc);
    }

    #[test]
    fn save_page_from_file() {
        let seq = Sequence::from_xml("../gkvocab_data/testsequence.xml");
        assert!(seq.is_ok());

        let gloss_occurrances = seq.as_ref().unwrap().process();
        assert!(gloss_occurrances.is_ok());

        let options = GlossPageOptions {
            filter_unique: false,
            filter_invisible: false,
            sort_alpha: false,
        };

        //let doc = make_document(
        let doc = seq.as_ref().unwrap().make_single_page(
            &gloss_occurrances.unwrap(),
            &ExportHTML {},
            &options,
            24,
        );
        let output_path = "../gkvocab_data/ulgv3_page.html";
        let _ = fs::write(output_path, &doc);
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
            },
            Word {
                uuid: Uuid::parse_str("7b6e9cf3-288f-4d40-b026-13f9544a9434").unwrap(),
                word: String::from("γαμεῖ"),
                gloss_uuid: Some(Uuid::parse_str("7cb7721c-c992-4178-84ce-8660d0d0e355").unwrap()),
                word_type: WordType::Word,
            },
            Word {
                uuid: Uuid::parse_str("f0d558ba-af7a-4224-867f-bc126f5ab9c7").unwrap(),
                word: String::from("ἄγει"),
                gloss_uuid: Some(Uuid::parse_str("0a2151b4-39a0-4b37-8ac8-72ea6252a1ab").unwrap()),
                word_type: WordType::Word,
            },
        ];

        let sequence = SequenceDescription {
            name: String::from("SGI"),
            start_page: 3,
            gloss_names: vec![String::from("H&Qplus")],
            arrowed_words: vec![
                GlossArrow {
                    word_uuid: Uuid::parse_str("8b8eb16b-5d74-4dc7-bce1-9d561e40d60f").unwrap(),
                    gloss_uuid: Uuid::parse_str("67e55044-10b1-426f-9247-bb680e5fe0c8").unwrap(),
                },
                GlossArrow {
                    word_uuid: Uuid::parse_str("7b6e9cf3-288f-4d40-b026-13f9544a9434").unwrap(),
                    gloss_uuid: Uuid::parse_str("7cb7721c-c992-4178-84ce-8660d0d0e355").unwrap(),
                },
                GlossArrow {
                    word_uuid: Uuid::parse_str("f0d558ba-af7a-4224-867f-bc126f5ab9c7").unwrap(),
                    gloss_uuid: Uuid::parse_str("0a2151b4-39a0-4b37-8ac8-72ea6252a1ab").unwrap(),
                },
            ],
            texts: vec![
                TextDescription {
                    display: true,
                    text: String::from("abc.xml"),
                },
                TextDescription {
                    display: true,
                    text: String::from("def.xml"),
                },
            ],
        };

        let mut glosses_hash = HashMap::new();
        for g in &glosses {
            glosses_hash.insert(g.uuid, g);
        }

        let mut arrowed_words_hash = HashMap::new();
        for s in sequence.arrowed_words.clone() {
            arrowed_words_hash.insert(s.word_uuid, s.gloss_uuid);
        }

        let text = Text {
            text_name: String::from(""),
            words,
            appcrits: Some(vec![]),
            words_per_page: String::from(""),
        };

        let seq = Sequence {
            sequence_description: sequence,
            texts: vec![text],
            glosses: vec![],
        };

        let v = seq.verify(&arrowed_words_hash, &glosses_hash);
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
            },
            Word {
                uuid: Uuid::parse_str("7b6e9cf3-288f-4d40-b026-13f9544a9434").unwrap(),
                word: String::from("γαμεῖ"),
                gloss_uuid: Some(Uuid::parse_str("7cb7721c-c992-4178-84ce-8660d0d0e355").unwrap()),
                word_type: WordType::Word,
            },
            Word {
                uuid: Uuid::parse_str("f0d558ba-af7a-4224-867f-bc126f5ab9c7").unwrap(),
                word: String::from("ἄγει"),
                gloss_uuid: Some(Uuid::parse_str("0a2151b4-39a0-4b37-8ac8-72ea6252a1ab").unwrap()),
                word_type: WordType::Word,
            },
        ];

        let sequence = SequenceDescription {
            name: String::from("SGI"),
            start_page: 3,
            gloss_names: vec![String::from("H&Qplus")],
            arrowed_words: vec![
                GlossArrow {
                    word_uuid: Uuid::parse_str("8b8eb16b-5d74-4dc7-bce1-9d561e40d60f").unwrap(),
                    gloss_uuid: Uuid::parse_str("67e55044-10b1-426f-9247-bb680e5fe0c8").unwrap(),
                },
                GlossArrow {
                    word_uuid: Uuid::parse_str("8b8eb16b-5d74-4dc7-bce1-9d561e40d60f").unwrap(),
                    gloss_uuid: Uuid::parse_str("7cb7721c-c992-4178-84ce-8660d0d0e355").unwrap(),
                },
                GlossArrow {
                    word_uuid: Uuid::parse_str("f0d558ba-af7a-4224-867f-bc126f5ab9c7").unwrap(),
                    gloss_uuid: Uuid::parse_str("0a2151b4-39a0-4b37-8ac8-72ea6252a1ab").unwrap(),
                },
            ],
            texts: vec![
                TextDescription {
                    display: true,
                    text: String::from("abc.xml"),
                },
                TextDescription {
                    display: true,
                    text: String::from("def.xml"),
                },
            ],
        };

        let mut glosses_hash = HashMap::new();
        for g in &glosses {
            glosses_hash.insert(g.uuid, g);
        }

        let mut arrowed_words_hash = HashMap::new();
        for s in sequence.arrowed_words.clone() {
            arrowed_words_hash.insert(s.word_uuid, s.gloss_uuid);
        }

        let text = Text {
            text_name: String::from(""),
            words,
            appcrits: Some(vec![]),
            words_per_page: String::from(""),
        };

        let seq = Sequence {
            sequence_description: sequence,
            texts: vec![text],
            glosses: vec![],
        };

        let v = seq.verify(&arrowed_words_hash, &glosses_hash);
        assert_eq!(
            v,
            Err(GlosserError::ArrowedWordTwice(String::from(
                "duplicate word_id in arrowed words 8b8eb16b-5d74-4dc7-bce1-9d561e40d60f"
            )))
        );
    }

    #[test]
    fn test_btree() {
        use std::collections::BTreeMap;
        use std::ops::Bound;
        use std::ops::Bound::{Included, Unbounded};
        let mut b: BTreeMap<&str, usize> = BTreeMap::new();
        b.insert("ααα", 1);
        b.insert("αββ", 2);
        b.insert("αγγ", 3);
        b.insert("αγδ", 4);

        let search_key = "αβγ";
        // Get an iterator over the range [search_key, Unbounded)
        let mut range_iter =
            b.range::<str, (Bound<&str>, Bound<&str>)>((Included(search_key), Unbounded));

        // The first element in this range will be the key-value pair
        // where the key is equal to or greater than the search_key.
        if let Some((key, value)) = range_iter.next() {
            println!(
                "First key >= {}: Key = {}, Value = {}",
                search_key, key, value
            );
        } else {
            println!("No key found equal to or greater than {}", search_key);
        }
    }
}
