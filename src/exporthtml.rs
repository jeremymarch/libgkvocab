use super::ExportDocument;
use crate::ArrowedState;
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
    fn gloss_entry(&self, gloss_occurrance: &GlossOccurrance, lemma: Option<&str>) -> String {
        let mut gloss_id = String::from("");
        let mut pos = String::from("");
        let mut def = String::from("");
        let mut is_glossed = String::from("");
        if let Some(gloss) = gloss_occurrance.gloss {
            gloss_id = gloss.uuid.to_string();
            pos = gloss.pos.clone();
            def = gloss.def.clone();
            is_glossed = String::from("hqListWord");
        }

        let real_lemma = if let Some(my_lemma) = lemma {
            my_lemma.to_string()
        } else {
            gloss_occurrance.word.word.to_string()
        };
        let word_id = gloss_occurrance.word.uuid;
        let running_count = gloss_occurrance.running_count.unwrap_or(0);
        let total_count = gloss_occurrance.total_count.unwrap_or(0);
        let arrowed_state_class = match gloss_occurrance.arrowed_state {
            ArrowedState::Arrowed => "arrowedHere",
            ArrowedState::Invisible => "alreadyArrowed",
            _ => "",
        };
        format!(
            r###"
<div id="gloss-word-{word_id}" lemmaid="{gloss_id}" class="listword {is_glossed} {arrowed_state_class}" textseq="1" arrowedtextseq="1">
    <div id="arrow{word_id}" class="listarrow"></div>
    <div class="glossHangingIndentDiv">
        <span class="realClickableGloss">
            <span class="listheadword" id="listheadword{word_id}">{real_lemma}</span>.
            &nbsp;&nbsp;<span class="listposwrapper">
                (<span class="listpos" id="listpos{word_id}">{pos}</span>)
            </span>
            <span class="listdef" id="listdef{word_id}">{def}</span>
        </span>
        <span class="listfrequency" id="gloss-freq-{gloss_id}">({running_count}&nbsp;of&nbsp;{total_count})</span>
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

        let mut para_open = false;
        let mut section_open = false;

        res.push_str("<div class=\"TextContainer\">\n<div class=\"TextContainerInner\">\n");

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
                    .push_str(format!("<div class='TextTitle'>{}</div>\n", &w.word.word).as_str()),
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
                        "<span id='text-word-{}' class='textword'>{}</span>",
                        w.word.uuid, s
                    );
                    if is_verse_section {
                        verse_line.push_str(&this_word);
                    } else {
                        res.push_str(&this_word);
                    }
                    prev_non_space = w.word.word == "<" || w.word.word == "[" || w.word.word == "(";
                }
                WordType::ParaWithIndent => {
                    if para_open {
                        res.push_str("\n</div><!--Close ParaIndented-->\n");
                    }
                    para_open = true;
                    res.push_str("\n<div class='ParaIndented'>\n");
                }
                WordType::ParaNoIndent => {
                    if para_open {
                        res.push_str("\n</div><!--Close ParaNotIndented-->\n");
                    }
                    para_open = true;
                    res.push_str("\n<div class='ParaNotIndented'>\n");
                }
                WordType::Section => {
                    if section_open {
                        //res.push_str("\n</span><!--Close Section-->\n");
                        section_open = false;
                    }
                    let section_input = w.word.word.replace("[section]", "");

                    let matches = re.captures(&section_input);

                    let s = if let Some(matches) = matches {
                        let section = matches.get(1).unwrap().as_str();
                        let subsection = matches.get(2).unwrap().as_str();

                        //To Do: for the next thee formats move space to start of line
                        if subsection == "1" {
                            format!("<span class='Section'>{}</span>\n", section)
                        } else {
                            format!("<span class='SubSection'>{}</span>\n", subsection)
                        }
                    } else {
                        format!("<span class='Section'>{}</span>\n", section_input)
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
                            format!(" <span class='InlineSpeaker'>{}</span> ", w.word.word)
                                .as_str(),
                        );
                    }
                }
                _ => (),
            }
        }

        if is_verse_section {
            //previous verse line is complete
            res.push_str(
                complete_verse_line(verse_speaker, &verse_line, &verse_line_number).as_str(),
            );
        }

        if para_open {
            res.push_str("\n</div><!--Close ParaNotIndented-->\n");
        }
        if section_open {
            //res.push_str("\n</div><!--Close Section-->\n");
        }

        if !appcrits_page.is_empty() {
            res.push_str("\n\n<div class='AppCritDiv'>\n");
        }
        for ap in &appcrits_page {
            res.push_str(format!("<div class='appcrit'>{}</div>\n", &ap).as_str());
        }
        if !appcrits_page.is_empty() {
            res.push_str("\n</div><!--End App Crit Div-->\n");
        }
        res.push_str("</div></div><!--End TextContainer-->\n");
        res
    }

    fn page_gloss_start(&self) -> String {
        String::from("<div class=\"GlossContainer\"><div class='gloss-table'>\n")
    }

    fn page_start(&self, title: &str, page_number: usize) -> String {
        format!(
            "\n<!--PAGE START-->\n<div class='Page'>\n<div class='PageTitle'>{title} - Page {page_number}</div>\n"
        )
    }

    fn page_end(&self) -> String {
        String::from("\n</div>\n</div><!--Gloss table end-->\n</div><!--END PAGE-->\n")
    }

    fn document_end(&self) -> String {
        String::from("\n</body></html>\n")
    }

    fn document_start(&self, _title: &str, _start_page: usize) -> String {
        String::from(
            r##"<html lang="en">
    <head>
        <meta charset="UTF-8">
        <title>Greek Vocab DB2</title>
        <meta http-equiv="content-type" content="text/html; charset=utf-8">
        <meta http-equiv="Cache-Control" content="no-cache, no-store, must-revalidate">
        <meta http-equiv="Pragma" content="no-cache">
        <meta http-equiv="Expires" content="0">
        <meta name="viewport" content="width=device-width, user-scalable=no, initial-scale=1, maximum-scale=1">
        <meta http-equiv="X-UA-Compatible" content="IE=edge,chrome=1">

        <style>
        @font-face {
                font-family: "WebNewAthenaUnicode";
                src:
                  local("NewAthenaUnicode"),
                  url("./newathu5_8.ttf") format("truetype");
              }
              @font-face {
                font-family: "WebIFAO";
                src:
                  local("IFAO-Grec-Unicode"),
                  url("./IFAOGrec.ttf") format("truetype");
              }
        BODY { font-family: WebIFAO, WebNewAthenaUnicode, NewAthenaUnicode, helvetica,
                  arial;
              width: 800px;
              margin: 20px auto;
              line-height: 1.5;
        }
        .Page { border-top: 2px solid black; position: relative; }
        .PageTitle { display:none; margin-bottom: 20px; }
        .TextTitle { margin-bottom: 20px; }
        .Section { margin-top: 0px; position:absolute; left:-50px; }
        .SubSection { margin-top: 20px; position:absolute; left:-50px; }
        .VerseLine { display: flex; position: relative; left: 60px;}
        .VerseText { width: 360px; }
        .AppCritDiv { margin: 20px 0px; }
        .gloss-table { border-top: 2px solid red; margin: 20px 0px; padding: 10px; }
        .arrowedHere .listarrow::after {
          content: "→";
          top: -2px;
          position: relative;
        }
        .listarrow {
          position: absolute;
          top: 4px;
          left: -30px;
          font-size: 11pt;
          height: 20px;
          width: 20px;
          border: 1px solid #ddd;
          text-indent: 0px;
          text-align: center;
        }
        .listword {
        cursor: pointer;
          position: relative;
          margin-left: 30px;
          }
        .listword:not(.hqListWord) .listheadword {
          color: red;
          font-weight: bold;
        }
        .clickablelistword {
          padding: 4px;
            padding-left: 4px;
          padding-left: 30px;
          text-indent: -30px;
        }
        .listword:not(.hqListWord) .listfrequency { display:none; }
        .listposwrapper { display: none; }
        .InlineSpeaker { font-weight: bold; }
        .ParaIndented { text-indent: 50px; }
        .TextContainer { padding: 10px; flex-grow: 1; flex-basis: 0; }
        BODY.split { width: auto; }

        .hide-arrowed .alreadyArrowed { display: none; }

        .split .Page { display:flex; }
        .split .gloss-table { border:0px solid transparent; margin:0px; }
        .split .TextContainer {  }
        .split .GlossContainer {
          flex-grow: 1; flex-basis: 0;
          border-left: 1px solid black;
          height: calc(100vh - 43px);
          overflow-y: scroll;
          overflow-x: hidden;
          -webkit-overflow-scrolling: touch;
          padding: 0px;
        }

        </style>
    </head>
    <body class="split hide-arrowed">"##,
        )
    }

    fn make_index(&self, _arrowed_words_index: &[ArrowedWordsIndex]) -> String {
        String::from("\n<!--INDEX-->\n")
    }

    fn blank_page(&self) -> String {
        String::from("\n<!--BLANK PAGE-->\n")
    }
}
