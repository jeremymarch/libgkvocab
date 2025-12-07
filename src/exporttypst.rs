use super::ExportDocument;
use crate::ArrowedState;
use crate::ArrowedWordsIndex;
use crate::GlossOccurrance;
use crate::WordType;
use crate::WordUuid;
use regex::Regex;
use std::collections::HashMap;

//https://tex.stackexchange.com/questions/34580/escape-character-in-latex
fn escape_typst(s: &str) -> String {
    s.replace("<i>", "#fakeitalic(\"") //cuti typst package
        .replace("</i>", "\")")
        .replace("<b>", "#strong[")
        .replace("</b>", "]")
        .replace("&", "\\&")
        .replace("\"", "\\\"")
        .replace("#", "\\#")
}

fn complete_verse_line(
    verse_speaker: Option<String>,
    verse_line: &str,
    verse_line_number: &str,
) -> String {
    format!(
        "[{}],\n[{}],\n[{}],\n\n",
        verse_speaker.as_ref().unwrap_or(&String::from("")),
        &verse_line,
        if let Ok(i) = verse_line_number.parse::<i32>() {
            if i % 5 == 0 { verse_line_number } else { "" }
        } else {
            &verse_line_number
        }
    )
}

pub struct ExportTypst {}
impl ExportDocument for ExportTypst {
    fn gloss_entry(&self, gloss_occurrance: &GlossOccurrance, lemma: Option<&str>) -> String {
        if gloss_occurrance.arrowed_state == ArrowedState::Invisible
            || lemma.is_none()
            || gloss_occurrance.gloss.is_none()
        {
            String::from("")
        } else {
            format!(
                "[{}],\n[{}],\n[{}],\n\n",
                if gloss_occurrance.arrowed_state == ArrowedState::Arrowed {
                    r##"#strong[→]"##
                } else {
                    ""
                },
                escape_typst(lemma.unwrap()),
                escape_typst(&gloss_occurrance.gloss.unwrap().def)
            )
        }
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

        for w in gloss_occurrances {
            if let Some(ap) = appcrit_hash.get(&w.word.uuid) {
                appcrits_page.push(ap.clone());
            }

            match w.word.word_type {
                WordType::VerseLine => {
                    if !is_verse_section {
                        res.push_str(
                            r###"
                            #placeverse()[
                                #versetable(
                                "###,
                        );
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
                WordType::WorkTitle => res.push_str(
                    format!("\n#align(center)[{}]\n", escape_typst(&w.word.word)).as_str(),
                ),
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
                    if is_verse_section {
                        verse_line.push_str(&s);
                    } else {
                        res.push_str(&s);
                    }
                    prev_non_space = w.word.word == "<" || w.word.word == "[" || w.word.word == "(";
                }
                WordType::ParaWithIndent => res.push_str("\n\n"),
                WordType::ParaNoIndent => res.push_str("\n\\noindent\n"),
                WordType::Section => {
                    let section_input = w.word.word.replace("[section]", "");

                    let matches = re.captures(&section_input);

                    let s = if let Some(matches) = matches {
                        let section = matches.get(1).unwrap().as_str();
                        let subsection = matches.get(2).unwrap().as_str();

                        //To Do: for the next three formats move space to start of line
                        if subsection == "1" {
                            format!("#sidenote[#strong[{}]] ", section)
                        } else {
                            format!("#sidenote[{}] ", subsection)
                        }
                    } else {
                        format!("#sidenote[#strong[{}]] ", section_input)
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
                    let s = format!("\n#align(center)[{}]\n", w.word.word);
                    res.push_str(s.as_str());
                }
                WordType::InlineSpeaker => {
                    if is_verse_section {
                        verse_speaker = Some(w.word.word.clone());
                    } else {
                        res.push_str(format!("\n\n#strong[{}] ", w.word.word).as_str());
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

            res.push_str("\n)\n]\n");
        } else {
            res.push_str("\n\n");
        }

        if !appcrits_page.is_empty() {
            res.push_str("\n");
        }
        for ap in appcrits_page {
            res.push_str(format!("{}\n\n", escape_typst(&ap)).as_str());
        }
        res
    }

    fn page_gloss_start(&self) -> String {
        String::from(
            r###"
            #placegloss()[
                #glosstable(
                "###,
        )
    }

    fn page_start(&self, title: &str, _page_number: usize) -> String {
        format!(
            r###"#set page(
              header: context {
                let page = counter(page).get().first() // Get current page number
                if calc.odd(page) {
                  align(left, "LGI - UPPER LEVEL GREEK") // Content for odd pages
                } else {
                  align(right, "{}") // Content for even pages
                }
              }
            )
            "###,
            title
        )
    }

    fn page_end(&self) -> String {
        String::from(
            r###"
            )
            ]

            #pagebreak()
            "###,
        )
    }

    fn document_end(&self) -> String {
        String::from("\n")
    }

    fn document_start(&self, title: &str, start_page: usize) -> String {
        let start = r###"#import "@preview/marge:0.1.0": sidenote
        #let sidenote = sidenote.with(side: left, padding: 3em)

        #import "@preview/cuti:0.4.0": fakeitalic

        #set page(width: 8.5in, height: 11in)
        #set page(numbering: "1")
        #counter(page).update(%PAGE_NUM%)
        #set page(
          header: context {
            let page = counter(page).get().first() // Get current page number
            if calc.odd(page) {
              align(left, "LGI - UPPER LEVEL GREEK") // Content for odd pages
            } else {
              align(right, "%MAIN_TITLE%") // Content for even pages
            }
          }
        )

        #let glosshang = par.with(hanging-indent: 2em, justify: false, leading: 0.7em,)
        #let glossdef = par.with(justify: false, leading: 0.7em,)
        #let glosstable = table.with(
            columns: (0.6cm, 8.0cm, 9.0cm),
            align: start + top,
            stroke: none,
            row-gutter: 0.07cm,)
        #let versetable = table.with(
            columns: (1.1cm, 9.0cm, 3.0cm),
            align: start + top,
            stroke: none,
            row-gutter: 0.07cm,)
        #let placegloss = place.with(bottom, dx: -0.8cm)
        #let placeverse = place.with(top, dx: 2cm)

        #set par(
          justify: true,
          leading: 0.9em,
          spacing: 2em
        )
        #set text(
          font: "IFAO-Grec Unicode",
          size: 12pt,
        )
"###;

        start
            .replace("%MAIN_TITLE%", title)
            .replace("%PAGE_NUM%", start_page.to_string().as_str())
    }

    fn make_index(&self, arrowed_words_index: &[ArrowedWordsIndex]) -> String {
        const ARROWED_INDEX_TEMPLATE: &str = r##"
        #pagebreak()
        "##;

        let mut latex = String::from(ARROWED_INDEX_TEMPLATE);
        let mut gloss_per_page = 0;
        for gloss in arrowed_words_index {
            //$latex .= explode(",", $a[0], 2)[0] . " \dotfill " . $a[2] . " \\\\ \n";
            latex.push_str(&gloss.gloss_lemma);
            latex.push_str(r" \dotfill ");
            latex.push_str(&gloss.page_number.to_string());
            latex.push_str("\n");

            gloss_per_page += 1;
            if gloss_per_page > 43 {
                gloss_per_page = 0;
                latex.push_str("\n#pagebreak()\n");
                //latex.push_str("\\noindent \n");
            }
        }
        latex
    }

    fn blank_page(&self) -> String {
        String::from(r##"#pagebreak()"##)
    }
}
