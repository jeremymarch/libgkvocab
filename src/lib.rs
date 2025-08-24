use std::collections::HashMap;

pub fn build_gloss_map() {}

pub struct Word {
    word_id: u32,
    word: String,
    gloss_id: Option<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ArrowedState {
    Visible,
    Arrowed,
    Invisible,
}

// #[derive(Clone, Debug, PartialEq)]
// pub enum ExportFormat {
//     Fotd,
//     Latex,
//     Html,
//     Xml,
// }

//the word id where a gloss is arrowed
pub struct GlossArrow {
    gloss_id: u32,
    word_id: u32,
}

#[derive(Clone, Debug)]
pub struct Gloss {
    gloss_id: u32,
    lemma: String,
    sort_alpha: String,
    gloss: String,
    unit: u32,
    pos: String,
    status: u32,
}

#[derive(Clone, Debug)]
pub struct GlossOccurrance {
    //<'a> {
    //gloss_ref: &'a Gloss,
    gloss_id: u32,
    lemma: String,
    sort_alpha: String,
    gloss: String,
    arrowed_seq: Option<usize>,
    arrowed_state: ArrowedState,
}

pub struct Sequence {
    sequence_id: u32,
    name: String,
    gloss_name: String,
    gloss: Vec<Gloss>,
    texts: Vec<String>,
    arrowed_words: Vec<GlossArrow>,
}

pub struct ExportLatex {}
impl ExportLatex {
    fn new() -> ExportLatex {
        ExportLatex {}
    }
}

pub trait ExportDocument {
    fn gloss_entry(&self, lemma: &str, gloss: &str, arrowed: bool) -> String;
    fn make_text(&self, words: &[Word]) -> String;
    fn page_start(&self, title: &str) -> String;
    fn page_end(&self) -> String;
    fn page_gloss_start(&self) -> String;
    fn document_end(&self) -> String;
    fn document_start(&self) -> String;
}

impl ExportDocument for ExportLatex {
    fn gloss_entry(&self, lemma: &str, gloss: &str, arrowed: bool) -> String {
        format!(
            " {} & {} & {} \\\\\n",
            if arrowed { "->" } else { "" },
            lemma,
            gloss
        )
    }

    fn make_text(&self, words: &[Word]) -> String {
        let mut res = String::from("");
        for w in words {
            res.push_str(format!("{} ", w.word).as_str());
        }
        res
    }

    fn page_gloss_start(&self) -> String {
        String::from(
            "\n\n\\begin{table}[b!]\\leftskip -0.84cm\n\\begin{tabular}{ m{0.2cm} L{3.25in} D{3.1in} }\n",
        )
    }

    fn page_start(&self, title: &str) -> String {
        format!("\n\\fancyhead[OR]{{{title}}}\n\\begin{{spacing}}{{\\GlossLineSpacing}}\n\n")
    }

    fn page_end(&self) -> String {
        String::from("\n\\end{tabular}\n\\end{table}\n\\newpage\n")
    }

    fn document_end(&self) -> String {
        String::from("\n\\end{document}\n")
    }

    fn document_start(&self) -> String {
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
          \fancyhead[EL]{LGI - UPPER LEVEL GREEK}% Title on Even page, Centered
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
        \setcounter{page}{24}
        %\newpage
        %\mbox{}
        \newpage
        "###;

        start.to_string()
    }
}

pub fn make_page(
    words: &[Word],
    gloss_hash: &HashMap<u32, GlossOccurrance>,
    seq_offset: usize,
    export: &impl ExportDocument,
) -> String {
    let mut page = export.page_start("title");
    page.push_str(&export.make_text(words));

    page.push_str(&export.page_gloss_start());

    let s = make_gloss_page(words, gloss_hash, seq_offset);
    page.push_str(&get_gloss_string(&s, export));

    page.push_str(&export.page_end());
    page
}

pub fn make_document(
    words: &[Word],
    gloss_hash: HashMap<u32, GlossOccurrance>,
    export: &impl ExportDocument,
) -> String {
    let words_per_page = [3, 3, 4];
    let mut doc = export.document_start();

    let mut index = 0;
    for (i, w) in words_per_page.iter().enumerate() {
        if i == words_per_page.len() - 1 {
            doc.push_str(make_page(&words[index..], &gloss_hash, index, export).as_str());
        } else {
            doc.push_str(make_page(&words[index..index + w], &gloss_hash, index, export).as_str());
        }
        index += w;
    }

    doc.push_str(&export.document_end());
    doc
}

//sets arrowed state and makes glosses unique on page
pub fn make_gloss_page(
    words: &[Word],
    glosshash: &HashMap<u32, GlossOccurrance>,
    seq_offset: usize,
) -> Vec<GlossOccurrance> {
    let mut glosses: HashMap<u32, GlossOccurrance> = HashMap::new();

    for (seq, w) in words.iter().enumerate() {
        if let Some(gloss_id) = w.gloss_id
            && let Some(gloss) = glosshash.get(&gloss_id)
        {
            let mut g = gloss.clone();
            if gloss.arrowed_seq.is_none()
                || (gloss.arrowed_seq.is_some() && seq + seq_offset < gloss.arrowed_seq.unwrap())
            {
                g.arrowed_state = ArrowedState::Visible;
            } else if gloss.arrowed_seq.is_some() && seq + seq_offset == gloss.arrowed_seq.unwrap()
            {
                g.arrowed_state = ArrowedState::Arrowed;
            } else {
                g.arrowed_state = ArrowedState::Invisible;
            }

            //if arrowed insert it, or if it's not already inserted
            //we want to avoid replacing an arrowed version with a non-arrowed version
            if g.arrowed_state == ArrowedState::Arrowed || !glosses.contains_key(&gloss_id) {
                glosses.insert(gloss_id, g);
            }
        }
    }

    let mut sorted_glosses: Vec<GlossOccurrance> = glosses.values().cloned().collect();
    sorted_glosses.sort_by(|a, b| {
        a.sort_alpha
            .to_lowercase()
            .cmp(&b.sort_alpha.to_lowercase())
    });

    sorted_glosses
}

pub fn get_gloss_string(glosses: &[GlossOccurrance], export: &impl ExportDocument) -> String {
    let mut res = String::from("");
    for g in glosses {
        match g.arrowed_state {
            ArrowedState::Arrowed => {
                res.push_str(export.gloss_entry(&g.lemma, &g.gloss, true).as_str())
            }
            ArrowedState::Visible => {
                res.push_str(export.gloss_entry(&g.lemma, &g.gloss, false).as_str())
            }
            ArrowedState::Invisible => (),
        }
    }
    res
}

pub fn make_gloss_occurrances(
    words: &[Word],
    seq: Sequence,
    glosses_hash: HashMap<u32, Gloss>,
) -> Vec<GlossOccurrance> {
    //hashmap of word_ids which are arrowed
    let mut aw = HashMap::new();
    for s in seq.arrowed_words {
        aw.insert(s.word_id, s.gloss_id);
    }

    //get sequence where the gloss is arrowed
    let mut glosses_seq = HashMap::new();
    for (seq, w) in words.iter().enumerate() {
        if let Some(arrowed_word_gloss) = aw.get(&w.word_id)
            && let Some(gloss) = w.gloss_id
            && *arrowed_word_gloss == gloss
        {
            glosses_seq.insert(gloss, seq);
        }
    }

    let mut r = vec![];
    for w in words {
        if let Some(gloss_id) = w.gloss_id
            && let Some(gloss) = glosses_hash.get(&gloss_id)
        {
            if let Some(gloss_seq) = glosses_seq.get(&gloss_id) {
                r.push(GlossOccurrance {
                    gloss_id,
                    lemma: gloss.lemma.clone(),
                    sort_alpha: gloss.sort_alpha.clone(),
                    gloss: gloss.gloss.clone(),
                    arrowed_seq: Some(*gloss_seq),
                    arrowed_state: ArrowedState::Visible,
                });
            } else {
                r.push(GlossOccurrance {
                    gloss_id,
                    lemma: gloss.lemma.clone(),
                    sort_alpha: gloss.sort_alpha.clone(),
                    gloss: gloss.gloss.clone(),
                    arrowed_seq: None,
                    arrowed_state: ArrowedState::Visible,
                });
            }
        }
    }

    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let glosses = vec![
            Gloss {
                gloss_id: 1,
                lemma: String::from("ἄγω"),
                sort_alpha: String::from("αγω"),
                gloss: String::from("blah gloss"),
                pos: String::from("verb"),
                unit: 8,
                status: 1,
            },
            Gloss {
                gloss_id: 3,
                lemma: String::from("γαμέω"),
                sort_alpha: String::from("γαμεω"),
                gloss: String::from("blah gloss"),
                pos: String::from("verb"),
                unit: 8,
                status: 1,
            },
            Gloss {
                gloss_id: 2,
                lemma: String::from("βλάπτω"),
                sort_alpha: String::from("βλαπτω"),
                gloss: String::from("blah gloss"),
                pos: String::from("verb"),
                unit: 8,
                status: 1,
            },
        ];

        let sequence = Sequence {
            sequence_id: 1,
            name: String::from("SGI"),
            gloss_name: String::from("H&Qplus"),
            gloss: glosses.clone(),
            arrowed_words: vec![
                GlossArrow {
                    word_id: 5,
                    gloss_id: 1,
                },
                GlossArrow {
                    word_id: 1,
                    gloss_id: 2,
                },
                GlossArrow {
                    word_id: 10,
                    gloss_id: 3,
                },
            ],

            texts: vec![],
        };

        let words = vec![
            Word {
                word_id: 0,
                word: String::from("βλάπτει"),
                gloss_id: Some(2),
            },
            Word {
                word_id: 10,
                word: String::from("γαμεῖ"),
                gloss_id: Some(3),
            },
            Word {
                word_id: 4,
                word: String::from("ἄγει"),
                gloss_id: Some(1),
            },
            Word {
                word_id: 1,
                word: String::from("βλάπτει"),
                gloss_id: Some(2),
            },
            Word {
                word_id: 6,
                word: String::from("ἄγει"),
                gloss_id: Some(1),
            },
            Word {
                word_id: 11,
                word: String::from("γαμεῖ"),
                gloss_id: Some(3),
            },
            Word {
                word_id: 2,
                word: String::from("βλάπτει"),
                gloss_id: Some(2),
            },
            Word {
                word_id: 20,
                word: String::from("βλάπτει"),
                gloss_id: Some(2),
            },
            Word {
                word_id: 5,
                word: String::from("ἄγεις"),
                gloss_id: Some(1),
            },
            Word {
                word_id: 7,
                word: String::from("ἄγεις"),
                gloss_id: Some(1),
            },
            Word {
                word_id: 8,
                word: String::from("γαμεῖ"),
                gloss_id: Some(3),
            },
            Word {
                word_id: 9,
                word: String::from("γαμεῖ"),
                gloss_id: Some(3),
            },
        ];

        let mut glosses_hash = HashMap::new();
        for g in glosses {
            glosses_hash.insert(g.gloss_id, g.clone());
        }
        let glosses_occurrances = make_gloss_occurrances(&words, sequence, glosses_hash);

        let mut gloss_occurrances_hash = HashMap::new();
        for g in glosses_occurrances {
            gloss_occurrances_hash.insert(g.gloss_id, g.clone());
        }

        let export = ExportLatex::new();
        let p = make_document(&words, gloss_occurrances_hash, &export);
        println!("test: \n{p}");
    }

    #[test]
    fn it_works2() {
        let glosses = vec![
            Gloss {
                gloss_id: 1,
                lemma: String::from("ἄγω"),
                sort_alpha: String::from("αγω"),
                gloss: String::from("blah gloss"),
                pos: String::from("verb"),
                unit: 8,
                status: 1,
            },
            Gloss {
                gloss_id: 3,
                lemma: String::from("γαμέω"),
                sort_alpha: String::from("γαμεω"),
                gloss: String::from("blah gloss"),
                pos: String::from("verb"),
                unit: 8,
                status: 1,
            },
            Gloss {
                gloss_id: 2,
                lemma: String::from("βλάπτω"),
                sort_alpha: String::from("βλαπτω"),
                gloss: String::from("blah gloss"),
                pos: String::from("verb"),
                unit: 8,
                status: 1,
            },
        ];

        let sequence = Sequence {
            sequence_id: 1,
            name: String::from("SGI"),
            gloss_name: String::from("H&Qplus"),
            gloss: glosses.clone(),
            arrowed_words: vec![
                GlossArrow {
                    word_id: 5,
                    gloss_id: 1,
                },
                GlossArrow {
                    word_id: 1,
                    gloss_id: 2,
                },
                GlossArrow {
                    word_id: 10,
                    gloss_id: 3,
                },
            ],

            texts: vec![],
        };

        let words = vec![
            Word {
                word_id: 0,
                word: String::from("βλάπτει"),
                gloss_id: Some(2),
            },
            Word {
                word_id: 10,
                word: String::from("γαμεῖ"),
                gloss_id: Some(3),
            },
            Word {
                word_id: 4,
                word: String::from("ἄγει"),
                gloss_id: Some(1),
            },
            Word {
                word_id: 1,
                word: String::from("βλάπτει"),
                gloss_id: Some(2),
            },
            Word {
                word_id: 6,
                word: String::from("ἄγει"),
                gloss_id: Some(1),
            },
            Word {
                word_id: 11,
                word: String::from("γαμεῖ"),
                gloss_id: Some(3),
            },
            Word {
                word_id: 2,
                word: String::from("βλάπτει"),
                gloss_id: Some(2),
            },
            Word {
                word_id: 20,
                word: String::from("βλάπτει"),
                gloss_id: Some(2),
            },
            Word {
                word_id: 5,
                word: String::from("ἄγεις"),
                gloss_id: Some(1),
            },
            Word {
                word_id: 7,
                word: String::from("ἄγεις"),
                gloss_id: Some(1),
            },
            Word {
                word_id: 8,
                word: String::from("γαμεῖ"),
                gloss_id: Some(3),
            },
            Word {
                word_id: 9,
                word: String::from("γαμεῖ"),
                gloss_id: Some(3),
            },
        ];

        let mut glosses_hash = HashMap::new();
        for g in glosses {
            glosses_hash.insert(g.gloss_id, g.clone());
        }
        let glosses_occurrances = make_gloss_occurrances(&words, sequence, glosses_hash);

        let mut gloss_occurrances_hash = HashMap::new();
        for g in glosses_occurrances {
            gloss_occurrances_hash.insert(g.gloss_id, g.clone());
        }

        let export = ExportLatex::new();
        let p = make_document(&words, gloss_occurrances_hash, &export);
        println!("test: \n{p}");
    }
}
