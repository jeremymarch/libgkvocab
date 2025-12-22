// typst compile ../gkvocab_data/ulgv3.typ --font-path .
use super::ExportDocument;
use crate::ArrowedState;
use crate::ArrowedWordsIndex;
use crate::GlossOccurrance;
use crate::HashMap;
use crate::WordType;
use crate::WordUuid;
use regex::Regex;

//https://stackoverflow.com/questions/79173197/how-to-escape-string-for-typst
fn escape_typst(s: &str) -> String {
    s.replace("\"", "\\\"")
        .replace("$", "\\$")
        .replace("#", "\\#")
        .replace("]", "\\u{005D}")
        .replace("[", "\\u{005B}")
        .replace("<b>", "#strong[")
        .replace("</b>", "]")
        .replace("</i>", "\")")
        .replace("<i>", "#fakeitalic(\"") //cuti typst package
        .replace("<sup>", "#super[")
        .replace("</sup>", "]")
        .replace(">", "\\>")
        .replace("<", "\\<")
        .replace("=", "\\u{003D}") //required when = starts a paragraph, else warning: block may not occur inside of a paragraph and was ignored
}

fn complete_verse_line(
    verse_speaker: Option<String>,
    verse_line: &str,
    verse_line_number: &str,
) -> String {
    if verse_line.is_empty() {
        return String::from("");
    }
    let escaped_num = escape_typst(verse_line_number);
    format!(
        "[{}],\n[{}],\n[{}],\n\n",
        verse_speaker.as_ref().unwrap_or(&String::from("")),
        &verse_line,
        if let Ok(i) = verse_line_number.to_string().parse::<i32>() {
            if i % 5 == 0 { verse_line_number } else { "" }
        } else {
            escaped_num.as_str()
        }
    )
}

pub struct ExportTypst {}
impl ExportDocument for ExportTypst {
    fn gloss_entry(&self, gloss_occurrance: &GlossOccurrance, lemma: Option<&str>) -> String {
        if gloss_occurrance.arrowed_state != ArrowedState::Invisible
            && let Some(lemma_unwrapped) = lemma
            && let Some(gloss_unwrapped) = gloss_occurrance.gloss
        {
            format!(
                "[{}],\n[#glosshang[{}]],\n[#glossdef[{}]],\n\n",
                if gloss_occurrance.arrowed_state == ArrowedState::Arrowed {
                    r##"#strong[→]"##
                } else {
                    ""
                },
                escape_typst(lemma_unwrapped),
                escape_typst(&gloss_unwrapped.def)
            )
        } else {
            String::from("")
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
                    format!("\n#align(center)[{}]\n\\\n\\\n", escape_typst(&w.word.word)).as_str(),
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
                        escape_typst(&w.word.word)
                    );
                    if is_verse_section {
                        verse_line.push_str(&s);
                    } else {
                        res.push_str(&s);
                    }
                    prev_non_space = w.word.word == "<" || w.word.word == "[" || w.word.word == "(";
                }
                WordType::ParaWithIndent => res.push_str("\n\n#h(2em)\n"),
                WordType::ParaNoIndent => res.push_str("\n\n"),
                WordType::SectionTitle => res.push_str(
                    format!("\\ #align(center)[{}] \\ ", escape_typst(&w.word.word)).as_str(),
                ),
                WordType::Section => {
                    let section_input = w.word.word.replace("[section]", "");

                    let matches = re.captures(&section_input);

                    let s = if let Some(matches) = matches {
                        let section = matches.get(1).unwrap().as_str();
                        let subsection = matches.get(2).unwrap().as_str();

                        //To Do: for the next three formats move space to start of line
                        if subsection == "1" {
                            format!(
                                "#sidenote(format: it => text(size: 1.2em, it.default))[#strong[{}]] ",
                                section
                            )
                        } else {
                            format!("#sidenote[{}] ", subsection)
                        }
                    } else {
                        format!(
                            "#sidenote(format: it => text(size: 1.2em, it.default))[#strong[{}]] ",
                            section_input
                        )
                    };

                    res.push_str(s.as_str());
                    prev_non_space = true;
                }
                WordType::Speaker => {
                    //fix me can't add this in middle of a versetable
                    if is_verse_section {
                        res.push_str(
                            complete_verse_line(
                                verse_speaker.clone(),
                                &verse_line,
                                &verse_line_number,
                            )
                            .as_str(),
                        );
                        verse_speaker = None;
                        verse_line = String::from("");
                        res.push(')');
                    }

                    res.push_str(&w.word.word);
                    if is_verse_section {
                        res.push_str(
                            r###"
#versetable(
"###,
                        );
                    }
                }
                WordType::InlineSpeaker => {
                    if is_verse_section {
                        verse_speaker = Some(w.word.word.clone());
                    } else {
                        res.push_str(format!("\n\n#strong[{}] ", w.word.word).as_str());
                    }
                }
                WordType::InlineVerseSpeaker => {
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

            res.push_str("\n)\n");
        } else {
            res.push_str("\n\n");
        }

        if !appcrits_page.is_empty() {
            res.push_str("\n\n");
        }
        for ap in appcrits_page {
            res.push_str(format!("{} \\\n", escape_typst(&ap)).as_str());
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
            r###"
            #set page(
              header: context {{
                let page = counter(page).get().first() // Get current page number
                if calc.odd(page) {{
                  align(right, "{}")
                }} else {{
                  align(left, "LGI - UPPER LEVEL GREEK")
                }}
              }}
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
              align(right, "{%MAIN_TITLE%}")
            } else {
              align(left, "LGI - UPPER LEVEL GREEK")
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
        #let placeverse = box.with(pad: (left: 2cm))

        #let indextable = table.with(
            columns: (90%, 10%),
            align: (start + top, end + top),
            stroke: none,
            inset: 0%,
            column-gutter: 0cm,
            row-gutter: 0.225cm,)

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
        #set page(
          header: context {
            let page = counter(page).get().first() // Get current page number
            if calc.odd(page) {
              align(right, "INDEX OF ARROWED WORDS")
            } else {
              align(left, "LGI - UPPER LEVEL GREEK")
            }
          }
        )
        #indextable(
        "##;

        let mut latex = String::from(ARROWED_INDEX_TEMPLATE);
        let mut gloss_per_page = 0;
        for gloss in arrowed_words_index {
            latex.push_str(
                format!(
                    "[{} #box(width: 1fr, repeat[.])],[#box(width: 1fr, repeat[.]) {}],",
                    gloss.gloss_lemma, gloss.page_number
                )
                .as_str(),
            );

            gloss_per_page += 1;
            if gloss_per_page > 43 {
                gloss_per_page = 0;
                //latex.push_str("\n#pagebreak()\n");
                //latex.push_str("\\noindent \n");
            }
        }
        latex.push_str("\n)");
        latex
    }

    fn blank_page(&self) -> String {
        String::from(r##"#pagebreak()"##)
    }
}
