use crate::{GlossUuid, WordUuid, get_entity};
use quick_xml::Reader;
use quick_xml::events::Event;
use quick_xml::name::QName;
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

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
pub struct AppCrit {
    pub word_uuid: WordUuid,
    pub entry: String,
}

#[derive(Default, Clone, Debug, PartialEq)]
pub struct Word {
    pub uuid: WordUuid,
    pub gloss_uuid: Option<GlossUuid>,
    pub word_type: WordType,
    pub word: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    pub text_name: String,
    pub words: Vec<Word>,
    pub appcrits: Option<Vec<AppCrit>>,
}

impl Text {
    pub fn to_xml(&self) -> Result<String, quick_xml::Error> {
        write_text_xml(self)
    }

    pub fn from_xml(
        s: &str,
        start: Option<WordUuid>,
        end: Option<WordUuid>,
    ) -> Result<Text, quick_xml::Error> {
        read_text_xml(s, start, end)
    }
}

pub fn read_text_xml(
    xml: &str,
    start: Option<WordUuid>,
    end: Option<WordUuid>,
) -> Result<Text, quick_xml::Error> {
    let mut res: Vec<Word> = vec![];
    let mut appcrits: Vec<AppCrit> = vec![];
    let mut reader = Reader::from_str(xml);
    reader.config_mut(); //.trim_text(true); // Trim whitespace from text nodes
    //reader.config_mut().trim_text(true); //we don't want this since it trims spaces around entities e.g. &lt;
    reader.config_mut().enable_all_checks(true);
    reader.config_mut().expand_empty_elements = true;

    let mut found_start = start.is_none(); //if none, consider the start already found to start at the first word
    let mut found_end = false;
    let mut buf = Vec::new();

    let mut current_word: Word = Default::default();
    let mut current_appcrit: AppCrit = Default::default();
    let mut text_name = String::from("");

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
                        _ => (), //println!("unknown tag: {}", this_tag),
                    }
                }
            }
            Ok(Event::End(e)) => {
                tags.pop();
                if b"word" == e.name().as_ref() {
                    if !found_start && start.is_some() && current_word.uuid == start.unwrap() {
                        found_start = true;
                    }
                    //don't push before we find start or after we find end
                    if found_start && !found_end {
                        res.push(current_word.clone());
                    }
                    //check for found end after pushing, so the last word is pushed
                    if !found_end && end.is_some() && current_word.uuid == end.unwrap() {
                        found_end = true;
                    }
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
    })
}

pub fn write_text_xml(text: &Text) -> Result<String, quick_xml::Error> {
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

    writer.write_event(Event::End(BytesEnd::new("text")))?;

    let result = writer.into_inner().into_inner();
    Ok(std::str::from_utf8(&result).unwrap().to_string())
}
