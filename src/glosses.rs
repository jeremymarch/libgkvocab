use crate::{GlossUuid, get_entity};
use quick_xml::Reader;
use quick_xml::events::Event;
use quick_xml::name::QName;
use uuid::Uuid;

#[derive(Default, Clone, Debug, PartialEq)]
pub struct Gloss {
    pub uuid: GlossUuid,
    pub parent_id: Option<GlossUuid>,
    pub lemma: String,
    pub sort_key: String,
    pub def: String,
    pub pos: String,
    pub unit: i32,
    pub note: String,
    pub updated: String,
    pub status: i32,
    pub updated_user: String,
    //pub origin: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Glosses {
    pub gloss_name: String,
    pub gloss: Vec<Gloss>,
}

impl Glosses {
    pub fn to_xml(&self) -> Result<String, quick_xml::Error> {
        write_gloss_xml(self)
    }

    pub fn from_xml(s: &str) -> Result<Glosses, quick_xml::Error> {
        read_gloss_xml(s)
    }
}

pub fn read_gloss_xml(xml: &str) -> Result<Glosses, quick_xml::Error> {
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
                        "sort_key" => current_gloss.sort_key.push_str(text),
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
                        "sort_key" => current_gloss.sort_key.push_str(&text),
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

pub fn write_gloss_xml(gloss: &Glosses) -> Result<String, quick_xml::Error> {
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
                    .create_element("sort_key")
                    .write_text_content(BytesText::new(&g.sort_key))?;
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
