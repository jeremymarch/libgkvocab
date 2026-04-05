use crate::{GlossUuid, Sequence, Text, Word, WordType, get_entity, sanitize_greek};
use quick_xml::Reader;
use quick_xml::events::Event;
use quick_xml::name::QName;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[cfg(feature = "morpheus")]
use morpheus_sys::morpheus_check;

#[cfg(feature = "morpheus")]
pub fn morpheus_check_unicode(input: &str, morphlib_path: Option<&str>) -> Option<String> {
    let my_string = betacode::converter::revert(input);
    //let morphlib_path = None; //or e.g.: Some("morpheus/dist/stemlib");
    //let morphlib_path = Some("../morpheus-sys/morpheus/dist/stemlib");
    morpheus_check(&my_string, morphlib_path)
}

// // Read an ICU4X data blob statically:
// static ICU_PROVIDER: &[u8] = include_bytes!("../greek_collation_blob.postcard");
// use icu::properties::{CanonicalCombiningClass, props::CanonicalCombiningClassV1Marker};
// use icu_provider::prelude::*;

// // Initialize the provider once (e.g., using once_cell or lazy_static)
// pub fn get_provider()
// -> Arc<impl DataProvider<icu_properties::provider::CanonicalCombiningClassV1Marker>> {
//     let provider = BlobDataProvider::try_new_from_static_blob(ICU_PROVIDER)
//         .expect("Failed to create provider from static blob");
//     Arc::new(provider)
// }
// fn is_combining_char(
//     c: char,
//     provider: &impl DataProvider<CanonicalCombiningClassV1Marker>,
// ) -> bool {
//     // Load the data for Canonical Combining Class
//     let data = provider
//         .load(DataRequest::default())
//         .expect("Failed to load data")
//         .payload;

//     // Get the class for the specific character
//     let ccc = data.get().get_ccc(c);

//     // If CCC is not 0 (NotReordered), it is a combining character
//     ccc != CanonicalCombiningClass::NotReordered
// }

fn is_combining_mark(c: char) -> bool {
    // let provider = get_provider();
    // is_combining_char(c, &*provider)

    unicode_normalization::char::is_combining_mark(c)
}

fn split_words(text: &str, lemmatizer: &HashMap<String, Uuid>) -> Vec<Word> {
    let mut words: Vec<Word> = vec![];
    let mut last = 0;

    for (index, matched) in
        text.match_indices(|c: char| !(c.is_alphanumeric() || c == '\'' || is_combining_mark(c)))
    {
        //add words
        if last != index && &text[last..index] != " " {
            let gloss_uuid = lemmatizer.get(&text[last..index]).copied();
            words.push(Word {
                uuid: Uuid::new_v4(),
                word: text[last..index].to_string(),
                word_type: WordType::Word,
                gloss_uuid,
            });
        }
        //add word separators
        if matched != " " && matched != "\n" && matched != "\t" {
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
            word_type: WordType::Word,
            gloss_uuid,
        });
    }

    words
}

//builds a lemmatizer of all previous word/gloss pairs which do not have collisions
pub fn build_lemmatizer(seq: &Sequence) -> HashMap<String, GlossUuid> {
    let mut lemmatizer: HashMap<String, GlossUuid> = HashMap::default();
    let mut duplicates: HashSet<GlossUuid> = HashSet::default();
    //let seq = Sequence::from_xml("");
    for t in &seq.texts {
        for w in &t.words {
            //get,
            // if not exist, insert
            // if exist and same, do nothing
            // if exist and different gloss id, remove original and print the conflict
            //
            if let Some(g) = w.gloss_uuid {
                if let Some(_r) = duplicates.get(&g) {
                    //println!("Another duplicate of {} {} {}", g, w.word, w.uuid);
                    continue;
                }
                if let Some(r) = lemmatizer.get(&w.word) {
                    if *r != g {
                        duplicates.insert(g);
                        //println!("Duplicate of {} {} {}", g, w.word, w.uuid);
                    } else {
                        continue;
                    }
                } else {
                    lemmatizer.insert(w.word.clone(), g);
                }
            }
        }
    }
    lemmatizer
}

pub fn import_text(
    xml_string: &str,
    lemmatizer: &HashMap<String, GlossUuid>,
) -> Result<Text, quick_xml::Error> {
    let mut words: Vec<Word> = Vec::new();

    let mut reader = Reader::from_str(xml_string);
    reader.config_mut().trim_text(true); //FIX ME: check docs, do we want true here?
    reader.config_mut().enable_all_checks(true);

    let mut buf = Vec::new();

    let mut found_tei = false;

    let mut in_text = false;

    let mut in_speaker = false;
    let mut in_head = false;
    let mut in_desc = false;
    let mut speaker = String::from("");
    let mut head = String::from("");
    let mut desc = String::from("");

    let mut work_title = String::from("");

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
                                let reference = if let Some(chap_unwrapped) = &chapter_value {
                                    Some(format!("{}.{}", chap_unwrapped, n_unwraped))
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
                    words.extend_from_slice(&split_words(&clean_string, lemmatizer)[..]);
                }
            }
            // unescape and decode the text event using the reader encoding
            Ok(Event::Text(ref e)) => {
                if let Ok(s) = e.decode() {
                    if in_desc {
                        desc.push_str(&s);
                    } else if in_speaker {
                        speaker.push_str(&s);
                    } else if in_head {
                        head.push_str(&s);
                    } else if in_text {
                        //let seperator = Regex::new(r"([ ,.;]+)").expect("Invalid regex");
                        let clean_string = sanitize_greek(&s);
                        words.extend_from_slice(&split_words(&clean_string, lemmatizer)[..]);
                    }
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
                    words.push(Word {
                        uuid: Uuid::new_v4(),
                        word: speaker,
                        word_type: WordType::Speaker,
                        gloss_uuid: None,
                    });
                    speaker = String::from("");
                } else if b"head" == e.name().as_ref() {
                    in_head = false;
                    work_title = head.to_owned();
                    words.push(Word {
                        uuid: Uuid::new_v4(),
                        word: head,
                        word_type: WordType::WorkTitle,
                        gloss_uuid: None,
                    });
                    head = String::from("");
                } else if b"desc" == e.name().as_ref() {
                    in_desc = false;
                    words.push(Word {
                        uuid: Uuid::new_v4(),
                        word: desc,
                        word_type: WordType::Desc,
                        gloss_uuid: None,
                    });
                    desc = String::from("");
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
        text_name: work_title,
        appcrits: None,
        words,
    })
}
