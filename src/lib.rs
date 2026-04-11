#[allow(dead_code)]
pub mod exportfodt;
pub mod exporthtml;
pub mod exportlatex;
pub mod exporttypst;
pub mod glosses;
pub mod import;
pub mod lines;
pub mod texts;
pub mod update;

use glosses::Gloss;
use glosses::Glosses;
use texts::{Text, Word, WordType};

//https://www.reddit.com/r/rust/comments/1ggl7am/how_to_use_typst_as_programmatically_using_rust/

use icu::locale::locale;
use icu_collator::Collator;
use icu_collator::options::CaseLevel;
use icu_collator::options::CollatorOptions;
use icu_collator::options::Strength;
use icu_provider_blob::BlobDataProvider;

use quick_xml::Reader;
use quick_xml::events::Event;
use quick_xml::name::QName;

use std::collections::BTreeMap;
use std::collections::{HashMap, HashSet};
//use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};
//use ahash::AHashMap as HashMap;

use std::io::{Cursor, Read, Write};
use zip::ZipArchive;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use std::borrow::Cow;
use std::fmt;
use std::fs;
use std::ops::Bound;
use std::ops::Bound::{Excluded, Included, Unbounded};
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
    pub sort_key: bool,
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
    SetGlossWordIsArrowed,
    SetGlossWordNotFound,
    ArrowWordWrongGloss,
    ArrowWordNotFound,
    ArrowWordWordAlreadyArrowed,
    ArrowWordGlossAlreadyArrowed,
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
            GlosserError::SetGlossWordIsArrowed => write!(f, "Set Gloss: Word is already arrowed"),
            GlosserError::SetGlossWordNotFound => write!(f, "Set Gloss: Word not found"),
            GlosserError::ArrowWordWrongGloss => write!(f, "Arrow Word: Wrong gloss"),
            GlosserError::ArrowWordNotFound => write!(f, "Arrow Word: Not Found"),
            GlosserError::ArrowWordWordAlreadyArrowed => {
                write!(f, "Arrow Word: Word Already Arrowed")
            }
            GlosserError::ArrowWordGlossAlreadyArrowed => {
                write!(f, "Arrow Word: Gloss Already Arrowed")
            }
        }
    }
}

//the word id where a gloss is arrowed
#[derive(Default, Clone, Debug, PartialEq)]
pub struct GlossArrow {
    gloss_uuid: GlossUuid,
    word_uuid: WordUuid,
}

#[derive(Default, Clone, Debug, PartialEq)]
pub struct SequenceDescription {
    pub name: String,
    pub start_page: usize,
    pub gloss_names: Vec<String>,
    pub texts: Vec<TextDescription>,
    pub arrowed_words: Vec<GlossArrow>,
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
    pub display: bool,
    pub text: String, //the file_name of the text xml
    pub words_per_page: String,
    pub start: Option<WordUuid>,
    pub end: Option<WordUuid>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sequence {
    pub sequence_description: SequenceDescription,
    pub glosses: Vec<Glosses>,
    pub texts: Vec<Text>,
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
                    && let Ok(text) = Text::from_xml(&contents, None, None)
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

    fn make_glosses_hash(&self) -> HashMap<GlossUuid, &Gloss> {
        let mut glosses_hash = HashMap::default();
        for gloss_file in &self.glosses {
            for gloss in &gloss_file.gloss {
                glosses_hash.insert(gloss.uuid, gloss);
            }
        }
        glosses_hash
    }

    fn make_arrowed_words_hash(&self) -> HashMap<WordUuid, GlossUuid> {
        let mut arrowed_words_hash: HashMap<WordUuid, GlossUuid> = HashMap::default();
        for arrowed_word in &self.sequence_description.arrowed_words {
            arrowed_words_hash.insert(arrowed_word.word_uuid, arrowed_word.gloss_uuid);
        }
        arrowed_words_hash
    }

    pub fn process(&self) -> Result<Vec<Vec<GlossOccurrance<'_>>>, GlosserError> {
        if !self.texts.is_empty() && !self.glosses.is_empty() {
            let glosses_hash = self.make_glosses_hash();
            let arrowed_words_hash = self.make_arrowed_words_hash();

            if self.verify(&arrowed_words_hash, &glosses_hash).is_err() {
                return Err(GlosserError::InvalidInput(String::from(
                    "Invalid input: Has errors",
                )));
            }

            let mut gloss_seq_count: HashMap<GlossUuid, GlossSeqCount> = HashMap::default();

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

        let mut seen_arrowed_words = HashSet::<WordUuid>::default();
        let mut seen_arrowed_glosses = HashSet::<GlossUuid>::default();
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

        let mut seen_words = HashSet::<WordUuid>::default();
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
        let mut map: BTreeMap<String, &Gloss> = BTreeMap::default();

        for g in &self.glosses {
            for gg in &g.gloss {
                if gg.status > 0 {
                    map.insert(gg.sort_key.to_lowercase(), gg);
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

        let mut appcrit_hash = HashMap::default();
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
            if !self.sequence_description.texts[t_idx]
                .words_per_page
                .is_empty()
            {
                pages = self.sequence_description.texts[t_idx]
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
            let mut options = CollatorOptions::default();
            options.strength = Some(Strength::Quaternary);
            options.case_level = Some(CaseLevel::Off); //whether to distinguish case above the tertiary level
            let blob_provider = BlobDataProvider::try_new_from_static_blob(include_bytes!(
                "../greek_collation_blob.postcard"
            ))
            .unwrap();

            let collator = Collator::try_new_with_buffer_provider(
                &blob_provider,
                locale!("el-u-kn-true").into(), //kn-true means to sort numbers numerically rather than as strings
                options,
            )
            .expect("Greek collation data present");

            arrowed_words_index.sort_by(|a, b| {
                collator.as_borrowed().compare(&a.gloss_sort, &b.gloss_sort)
                // a.gloss_sort
                //     .to_lowercase()
                //     .cmp(&b.gloss_sort.to_lowercase())
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

        let appcrit_hash = HashMap::default();
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
            if !self.sequence_description.texts[t_idx]
                .words_per_page
                .is_empty()
            {
                pages = self.sequence_description.texts[t_idx]
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
    fn document_start(&self, title: &str, start_page: usize) -> String;
    fn blank_page(&self) -> String;
    fn page_start(&self, title: &str, page_number: usize) -> String;
    fn make_text(
        &self,
        gloss_occurrances: &[GlossOccurrance],
        appcrit_hash: &HashMap<WordUuid, String>,
    ) -> String;
    fn page_gloss_start(&self) -> String;
    fn gloss_entry(&self, gloss_occurrance: &GlossOccurrance, lemma: Option<&str>) -> String;
    fn page_end(&self) -> String;
    fn make_index(&self, arrowed_words_index: &[ArrowedWordsIndex]) -> String;
    fn document_end(&self) -> String;
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
    let mut unique: HashMap<GlossUuid, GlossOccurrance> = HashMap::default();
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
                        gloss_sort: gg.sort_key.to_owned(),
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
    if options.sort_key {
        let mut options = CollatorOptions::default();
        options.strength = Some(Strength::Quaternary);
        options.case_level = Some(CaseLevel::Off); //whether to distinguish case above the tertiary level
        let blob_provider = BlobDataProvider::try_new_from_static_blob(include_bytes!(
            "../greek_collation_blob.postcard"
        ))
        .unwrap();

        let collator = Collator::try_new_with_buffer_provider(
            &blob_provider,
            locale!("el-u-kn-true").into(), //kn-true means to sort numbers numerically rather than as strings
            options,
        )
        .expect("Greek collation data present");

        sorted_glosses.sort_by(|a, b| {
            collator
                .as_borrowed()
                .compare(&a.gloss.unwrap().sort_key, &b.gloss.unwrap().sort_key)
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

pub fn read_seq_desc_xml(xml: &str) -> Result<SequenceDescription, quick_xml::Error> {
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
                                } else if attr.key == QName(b"file_name") {
                                    let file_name = std::str::from_utf8(&attr.value).unwrap();
                                    current_text.text = file_name.to_string();
                                } else if attr.key == QName(b"start") {
                                    current_text.start = Some(
                                        Uuid::parse_str(std::str::from_utf8(&attr.value).unwrap())
                                            .unwrap(),
                                    );
                                } else if attr.key == QName(b"end") {
                                    current_text.end = Some(
                                        Uuid::parse_str(std::str::from_utf8(&attr.value).unwrap())
                                            .unwrap(),
                                    );
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
                        //"text" => current_text.text.push_str(text),
                        "words_per_page" => current_text.words_per_page.push_str(text),
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
                        //"text" => current_text.text.push_str(&text),
                        "words_per_page" => current_text.words_per_page.push_str(&text),
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
        if t.start.is_none() && t.end.is_none() {
            writer
                .create_element("text")
                .with_attribute(("display", t.display.to_string().as_str()))
                .with_attribute(("file_name", t.text.as_str()))
                .write_inner_content(|writer| {
                    writer
                        .create_element("words_per_page")
                        .write_text_content(BytesText::new(&t.words_per_page))?;
                    Ok(())
                })?;
        } else if let Some(start) = t.start
            && t.end.is_none()
        {
            writer
                .create_element("text")
                .with_attribute(("display", t.display.to_string().as_str()))
                .with_attribute(("file_name", t.text.as_str()))
                .with_attribute(("start", start.to_string().as_str()))
                .write_inner_content(|writer| {
                    writer
                        .create_element("words_per_page")
                        .write_text_content(BytesText::new(&t.words_per_page))?;
                    Ok(())
                })?;
        } else if let Some(start) = t.start
            && let Some(end) = t.end
        {
            writer
                .create_element("text")
                .with_attribute(("display", t.display.to_string().as_str()))
                .with_attribute(("file_name", t.text.as_str()))
                .with_attribute(("start", start.to_string().as_str()))
                .with_attribute(("end", end.to_string().as_str()))
                .write_inner_content(|writer| {
                    writer
                        .create_element("words_per_page")
                        .write_text_content(BytesText::new(&t.words_per_page))?;
                    Ok(())
                })?;
        } else if t.start.is_none()
            && let Some(end) = t.end
        {
            writer
                .create_element("text")
                .with_attribute(("display", t.display.to_string().as_str()))
                .with_attribute(("file_name", t.text.as_str()))
                .with_attribute(("end", end.to_string().as_str()))
                .write_inner_content(|writer| {
                    writer
                        .create_element("words_per_page")
                        .write_text_content(BytesText::new(&t.words_per_page))?;
                    Ok(())
                })?;
        }
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

pub fn create_sequence_zip(seq: &Sequence, seq_file: &str) -> Option<Vec<u8>> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default();

    // Sequence Description
    if let Ok(xml) = seq.sequence_description.to_xml() {
        let _ = zip.start_file(seq_file, options);
        let _ = zip.write_all(xml.as_bytes());
    }

    // Glosses
    for (i, gloss) in seq.glosses.iter().enumerate() {
        if i < seq.sequence_description.gloss_names.len()
            && let Ok(xml) = gloss.to_xml()
        {
            let name = &seq.sequence_description.gloss_names[i];
            let _ = zip.start_file(name, options);
            let _ = zip.write_all(xml.as_bytes());
        }
    }

    // Texts
    for (i, text) in seq.texts.iter().enumerate() {
        if i < seq.sequence_description.texts.len()
            && let Ok(xml) = text.to_xml()
        {
            let name = &seq.sequence_description.texts[i].text;
            let _ = zip.start_file(name, options);
            let _ = zip.write_all(xml.as_bytes());
        }
    }

    zip.finish().ok().map(|c| c.into_inner())
}

pub fn from_sequence_zip(zip_data: Vec<u8>, seq_file: &str) -> Result<Sequence, GlosserError> {
    let mut archive = ZipArchive::new(Cursor::new(zip_data))
        .map_err(|e| GlosserError::Other(format!("Zip error: {}", e)))?;

    let mut seq_desc_content = String::new();
    {
        let mut file = archive.by_name(seq_file).map_err(|_| {
            GlosserError::NotFound(format!("Sequence file {} not found in zip", seq_file))
        })?;
        file.read_to_string(&mut seq_desc_content)
            .map_err(|e| GlosserError::Other(format!("Error reading sequence file: {}", e)))?;
    }

    let seq_desc = SequenceDescription::from_xml(&seq_desc_content)
        .map_err(|e| GlosserError::Other(format!("Error parsing sequence XML: {}", e)))?;

    let mut glosses = Vec::new();
    for name in &seq_desc.gloss_names {
        let mut content = String::new();
        let mut file = archive
            .by_name(name)
            .map_err(|_| GlosserError::NotFound(format!("Gloss file {} not found in zip", name)))?;
        file.read_to_string(&mut content).map_err(|e| {
            GlosserError::Other(format!("Error reading gloss file {}: {}", name, e))
        })?;
        let gloss = Glosses::from_xml(&content)
            .map_err(|e| GlosserError::Other(format!("Error parsing gloss XML {}: {}", name, e)))?;
        glosses.push(gloss);
    }

    let mut texts = Vec::new();
    for text_desc in &seq_desc.texts {
        let name = &text_desc.text;
        let mut content = String::new();
        let mut file = archive
            .by_name(name)
            .map_err(|_| GlosserError::NotFound(format!("Text file {} not found in zip", name)))?;
        file.read_to_string(&mut content)
            .map_err(|e| GlosserError::Other(format!("Error reading text file {}: {}", name, e)))?;
        let text = Text::from_xml(&content, None, None)
            .map_err(|e| GlosserError::Other(format!("Error parsing text XML {}: {}", name, e)))?;
        texts.push(text);
    }

    if texts.is_empty() || glosses.is_empty() {
        return Err(GlosserError::NotFound(String::from(
            "text or gloss not found in zip",
        )));
    }

    Ok(Sequence {
        sequence_description: seq_desc,
        glosses,
        texts,
    })
}

/*
fn update_uuids(seq: &Sequence) {
    //make gloss hash of olduuid, newuuid
    //make word hash of olduuid, newuuid
    // go through all gloss, words (word,gloss), arroweds (word,gloss)
    // updating based on appropriate hash lookup
}
*/
#[cfg(test)]
mod tests {
    use super::*;
    use exportfodt::ExportFodt;
    use exporthtml::ExportHTML;
    use exportlatex::ExportLatex;
    use exporttypst::ExportTypst;
    use texts::{AppCrit, read_text_xml, write_text_xml};

    #[cfg(feature = "morpheus")]
    use morpheus_sys::morpheus_check;

    fn get_filename_without_extension(full_path: &str) -> Option<&str> {
        use std::ffi::OsStr;
        use std::path::Path;
        let path = Path::new(full_path);
        path.file_stem() // Get the stem (filename without extension)
            .and_then(OsStr::to_str) // Convert OsStr to &str
    }

    #[test]
    fn citest_test_import() {
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

        let mut lemmatizer: HashMap<String, Uuid> = HashMap::default();
        lemmatizer.insert(
            String::from("δ"),
            Uuid::parse_str("d8a70e71-f04b-430e-98da-359a98b12931").unwrap(),
        );

        let text_struct = import::import_text(source_xml, &lemmatizer);
        assert!(text_struct.is_ok());

        let text_xml_string = text_struct.as_ref().unwrap().to_xml();
        assert!(text_xml_string.is_ok());

        //println!("text: {}", text_xml_string.unwrap());
        let r = text_struct.unwrap().words;
        assert_eq!(r.len(), 29);
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
        assert_eq!(r[28].word, "This is a test.");
        assert_eq!(r[28].word_type, WordType::Desc);
    }

    #[test]
    fn citest_test_read_write_gloss_xml_roundtrip() {
        let source_xml = r###"<glosses gloss_name="testgloss">
  <gloss uuid="f8d14d83-e5c8-4407-b3ad-d119887ea63d">
    <lemma>ψῡχρός, ψῡχρ, &apos; &lt; &gt; &quot; &amp; ψῡχρόν</lemma>
    <sort_key>ψυχροςψυχραψυχρον</sort_key>
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
    <sort_key>Νυμφη, Νυμφης, η</sort_key>
    <def>minor goddess, especially of streams, pools and fountains</def>
    <pos>noun</pos>
    <unit>0</unit>
    <note></note>
    <updated>2021-04-07 19:44:48</updated>
    <status>1</status>
    <updated_user></updated_user>
  </gloss>
</glosses>"###;
        let gloss_struct = glosses::read_gloss_xml(source_xml);

        let expected_gloss_struct = Glosses {
            gloss_name: String::from("testgloss"),
            gloss: vec![
                Gloss {
                    uuid: Uuid::parse_str("f8d14d83-e5c8-4407-b3ad-d119887ea63d").unwrap(),
                    parent_id: None,
                    lemma: String::from("ψῡχρός, ψῡχρ\u{eb00}, ' < > \" & ψῡχρόν"),
                    sort_key: String::from("ψυχροςψυχραψυχρον"),
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
                    sort_key: String::from("Νυμφη, Νυμφης, η"),
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

        let xml_string = glosses::write_gloss_xml(gloss_struct.as_ref().unwrap());

        assert_eq!(gloss_struct.unwrap(), expected_gloss_struct);
        assert_eq!(xml_string.unwrap(), source_xml);
    }

    #[test]
    fn citest_test_read_write_text_xml_roundtrip() {
        let source_xml = r###"<text text_name="ΥΠΕΡ ΤΟΥ ΕΡΑΤΟΣΘΕΝΟΥΣ ΦΟΝΟΥ ΑΠΟΛΟΓΙΑ">
  <words>
    <word uuid="46bc20ad-bb8d-486f-a61e-fa783f0d558a" type="Section">1</word>
    <word uuid="d8a70e71-f04b-430e-98da-359a98b12931" gloss_uuid="565de2e3-bf50-49b0-bf71-757ccf34080f" type="Word">Περὶ &apos; &lt; &gt; &quot; &amp;</word>
  </words>
  <appcrits>
    <appcrit word_uuid="cc402eca-165d-4af0-9514-4c57aee17bb7">1.4 ἀγανακτήσειε Η; οὐκ ἀγανακτείση P$^1$ -οίη P$^c$</appcrit>
    <appcrit word_uuid="8680e45e-f6e0-4c9d-aed4-d0deb9470b4f">2.1 ἡγοῖσθε (OCT, Carey); ἡγεῖσθαι P</appcrit>
  </appcrits>
</text>"###;
        let text_struct = read_text_xml(source_xml, None, None);

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
            // words_per_page: String::from(
            //     "154, 151, 137, 72, 121, 63, 85, 107, 114, 142, 109, 79, 82, 81, 122, 99, 86, 110, 112, 151, 140, 99, 71, 117, 114, 1",
            // ),
        };

        let xml_string = write_text_xml(text_struct.as_ref().unwrap());

        assert_eq!(text_struct.unwrap(), expected_text_struct);
        assert_eq!(xml_string.unwrap(), source_xml);
    }

    #[test]
    fn citest_test_read_write_text_xml_start_end() {
        let source_xml = r###"<text text_name="ΥΠΕΡ ΤΟΥ ΕΡΑΤΟΣΘΕΝΟΥΣ ΦΟΝΟΥ ΑΠΟΛΟΓΙΑ">
  <words>
    <word uuid="46bc20ad-bb8d-486f-a61e-fa783f0d558a" type="Section">1</word>
    <word uuid="d8a70e71-f04b-430e-98da-359a98b12931" gloss_uuid="565de2e3-bf50-49b0-bf71-757ccf34080f" type="Word">Περὶ</word>
    <word uuid="a8517e23-fc53-4d76-8d0d-09a3e3f571eb" gloss_uuid="836f72d8-c327-4587-a194-5e6b4dba33cc" type="Word">πολλοῦ</word>
    <word uuid="ad7cc3b1-aa7e-49a8-b1b6-034e8bcb318f" gloss_uuid="cdb183fe-4fa0-48e5-bcff-e00a2e41f8b8" type="Word">ἂν</word>
    <word uuid="1eba6d85-41e9-4fee-b76d-2301908a76b7" gloss_uuid="bb5502e9-a181-4633-abdd-f74c8dcd8360" type="Word">ποιησαίμην</word>
  </words>
  <appcrits>
    <appcrit word_uuid="cc402eca-165d-4af0-9514-4c57aee17bb7">1.4 ἀγανακτήσειε Η; οὐκ ἀγανακτείση P$^1$ -οίη P$^c$</appcrit>
    <appcrit word_uuid="8680e45e-f6e0-4c9d-aed4-d0deb9470b4f">2.1 ἡγοῖσθε (OCT, Carey); ἡγεῖσθαι P</appcrit>
  </appcrits>
</text>"###;
        let text_struct = read_text_xml(
            source_xml,
            Some(Uuid::parse_str("a8517e23-fc53-4d76-8d0d-09a3e3f571eb").unwrap()),
            Some(Uuid::parse_str("ad7cc3b1-aa7e-49a8-b1b6-034e8bcb318f").unwrap()),
        );

        let expected_text_struct = Text {
            text_name: String::from("ΥΠΕΡ ΤΟΥ ΕΡΑΤΟΣΘΕΝΟΥΣ ΦΟΝΟΥ ΑΠΟΛΟΓΙΑ"),
            words: vec![
                Word {
                    uuid: Uuid::parse_str("a8517e23-fc53-4d76-8d0d-09a3e3f571eb").unwrap(),
                    gloss_uuid: Some(
                        Uuid::parse_str("836f72d8-c327-4587-a194-5e6b4dba33cc").unwrap(),
                    ),
                    word_type: WordType::Word,
                    word: String::from("πολλοῦ"),
                },
                Word {
                    uuid: Uuid::parse_str("ad7cc3b1-aa7e-49a8-b1b6-034e8bcb318f").unwrap(),
                    gloss_uuid: Some(
                        Uuid::parse_str("cdb183fe-4fa0-48e5-bcff-e00a2e41f8b8").unwrap(),
                    ),
                    word_type: WordType::Word,
                    word: String::from("ἂν"),
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
        };
        assert_eq!(text_struct.unwrap(), expected_text_struct);
    }

    #[test]
    fn citest_test_read_write_seq_desc_xml_roundtrip() {
        let source_xml = r###"<sequence_description>
  <name>LGI - UPPER LEVEL GREEK &apos; &lt; &gt; &quot; &amp;</name>
  <start_page>24</start_page>
  <glosses>
    <gloss_name>glosses.xml</gloss_name>
  </glosses>
  <texts>
    <text display="false" file_name="hq.xml">
      <words_per_page>100, 200</words_per_page>
    </text>
    <text display="false" file_name="ion.xml">
      <words_per_page>300, 400</words_per_page>
    </text>
    <text display="true" file_name="ajax.xml">
      <words_per_page>500, 600</words_per_page>
    </text>
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
                    text: String::from("hq.xml"),
                    words_per_page: String::from("100, 200"),
                    start: None,
                    end: None,
                },
                TextDescription {
                    display: false,
                    text: String::from("ion.xml"),
                    words_per_page: String::from("300, 400"),
                    start: None,
                    end: None,
                },
                TextDescription {
                    display: true,
                    text: String::from("ajax.xml"),
                    words_per_page: String::from("500, 600"),
                    start: None,
                    end: None,
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
    fn citest_test_data() {
        let glosses = vec![
            Gloss {
                uuid: Uuid::parse_str("67e55044-10b1-426f-9247-bb680e5fe0c8").unwrap(),
                parent_id: None,
                lemma: String::from("ἄγω"),
                sort_key: String::from("αγω"),
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
                sort_key: String::from("γαμεω"),
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
                sort_key: String::from("βλαπτω"),
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
                    words_per_page: String::from(""),
                    start: None,
                    end: None,
                },
                TextDescription {
                    display: true,
                    text: String::from("def.xml"),
                    words_per_page: String::from(""),
                    start: None,
                    end: None,
                },
            ],
        };

        let mut glosses_hash = HashMap::default();
        for g in &glosses {
            glosses_hash.insert(g.uuid, g);
        }

        let mut arrowed_words_hash = HashMap::default();
        for s in sequence.arrowed_words.clone() {
            arrowed_words_hash.insert(s.word_uuid, s.gloss_uuid);
        }

        let text = Text {
            text_name: String::from(""),
            words,
            appcrits: Some(vec![]),
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
    fn citest_test_data_dup_arrowed_word() {
        let glosses = vec![
            Gloss {
                uuid: Uuid::parse_str("67e55044-10b1-426f-9247-bb680e5fe0c8").unwrap(),
                parent_id: None,
                lemma: String::from("ἄγω"),
                sort_key: String::from("αγω"),
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
                sort_key: String::from("γαμεω"),
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
                sort_key: String::from("βλαπτω"),
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
                    words_per_page: String::from(""),
                    start: None,
                    end: None,
                },
                TextDescription {
                    display: true,
                    text: String::from("def.xml"),
                    words_per_page: String::from(""),
                    start: None,
                    end: None,
                },
            ],
        };

        let mut glosses_hash = HashMap::default();
        for g in &glosses {
            glosses_hash.insert(g.uuid, g);
        }

        let mut arrowed_words_hash = HashMap::default();
        for s in sequence.arrowed_words.clone() {
            arrowed_words_hash.insert(s.word_uuid, s.gloss_uuid);
        }

        let text = Text {
            text_name: String::from(""),
            words,
            appcrits: Some(vec![]),
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

    /*********** */
    //these tests read or write local files

    #[test]
    fn test_width() {
        //let s = "νόμοι κελεύουσι τοὺς τὰ τοιαῦτα πράττοντας, οὐκ εἰσαρπασθεὶς ἐκ τῆς ὁδοῦ, οὐδ’ ἐπὶ τὴν ἑστίαν";
        //let s = "πλήττω, -πλήξω, -ἐπλήξα, πέπληγα, πέπληγμαι,";
        let s = "ward off (+ acc.) from (someone, in dat.);";
        let w = lines::get_width(s);
        println!("width: {}", w);
    }

    #[cfg(feature = "morpheus")]
    #[test]
    fn morpheus_check_word() {
        let input = String::from("μῆνιν ἄειδε θεὰ Πηληϊάδεω Ἀχιλῆος");
        let output = String::from("mh=nin a)/eide qea\\ *phlhi+a/dew *a)xilh=os");
        let result = betacode::converter::revert(input);
        assert_eq!(result, output);

        let input = String::from("φέρω");
        let my_string = betacode::converter::revert(input);
        //let my_string = "fe/rw";

        //let morphlib_path = None; //or e.g.: Some("morpheus/dist/stemlib");
        let morphlib_path = Some("../morpheus-sys/morpheus/dist/stemlib");
        let res = morpheus_check(&my_string, morphlib_path);

        assert_eq!(
            res.unwrap(),
            String::from(
                r##"<word>
<form xml:lang="grc-x-beta">fe/rw</form>
<entry>
<dict>
<hdwd xml:lang="grc-x-beta">fe/rw</hdwd>
<pofs order="1">verb</pofs>
</dict>
<infl>
<term xml:lang="grc-x-beta"><stem>fer</stem><suff>w</suff></term>
<pofs order="1">verb</pofs>
<mood>subjunctive</mood>
<num>singular</num>
<pers>1st</pers>
<tense>present</tense>
<voice>active</voice>
<stemtype>w_stem</stemtype>
</infl>
<infl>
<term xml:lang="grc-x-beta"><stem>fer</stem><suff>w</suff></term>
<pofs order="1">verb</pofs>
<mood>indicative</mood>
<num>singular</num>
<pers>1st</pers>
<tense>present</tense>
<voice>active</voice>
<stemtype>w_stem</stemtype>
</infl>
</entry>
</word>
</words>
"##
            )
        );
    }

    #[test]
    fn save_xml() {
        let seq = Sequence::from_xml("../gkvocab_data/testsequence.xml");
        assert!(seq.is_ok());

        let res = seq.unwrap().to_xml("../gkvocab_data2", "testsequence2.xml");
        assert!(res.is_ok());
    }

    #[test]
    fn get_glosses() {
        let seq = Sequence::from_xml("../gkvocab_data/testsequence.xml");
        assert!(seq.is_ok());
        let r = seq.unwrap().get_glosses("βε", 3);
        assert!(!r.0.is_empty());
        println!("res {:?}", r);
    }

    //this test requires local files and writes an FODT document of the HQ glosses
    #[test]
    fn make_hq_gloss_document() {
        let gloss_path = "../gkvocab_data/glosses.xml";
        let output_path = "../gkvocab_data/justhq.fodt";
        let page_number = 1;
        let export = ExportFodt {};
        let mut doc = export.document_start("", page_number);

        let mut hq_glosses_vec: Vec<Gloss> = Vec::new();
        let fake_word = Word::default();
        if let Ok(contents) = fs::read_to_string(gloss_path)
            && let Ok(gloss) = Glosses::from_xml(&contents)
        {
            for g in gloss.gloss {
                if g.status == 1 && g.unit > 0 && g.unit < 21 {
                    hq_glosses_vec.push(g.clone());
                }
            }
            for unit in 1..=20 {
                let mut gloss_ocurrances: Vec<GlossOccurrance> = Vec::new();
                for g in &hq_glosses_vec {
                    if g.unit == unit {
                        gloss_ocurrances.push(GlossOccurrance {
                            word: &fake_word,
                            gloss: Some(g),
                            arrowed_state: ArrowedState::Visible,
                            running_count: None,
                            total_count: None,
                        });
                    }
                }
                gloss_ocurrances.sort_by(|a, b| {
                    a.gloss
                        .as_ref()
                        .unwrap()
                        .sort_key
                        .to_lowercase()
                        .cmp(&b.gloss.as_ref().unwrap().sort_key.to_lowercase())
                });
                doc.push_str(
                    format!(
                        r##"
        <text:p text:style-name="P7">Unit {}</text:p>
"##,
                        unit
                    )
                    .as_str(),
                );
                doc.push_str(&export.page_gloss_start());
                doc.push_str(get_gloss_string(&gloss_ocurrances, &export).as_str());
                doc.push_str(
                    r##"
        </table:table>
        <text:p text:style-name="P7"></text:p>
        <text:p text:style-name="P7"></text:p>"##,
                );
            }
            doc.push_str(export.document_end().as_str());
            //allow tables to break between rows
            doc = doc.replace(
                r##"style:may-break-between-rows="false""##,
                r##"style:may-break-between-rows="true""##,
            );
            fs::write(output_path, &doc).unwrap();
        }
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
            sort_key: false,
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
            sort_key: true,
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
    fn save_typst_document_from_file() {
        let seq = Sequence::from_xml("../gkvocab_data/testsequence.xml");
        assert!(seq.is_ok());

        let gloss_occurrances = seq.as_ref().unwrap().process();
        assert!(gloss_occurrances.is_ok());

        let options = GlossPageOptions {
            filter_unique: true,
            filter_invisible: true,
            sort_key: true,
        };

        let doc = seq.as_ref().unwrap().make_document(
            &gloss_occurrances.unwrap(),
            &ExportTypst {},
            &options,
        );
        let output_path = "../gkvocab_data/ulgv3.typ";
        let _ = fs::write(output_path, &doc);
    }

    #[test]
    fn save_fodt_document_from_file() {
        let seq = Sequence::from_xml("../gkvocab_data/testsequence.xml");
        assert!(seq.is_ok());

        let gloss_occurrances = seq.as_ref().unwrap().process();
        assert!(gloss_occurrances.is_ok());

        let options = GlossPageOptions {
            filter_unique: true,
            filter_invisible: true,
            sort_key: true,
        };

        let doc = seq.as_ref().unwrap().make_document(
            &gloss_occurrances.unwrap(),
            &ExportFodt {},
            &options,
        );
        let output_path = "../gkvocab_data/ulgv3.fodt";
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
            sort_key: false,
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
    fn get_word_counts_per_page() {
        let seq = Sequence::from_xml("../gkvocab_data/testsequence.xml");
        assert!(seq.is_ok());

        let gloss_occurrances = seq.as_ref().unwrap().process();
        assert!(gloss_occurrances.is_ok());
        //println!("{:?}", gloss_occurrances.as_ref().unwrap()[3]);
        let words = lines::count_lines(&gloss_occurrances.unwrap()[3]);
        println!("{:?}", words);
    }

    #[test]
    fn local_import_text() {
        let input_path =
            "/Users/jeremy/Documents/aaanewsurveyxml/prose/2_Herodotus_1.30.1.4-32.2.2.xml"; //1_Anaxagoras_Fragment_12.xml";
        let output_path = "/Users/jeremy/Documents/aaanewsurveyxml/prose/2_Herodotus_1.30.1.4-32.2.2-processed.xml"; //1_Anaxagoras_Fragment_12-processsed.xml";
        let seq = Sequence::from_xml("../gkvocab_data/testsequence.xml").unwrap();
        let lemmatizer: HashMap<String, GlossUuid> = import::build_lemmatizer(&seq);

        let source_xml = fs::read_to_string(input_path).unwrap();
        let text_struct = import::import_text(&source_xml, &lemmatizer).unwrap();
        let xml = text_struct.to_xml().unwrap();
        fs::write(output_path, &xml).unwrap();
    }

    #[test]
    fn local_import_dir() {
        let seq = Sequence::from_xml("../gkvocab_data/testsequence.xml").unwrap();
        let lemmatizer: HashMap<String, GlossUuid> = import::build_lemmatizer(&seq);

        let input_directory = "/Users/jeremy/Documents/aaanewsurveyxml/poetry";
        let output_directory = format!("{}/output", input_directory);

        let entries = fs::read_dir(input_directory).expect("Failed to read directory");

        for entry in entries {
            let entry = entry.expect("Failed to read directory entry");
            let path = entry.path();

            if path.is_file() && path.extension().is_some_and(|ext| ext == "xml") {
                match fs::read_to_string(&path) {
                    Ok(content) => {
                        let text_struct = import::import_text(&content, &lemmatizer).unwrap();
                        let xml = text_struct.to_xml().unwrap();

                        if let Some(path_str) = path.to_str()
                            && let Some(file_name) = get_filename_without_extension(path_str)
                        {
                            let output_path =
                                format!("{}/{}-processed.xml", output_directory, file_name);
                            println!("write to: {}", output_path);
                            fs::write(output_path, &xml).unwrap();
                        }
                    }
                    Err(e) => {
                        eprintln!("Error reading file {:?}: {}", path, e);
                    }
                }
            }
        }
    }

    #[test]
    fn citest_test_zip_roundtrip() {
        let seq_desc = SequenceDescription {
            name: String::from("Test Sequence"),
            start_page: 1,
            gloss_names: vec![String::from("gloss1.xml")],
            texts: vec![TextDescription {
                display: true,
                text: String::from("text1.xml"),
                words_per_page: String::from("10"),
                start: None,
                end: None,
            }],
            arrowed_words: vec![],
        };

        let glosses = vec![Glosses {
            gloss_name: String::from("Gloss 1"),
            gloss: vec![Gloss {
                uuid: Uuid::new_v4(),
                lemma: String::from("lemma"),
                def: String::from("definition"),
                ..Default::default()
            }],
        }];

        let texts = vec![Text {
            text_name: String::from("Text 1"),
            words: vec![Word {
                uuid: Uuid::new_v4(),
                word: String::from("word"),
                word_type: WordType::Word,
                ..Default::default()
            }],
            appcrits: None,
        }];

        let sequence = Sequence {
            sequence_description: seq_desc,
            glosses,
            texts,
        };

        let zip_filename = "sequence.xml";
        let zip_data = create_sequence_zip(&sequence, zip_filename).expect("Failed to create zip");
        let restored_sequence =
            from_sequence_zip(zip_data, zip_filename).expect("Failed to read zip");

        assert_eq!(sequence, restored_sequence);
    }
}
