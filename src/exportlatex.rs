use crate::WordType;

use super::{ExportDocument, Word};

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

#[allow(dead_code)]
pub struct ExportLatex {}
impl ExportDocument for ExportLatex {
    fn gloss_entry(&self, lemma: &str, gloss: &str, arrowed: bool) -> String {
        format!(
            " {} & {} & {} \\\\\n",
            if arrowed { r#"\textbf{→}"# } else { "" },
            escape_latex(lemma),
            escape_latex(gloss)
        )
    }

    fn make_text(&self, words: &[Word]) -> String {
        let mut res = String::from("");
        let mut prev_non_space = true;
        let mut last_type = WordType::InvalidType;
        for w in words {
            match w.word_type {
                WordType::WorkTitle => res.push_str(
                    format!(
                        "\\begin{{center}}\\noindent\\textbf{{{}}}\\par\\end{{center}}\n",
                        escape_latex(&w.word)
                    )
                    .as_str(),
                ),
                WordType::Word | WordType::Punctuation => {
                    //0 | 1
                    let punc = vec![
                        ".", ",", "·", "·", ";", ";", ">", "]", ")", ",\"", ".”", ".\"", "·\"",
                        "·\"", ".’",
                    ];
                    res.push_str(
                        format!(
                            "{}{}",
                            if punc.contains(&w.word.as_str()) || prev_non_space {
                                ""
                            } else {
                                " "
                            },
                            w.word
                        )
                        .as_str(),
                    );
                    prev_non_space = w.word == "<" || w.word == "[" || w.word == "(";
                }
                WordType::ParaWithIndent => res.push_str("\n\\par\n"),
                WordType::ParaNoIndent => res.push_str("\n\\noindent\n"),
                WordType::Section => {
                    res.push_str(format!(" \\hspace{{0pt}}\\marginsec{{{}}}", w.word).as_str());
                    //if last_type == WordType::InvalidType || last_type == WordType::ParaWithIndent {
                    //-1 || 6
                    prev_non_space = true;
                    // } else {
                    //     prev_non_space = false;
                    // }
                }
                WordType::Speaker => {
                    res.push_str(format!("\\begin{{center}}{}\\end{{center}}", w.word).as_str());
                }
                _ => (),
            }
            last_type = w.word_type.clone();
        }
        res
    }

    fn page_gloss_start(&self) -> String {
        String::from(
            "\n\n\\begin{table}[b!]\\leftskip -0.84cm\n\\begin{tabular}{ m{0.2cm} L{3.25in} D{3.1in} }\n",
        )
    }

    fn page_start(&self, title: &str) -> String {
        format!(
            "\n\\fancyhead[OR]{{{title}}}\n\\begin{{spacing}}{{\\GlossLineSpacing}}\n\\noindent\n"
        )
    }

    fn page_end(&self) -> String {
        String::from("\n\\end{tabular}\n\\end{table}\n\\newpage\n")
    }

    fn document_end(&self) -> String {
        String::from("\n\\end{document}\n")
    }

    fn document_start(&self, start_page: usize) -> String {
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
\setcounter{page}{%PAGE_NUM%}
%\newpage
%\mbox{}
\newpage
"###;

        start.replace("%PAGE_NUM%", start_page.to_string().as_str())
    }
}
