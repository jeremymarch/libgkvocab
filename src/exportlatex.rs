use crate::WordType;

use super::{ExportDocument, Word};

#[allow(dead_code)]
pub struct ExportLatex {}
impl ExportDocument for ExportLatex {
    fn gloss_entry(&self, lemma: &str, gloss: &str, arrowed: bool) -> String {
        format!(
            " {} & {} & {} \\\\\n",
            if arrowed { r#"\textbf{→}"# } else { "" },
            lemma,
            gloss
        )
    }

    fn make_text(&self, words: &[Word]) -> String {
        let mut res = String::from("");
        for w in words {
            match w.word_type {
                WordType::Word => res.push_str(format!("{} ", w.word).as_str()),
                WordType::ParaWithIndent => res.push_str("\n\\par\n"),
                WordType::ParaNoIndent => res.push_str("\n\\noindent\n"),
                _ => (),
            }
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
