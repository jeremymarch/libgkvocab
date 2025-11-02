use super::ExportDocument;
use crate::ArrowedState;
use crate::ArrowedWordsIndex;
use crate::GlossOccurrance;
use crate::WordType;
use crate::WordUuid;
use regex::Regex;
use std::collections::HashMap;

//https://tex.stackexchange.com/questions/34580/escape-character-in-latex
fn escape_latex(s: &str) -> String {
    s.replace("\\", "\\textbackslash")
        .replace("{", "\\{")
        .replace("}", "\\}")
        .replace("<i>", "\\textit{")
        .replace("</i>", "}")
        .replace("<b>", "\\textbf{")
        .replace("</b>", "}")
        .replace("&", "\\&")
        .replace("%", "\\%")
        .replace("$", "\\$")
        .replace("#", "\\#")
        .replace("_", "\\_")
        .replace("~", "\\textasciitilde")
        .replace("^", "\\textasciicircum")
}

fn complete_verse_line(
    verse_speaker: Option<String>,
    verse_line: &str,
    verse_line_number: &str,
) -> String {
    format!(
        "{} & {} & {} \\\\\n",
        verse_speaker.as_ref().unwrap_or(&String::from("")),
        &verse_line,
        if let Ok(i) = verse_line_number.parse::<i32>() {
            if i % 5 == 0 { verse_line_number } else { "" }
        } else {
            &verse_line_number
        }
    )
}

pub struct ExportLatex {}
impl ExportDocument for ExportLatex {
    fn gloss_entry(&self, gloss_occurrance: &GlossOccurrance, lemma: &str, gloss: &str) -> String {
        if gloss_occurrance.arrowed_state == ArrowedState::Invisible {
            String::from("")
        } else {
            format!(
                "{} & {} & {} \\\\\n",
                if gloss_occurrance.arrowed_state == ArrowedState::Arrowed {
                    r#"\textbf{→}"#
                } else {
                    ""
                },
                escape_latex(lemma),
                escape_latex(gloss)
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

        //println!("page count {}", gloss_occurrances.len());
        for w in gloss_occurrances {
            if let Some(ap) = appcrit_hash.get(&w.word.uuid) {
                appcrits_page.push(ap.clone());
            }
            //println!("word type {:?}", w.word.word_type);

            match w.word.word_type {
                WordType::VerseLine => {
                    if !is_verse_section {
                        res.push_str(r##"
\end{spacing}
\begin{tabular}%https://tex.stackexchange.com/questions/338009/right-alignment-for-plength-box-in-tabular
  {>{\raggedright\arraybackslash}p{1cm}%
   >{\raggedright\arraybackslash}p{9.5cm}%
   >{\raggedleft\arraybackslash}p{2cm}%
  }"##);
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
                    format!(
                        "\\begin{{center}}\\noindent\\textbf{{{}}}\\par\\end{{center}}\n",
                        escape_latex(&w.word.word)
                    )
                    .as_str(),
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
                            format!("\\hspace{{0pt}}\\marginsec{{{}}} ", section)
                        } else {
                            format!("\\hspace{{0pt}}\\marginseclight{{{}}} ", subsection)
                        }
                    } else {
                        format!("\\hspace{{0pt}}\\marginsec{{{}}} ", section_input)
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
                    let s = format!("\\begin{{center}}{}\\end{{center}}", w.word.word);
                    res.push_str(s.as_str());
                }
                WordType::InlineSpeaker => {
                    if is_verse_section {
                        verse_speaker = Some(w.word.word.clone());
                    } else {
                        res.push_str(format!("\\par \\textbf{{{}}} ", w.word.word).as_str());
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

            res.push_str("~\\\\\n\\end{tabular}");
        } else {
            res.push_str("\\hspace*{\\fill}\n\\end{spacing}\n");
        }

        if !appcrits_page.is_empty() {
            res.push_str("~\\\\\n");
        }
        for ap in appcrits_page {
            res.push_str(format!("{}\\\\\n", escape_latex(&ap)).as_str());
        }
        res
    }

    fn page_gloss_start(&self) -> String {
        String::from(
            "\\begin{table}[b!]\\leftskip -0.84cm\n\\begin{tabular}{ m{0.2cm} L{3.25in} D{3.1in} }\n",
        )
    }

    fn page_start(&self, title: &str, _page_number: usize) -> String {
        format!(
            "\\fancyhead[OR]{{{title}}}\n\\begin{{spacing}}{{\\GlossLineSpacing}}\n\\noindent\n"
        )
    }

    fn page_end(&self) -> String {
        String::from("\\end{tabular}\n\\end{table}\n\\newpage\n")
    }

    fn document_end(&self) -> String {
        String::from("\\end{document}\n")
    }

    fn document_start(&self, title: &str, start_page: usize) -> String {
        let start = r###"\documentclass[twoside,openright,12pt,letterpaper]{book}
%\usepackage[margin=1.0in]{geometry}
\usepackage[twoside, margin=1.0in]{geometry} %bindingoffset=0.5in,
\usepackage[utf8]{inputenc}
\usepackage{fontspec}
\usepackage{array}
\usepackage{booktabs}
\usepackage{ragged2e}
\usepackage{setspace}
\usepackage{navigator}

\newcommand{\GlossLineSpacing}{1.5}


\setmainfont[Scale=MatchUppercase,Ligatures=TeX, BoldFont={*BOLD}, ItalicFont={IFAOGrec.ttf}, ItalicFeatures={FakeSlant=0.2}]{IFAOGrec.ttf}
%\setmainlanguage[variant=polytonic]{greek}
\tolerance=10000 % https://www.texfaq.org/FAQ-overfull
\setlength{\extrarowheight}{8pt}
\newcolumntype{L}{>{\setlength{\RaggedRight\parindent}{-2em}\leftskip 2em}p}
\newcolumntype{D}{>{\setlength{\RaggedRight}}p}

\usepackage{fancyhdr} % http://tug.ctan.org/tex-archive/macros/latex/contrib/fancyhdr/fancyhdr.pdf

\pagestyle{fancy}
\fancyhf{}
\renewcommand{\headrulewidth}{0.0pt}
  \fancyhead[EL]{%MAIN_TITLE%}% Title on Even page, Centered
  \fancyhead[OR]{}% Author on Odd page, Centered
\setlength{\headheight}{14.49998pt}
\cfoot{\thepage}

%\usepackage{enumitem}
%\SetLabelAlign{margin}{\llap{#1~~}}
%\usepackage{showframe} % just to show the margins
%https://tex.stackexchange.com/questions/223701/labels-in-the-left-margin

%https://tex.stackexchange.com/questions/40748/use-sections-inline
\newcommand{\marginsec}[1]{\vadjust{\vbox to 0pt{\sbox0{\bfseries#1\quad}\kern-0.89em\llap{\box0}}}}
\newcommand{\marginseclight}[1]{\vadjust{\vbox to 0pt{\sbox0{\footnotesize#1\hspace{0.25em}\quad}\kern-0.85em\llap{\box0}}}}
\usepackage[none]{hyphenat}
\usepackage[polutonikogreek,english]{babel} %https://tex.stackexchange.com/questions/13067/utf8x-vs-utf8-inputenc
\usepackage{microtype}
\begin{document}
%\clearpage
\setcounter{page}{%PAGE_NUM%}
%\newpage
%\mbox{}
\newpage
"###;

        start
            .replace("%MAIN_TITLE%", title)
            .replace("%PAGE_NUM%", start_page.to_string().as_str())
    }

    fn make_index(&self, arrowed_words_index: &[ArrowedWordsIndex]) -> String {
        const ARROWED_INDEX_TEMPLATE: &str = r##"
        \newpage
        \fancyhead[OR]{INDEX OF ARROWED WORDS}
        %\begin{spacing}{\GlossLineSpacing}
        \noindent
        "##;

        let mut latex = String::from(ARROWED_INDEX_TEMPLATE);
        let mut gloss_per_page = 0;
        for gloss in arrowed_words_index {
            //$latex .= explode(",", $a[0], 2)[0] . " \dotfill " . $a[2] . " \\\\ \n";
            latex.push_str(
                &gloss
                    .gloss_lemma
                    .chars()
                    .take_while(|&ch| ch != ',')
                    .collect::<String>(),
            );
            latex.push_str(r" \dotfill ");
            latex.push_str(&gloss.page_number.to_string());
            latex.push_str(" \\\\ \n");

            gloss_per_page += 1;
            if gloss_per_page > 43 {
                gloss_per_page = 0;
                latex.push_str("\\newpage \n");
                latex.push_str("\\noindent \n");
            }
        }
        latex
    }

    fn blank_page(&self) -> String {
        String::from(
            r##"\fancyhead[OR]{}
\begin{spacing}{\GlossLineSpacing}
\noindent
\hspace*{\fill}
\end{spacing}
\begin{table}[b!]\leftskip -0.84cm
\begin{tabular}{ m{0.2cm} L{3.25in} D{3.1in} }
\end{tabular}
\end{table}
\newpage
"##,
        )
    }
}
