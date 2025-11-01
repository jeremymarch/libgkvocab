use super::ExportDocument;
use crate::ArrowedWordsIndex;
use crate::GlossOccurrance;
use crate::WordType;
use crate::WordUuid;
use regex::Regex;
use std::collections::HashMap;

fn complete_verse_line(
    verse_speaker: Option<String>,
    verse_line: &str,
    verse_line_number: &str,
) -> String {
    format!(
        "<div class='VerseLine'><div class='VerseSpeaker'>{}</div><div class='VerseText'>{}</div><div class='VerseLineNumber'>{}</div></div>\n",
        verse_speaker.as_ref().unwrap_or(&String::from("")),
        &verse_line,
        if let Ok(i) = verse_line_number.parse::<i32>() {
            if i % 5 == 0 { verse_line_number } else { "" }
        } else {
            &verse_line_number
        }
    )
}

pub struct ExportHTML {}
impl ExportDocument for ExportHTML {
    fn gloss_entry(
        &self,
        gloss_occurrance: &GlossOccurrance,
        lemma: &str,
        gloss: &str,
        arrowed: bool,
    ) -> String {
        let word_id = gloss_occurrance.word.uuid;
        let gloss_id = gloss_occurrance.gloss.unwrap().uuid;
        let pos = &gloss_occurrance.gloss.unwrap().pos;
        let running_count = gloss_occurrance.running_count.unwrap();
        let total_count = gloss_occurrance.total_count.unwrap();
        let arrowed_here = if arrowed { "arrowedHere" } else { "" };
        format!(
            r###"
<div id="word{word_id}" lemmaid="{gloss_id}" class="listword hqListWord {arrowed_here}" textseq="1" arrowedtextseq="1">
    <div id="arrow{word_id}" class="listarrow"></div>
    <div class="clickablelistword">
        <span class="listheadword" id="listheadword{word_id}">{lemma}</span>.
        &nbsp;&nbsp;<span class="listposwrapper">
            (<span class="listpos" id="listpos{word_id}">{pos}</span>)
        </span>
        <span class="listdef" id="listdef{word_id}">{gloss}</span>
        <a class="listfrequency" href="javascript:showGlossOccurrencesList({gloss_id})">({running_count} of {total_count})</a>
    </div>
</div>
"###
        )
    }

    fn make_text(
        &self,
        gloss_occurrances: &[GlossOccurrance],
        appcrit_hash: &HashMap<WordUuid, String>,
    ) -> String {
        let re = Regex::new("([0-9]+)[.]([0-9]+)").unwrap();
        let mut res = String::from("");
        let mut prev_non_space = true;
        //let mut last_type = WordType::InvalidType;
        let mut is_verse_section = false;
        let mut verse_speaker: Option<String> = None;
        let mut verse_line = String::from("");
        let mut verse_line_number = String::from("");

        let mut appcrits_page: Vec<String> = vec![];

        //println!("page count {}", gloss_occurrances.len());
        for w in gloss_occurrances {
            if let Some(ap) = appcrit_hash.get(&w.word.uuid) {
                appcrits_page.push(ap.clone());
            }
            //println!("word type {:?}", w.word.word_type);

            match w.word.word_type {
                WordType::VerseLine => {
                    if !is_verse_section {
                        //res.push_str(r##"<div class="VerseLine">"##);
                        is_verse_section = true;
                    } else {
                        //previous verse line is complete
                        res.push_str(
                            complete_verse_line(verse_speaker, &verse_line, &verse_line_number)
                                .as_str(),
                        );
                        verse_speaker = None;
                        verse_line = String::from("");
                    }
                    verse_line_number = w.word.word.replace("[line]", "");
                }
                WordType::WorkTitle => res
                    .push_str(format!("<div class='WorkTitle'>{}</div>\n", &w.word.word).as_str()),
                WordType::Word | WordType::Punctuation => {
                    //0 | 1
                    let punc = vec![
                        ".", ",", "·", "·", ";", ";", ">", "]", ")", ",\"", ".”", ".\"", "·\"",
                        "·\"", ".’",
                    ];
                    let s = format!(
                        "{}{}",
                        if punc.contains(&w.word.word.as_str()) || prev_non_space {
                            ""
                        } else {
                            " "
                        },
                        w.word.word
                    );
                    let this_word = format!(
                        "<span id='word{}' class='textword'>{}</span> \n",
                        w.word.uuid, s
                    );
                    if is_verse_section {
                        verse_line.push_str(&this_word);
                    } else {
                        res.push_str(&this_word);
                    }
                    prev_non_space = w.word.word == "<" || w.word.word == "[" || w.word.word == "(";
                }
                WordType::ParaWithIndent => res.push_str("\n\\par\n"),
                WordType::ParaNoIndent => res.push_str("\n\\noindent\n"),
                WordType::Section => {
                    let section_input = w.word.word.replace("[section]", "");

                    let matches = re.captures(&section_input);

                    let s = if let Some(matches) = matches {
                        let section = matches.get(1).unwrap().as_str();
                        let subsection = matches.get(2).unwrap().as_str();

                        //To Do: for the next thee formats move space to start of line
                        if subsection == "1" {
                            format!("<span class='Section'>{}</span> ", section)
                        } else {
                            format!("<span class='SubSection'>{}</span> ", subsection)
                        }
                    } else {
                        format!("<span class='Section'>{}</span> ", section_input)
                    };

                    res.push_str(s.as_str());
                    //if last_type == WordType::InvalidType || last_type == WordType::ParaWithIndent {
                    //-1 || 6
                    prev_non_space = true;
                    // } else {
                    //     prev_non_space = false;
                    // }
                }
                WordType::Speaker => {
                    let s = format!("<span class='Speaker'>{}</span> ", w.word.word);
                    res.push_str(s.as_str());
                }
                WordType::InlineSpeaker => {
                    if is_verse_section {
                        verse_speaker = Some(w.word.word.clone());
                    } else {
                        res.push_str(
                            format!("<span class='InlineSpeaker'>{}</span> ", w.word.word).as_str(),
                        );
                    }
                }
                _ => (),
            }
            //last_type = w.word_type.clone();
        }

        if is_verse_section {
            //previous verse line is complete
            res.push_str(
                complete_verse_line(verse_speaker, &verse_line, &verse_line_number).as_str(),
            );

            res.push_str("<br>\n");
        } else {
            res.push_str("<br><br><br>\n");
        }

        if !appcrits_page.is_empty() {
            res.push_str("<br>\n");
        }
        for ap in appcrits_page {
            res.push_str(format!("<div class='appcrit'>{}</div>\n", &ap).as_str());
        }
        res
    }

    fn page_gloss_start(&self) -> String {
        String::from("<div class='gloss-table'>\n")
    }

    fn page_start(&self, title: &str) -> String {
        format!("\n<!--PAGE START--><div>{title}</div>\n")
    }

    fn page_end(&self) -> String {
        String::from("\n<!--END PAGE--><br><br>\n")
    }

    fn document_end(&self) -> String {
        String::from("\n<BR><BR><!--END DOCUMENT--></body></html>\n")
    }

    fn document_start(&self, title: &str, start_page: usize) -> String {
        let start = String::from("<html><body>");
        start
        // start
        //     .replace("%MAIN_TITLE%", title)
        //     .replace("%PAGE_NUM%", start_page.to_string().as_str())
    }

    fn make_index(&self, arrowed_words_index: &[ArrowedWordsIndex]) -> String {
        String::from("\n<!--INDEX-->\n")
    }

    fn blank_page(&self) -> String {
        String::from("\n<!--BLANK PAGE-->\n")
    }
}
