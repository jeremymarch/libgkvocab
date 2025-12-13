// typst compile ../gkvocab_data/ulgv3.typ --font-path .
use super::ExportDocument;
use crate::ArrowedState;
use crate::ArrowedWordsIndex;
use crate::GlossOccurrance;
use crate::WordType;
use crate::WordUuid;
use regex::Regex;
use std::collections::HashMap;

//https://stackoverflow.com/questions/79173197/how-to-escape-string-for-typst
fn escape_fodt(s: &str) -> String {
    s.replace("&", "&amp;")
        .replace("\"", "&quot;")
        .replace("'", "&apos;")
        .replace(">", "&gt;")
        .replace("<", "&lt;")
        .replace("&lt;b&gt;", r###"<text:span text:style-name="T1">"###)
        .replace("&lt;/b&gt;", "</text:span>")
        .replace("&lt;i&gt;", r###"<text:span text:style-name="T2">"###)
        .replace("&lt;/i&gt;", "</text:span>")
        .replace("&lt;sup&gt;", r###"<text:span text:style-name="T3">"###)
        .replace("&lt;/sup&gt;", "</text:span>")
}

fn complete_verse_line(
    verse_speaker: Option<String>,
    verse_line: &str,
    verse_line_number: &str,
) -> String {
    if verse_line.is_empty() {
        return String::from("");
    }
    let escaped_num = escape_fodt(verse_line_number);
    format!(
        r###"
        <table:table-row>
         <table:table-cell table:style-name="VerseTable.A1" office:value-type="string">
          <text:p text:style-name="Table_20_Contents">{}</text:p>
         </table:table-cell>
         <table:table-cell table:style-name="VerseTable.A1" office:value-type="string">
          <text:p text:style-name="Table_20_Contents">{}</text:p>
         </table:table-cell>
         <table:table-cell table:style-name="VerseTable.A1" office:value-type="string">
          <text:p text:style-name="Table_20_Contents">{}</text:p>
         </table:table-cell>
        </table:table-row>
"###,
        verse_speaker.as_ref().unwrap_or(&String::from("")),
        &verse_line,
        if let Ok(i) = verse_line_number.to_string().parse::<i32>() {
            if i % 5 == 0 { verse_line_number } else { "" }
        } else {
            escaped_num.as_str()
        }
    )
}

pub struct ExportFodt {}
impl ExportDocument for ExportFodt {
    fn gloss_entry(&self, gloss_occurrance: &GlossOccurrance, lemma: Option<&str>) -> String {
        if gloss_occurrance.arrowed_state != ArrowedState::Invisible
            && let Some(lemma_unwrapped) = lemma
            && let Some(gloss_unwrapped) = gloss_occurrance.gloss
        {
            format!(
                r###"
    <table:table-row table:style-name="GlossTableRow">
      <table:table-cell table:style-name="GlossTableCell" office:value-type="string">
        <text:p text:style-name="P8">{}</text:p>
      </table:table-cell>
      <table:table-cell table:style-name="GlossTableCell" office:value-type="string">
        <text:p text:style-name="GlossTableLemma">{}</text:p>
      </table:table-cell>
      <table:table-cell table:style-name="GlossTableCell" office:value-type="string">
        <text:p text:style-name="GlossTableDef">{}</text:p>
      </table:table-cell>
    </table:table-row>
"###,
                if gloss_occurrance.arrowed_state == ArrowedState::Arrowed {
                    r##"→"##
                } else {
                    ""
                },
                escape_fodt(lemma_unwrapped),
                escape_fodt(&gloss_unwrapped.def)
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

        let mut para_open = false;

        for w in gloss_occurrances {
            if let Some(ap) = appcrit_hash.get(&w.word.uuid) {
                appcrits_page.push(ap.clone());
            }

            match w.word.word_type {
                WordType::VerseLine => {
                    if para_open {
                        para_open = false;
                        res.push_str(
                            r###"
    </text:p>
"###,
                        );
                    }
                    if !is_verse_section {
                        res.push_str(
                            r###"
    <table:table table:name="VerseTable" table:style-name="VerseTable">
        <table:table-column table:style-name="VerseTable.A"/>
        <table:table-column table:style-name="VerseTable.B"/>
        <table:table-column table:style-name="VerseTable.C"/>
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
                WordType::WorkTitle => {
                    if para_open {
                        para_open = false;
                        res.push_str(
                            r###"
    </text:p>
"###,
                        );
                    }
                    res.push_str(
                        format!(
                            r###"
    <text:p text:style-name="WorkTitleCenter">{}</text:p>
    <text:p text:style-name="Standard"></text:p>
                        "###,
                            escape_fodt(&w.word.word)
                        )
                        .as_str(),
                    )
                }
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
                        escape_fodt(&w.word.word)
                    );
                    if is_verse_section {
                        verse_line.push_str(&s);
                    } else {
                        if !para_open {
                            para_open = true;
                            res.push_str(
                                r###"
    <text:p text:style-name="Standard">
"###,
                            );
                        }
                        res.push_str(&s);
                    }
                    prev_non_space = w.word.word == "<" || w.word.word == "[" || w.word.word == "(";
                }
                WordType::ParaWithIndent => {
                    if para_open {
                        res.push_str(
                            r###"
    </text:p>
"###,
                        );
                    }
                    para_open = true;
                    res.push_str(
                        r###"
    <text:p text:style-name="PIndented">
"###,
                    )
                }
                WordType::ParaNoIndent => {
                    if para_open {
                        res.push_str(
                            r###"
    </text:p>
"###,
                        );
                    }
                    para_open = true;
                    res.push_str(
                        r###"
    <text:p text:style-name="Standard">
"###,
                    )
                }
                WordType::SectionTitle => {
                    if para_open {
                        para_open = false;
                        res.push_str(
                            r###"
    </text:p>
"###,
                        );
                    }
                    res.push_str(
                        format!(
                            r###"
    <text:p text:style-name="P18">{}</text:p>
"###,
                            escape_fodt(&w.word.word)
                        )
                        .as_str(),
                    )
                }
                WordType::Section => {
                    let section_input = w.word.word.replace("[section]", "");

                    let matches = re.captures(&section_input);

                    let s = if let Some(matches) = matches {
                        let section = matches.get(1).unwrap().as_str();
                        let subsection = matches.get(2).unwrap().as_str();

                        //To Do: for the next three formats move space to start of line
                        if subsection == "1" {
                            format!(
                                r###" <text:span text:style-name="T1">{}</text:span> "###,
                                section
                            )
                        } else {
                            format!(
                                r###" <text:span text:style-name="T1">{}</text:span> "###,
                                subsection
                            )
                        }
                    } else {
                        format!(
                            r###" <text:span text:style-name="T1">{}</text:span> "###,
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
                        res.push_str(
                            r###"
    </table:table>
"###,
                        );
                    }

                    res.push_str(
                        format!(
                            r###"
    <text:p text:style-name="Standard">{}</text:p>
"###,
                            w.word.word
                        )
                        .as_str(),
                    );
                    if is_verse_section {
                        res.push_str(
                            r###"
    <table:table table:name="VerseTable" table:style-name="VerseTable">
        <table:table-column table:style-name="VerseTable.A"/>
        <table:table-column table:style-name="VerseTable.B"/>
        <table:table-column table:style-name="VerseTable.C"/>
"###,
                        );
                    }
                }
                WordType::InlineSpeaker => {
                    if is_verse_section {
                        verse_speaker = Some(w.word.word.clone());
                    } else {
                        if !para_open {
                            para_open = true;
                            res.push_str(
                                r###"
    <text:p text:style-name="Standard">
"###,
                            )
                        }
                        res.push_str(
                            format!(
                                r###"<text:span text:style-name="T1">{}</text:span> "###,
                                w.word.word
                            )
                            .as_str(),
                        );
                    }
                }
                WordType::InlineVerseSpeaker => {
                    verse_speaker = Some(w.word.word.clone());
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

            res.push_str(
                r###"
</table:table>
"###,
            );
        } else if para_open {
            res.push_str("</text:p>\n");
        }

        if !appcrits_page.is_empty() {
            res.push_str("<text:p></text:p>\n");
        }
        for ap in appcrits_page {
            res.push_str(format!("<text:p>{}</text:p>\n", escape_fodt(&ap)).as_str());
        }

        res
    }

    fn page_gloss_start(&self) -> String {
        String::from(
            r###"
    <text:p text:style-name="P1"/>
    <text:p text:style-name="P1"/>
    <table:table table:name="Table1" table:style-name="GlossTable">
      <table:table-column table:style-name="GlossTable.A"/>
      <table:table-column table:style-name="GlossTable.B"/>
      <table:table-column table:style-name="GlossTable.C"/>
"###,
        )
    }

    fn page_start(&self, _title: &str, _page_number: usize) -> String {
        String::from(
            r###"

            "###,
        )
    }

    fn page_end(&self) -> String {
        String::from(
            r###"
    </table:table>
    <text:p text:style-name="PageBreakStyle"/>
"###,
        )
    }

    fn document_end(&self) -> String {
        String::from(
            r###"<text:p text:style-name="P5"/>
  </office:text>
 </office:body>
</office:document>
"###,
        )
    }

    fn document_start(&self, title: &str, start_page: usize) -> String {
        let start = r###"<?xml version="1.0" encoding="UTF-8"?>

        <office:document xmlns:css3t="http://www.w3.org/TR/css3-text/" xmlns:grddl="http://www.w3.org/2003/g/data-view#" xmlns:xhtml="http://www.w3.org/1999/xhtml" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xforms="http://www.w3.org/2002/xforms" xmlns:dom="http://www.w3.org/2001/xml-events" xmlns:script="urn:oasis:names:tc:opendocument:xmlns:script:1.0" xmlns:form="urn:oasis:names:tc:opendocument:xmlns:form:1.0" xmlns:math="http://www.w3.org/1998/Math/MathML" xmlns:meta="urn:oasis:names:tc:opendocument:xmlns:meta:1.0" xmlns:loext="urn:org:documentfoundation:names:experimental:office:xmlns:loext:1.0" xmlns:field="urn:openoffice:names:experimental:ooo-ms-interop:xmlns:field:1.0" xmlns:number="urn:oasis:names:tc:opendocument:xmlns:datastyle:1.0" xmlns:officeooo="http://openoffice.org/2009/office" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" xmlns:chart="urn:oasis:names:tc:opendocument:xmlns:chart:1.0" xmlns:formx="urn:openoffice:names:experimental:ooxml-odf-interop:xmlns:form:1.0" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" xmlns:tableooo="http://openoffice.org/2009/table" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0" xmlns:rpt="http://openoffice.org/2005/report" xmlns:dr3d="urn:oasis:names:tc:opendocument:xmlns:dr3d:1.0" xmlns:of="urn:oasis:names:tc:opendocument:xmlns:of:1.2" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:calcext="urn:org:documentfoundation:names:experimental:calc:xmlns:calcext:1.0" xmlns:oooc="http://openoffice.org/2004/calc" xmlns:config="urn:oasis:names:tc:opendocument:xmlns:config:1.0" xmlns:ooo="http://openoffice.org/2004/office" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:drawooo="http://openoffice.org/2010/draw" xmlns:ooow="http://openoffice.org/2004/writer" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
         <office:meta><meta:creation-date>2021-03-08T17:40:14</meta:creation-date><meta:initial-creator>Jeremy March</meta:initial-creator><dc:language>en-US</dc:language><dc:creator>Jeremy March</dc:creator><dc:date>2021-04-08T20:17:25.947067138</dc:date><meta:editing-cycles>25</meta:editing-cycles><meta:editing-duration>P1DT5H15M2S</meta:editing-duration><meta:generator>LibreOffice/7.1.2.1$MacOSX_X86_64 LibreOffice_project/094b4116e8de6d2085e9b65d26912d6eac4c74a9</meta:generator><meta:document-statistic meta:table-count="1" meta:image-count="0" meta:object-count="0" meta:page-count="1" meta:paragraph-count="32" meta:word-count="249" meta:character-count="1664" meta:non-whitespace-character-count="1446"/><meta:user-defined meta:name="AppVersion">15.0000</meta:user-defined></office:meta>
         <office:settings>
          <config:config-item-set config:name="ooo:view-settings">
           <config:config-item config:name="ViewAreaTop" config:type="long">1803</config:config-item>
           <config:config-item config:name="ViewAreaLeft" config:type="long">0</config:config-item>
           <config:config-item config:name="ViewAreaWidth" config:type="long">55002</config:config-item>
           <config:config-item config:name="ViewAreaHeight" config:type="long">27141</config:config-item>
           <config:config-item config:name="ShowRedlineChanges" config:type="boolean">true</config:config-item>
           <config:config-item config:name="InBrowseMode" config:type="boolean">false</config:config-item>
           <config:config-item-map-indexed config:name="Views">
            <config:config-item-map-entry>
             <config:config-item config:name="ViewId" config:type="string">view2</config:config-item>
             <config:config-item config:name="ViewLeft" config:type="long">11763</config:config-item>
             <config:config-item config:name="ViewTop" config:type="long">12866</config:config-item>
             <config:config-item config:name="VisibleLeft" config:type="long">0</config:config-item>
             <config:config-item config:name="VisibleTop" config:type="long">1803</config:config-item>
             <config:config-item config:name="VisibleRight" config:type="long">55000</config:config-item>
             <config:config-item config:name="VisibleBottom" config:type="long">28942</config:config-item>
             <config:config-item config:name="ZoomType" config:type="short">0</config:config-item>
             <config:config-item config:name="ViewLayoutColumns" config:type="short">0</config:config-item>
             <config:config-item config:name="ViewLayoutBookMode" config:type="boolean">false</config:config-item>
             <config:config-item config:name="ZoomFactor" config:type="short">100</config:config-item>
             <config:config-item config:name="IsSelectedFrame" config:type="boolean">false</config:config-item>
             <config:config-item config:name="AnchoredTextOverflowLegacy" config:type="boolean">false</config:config-item>
            </config:config-item-map-entry>
           </config:config-item-map-indexed>
          </config:config-item-set>
          <config:config-item-set config:name="ooo:configuration-settings">
           <config:config-item config:name="PrintBlackFonts" config:type="boolean">false</config:config-item>
           <config:config-item config:name="PrintReversed" config:type="boolean">false</config:config-item>
           <config:config-item config:name="PrintAnnotationMode" config:type="short">0</config:config-item>
           <config:config-item config:name="PrintGraphics" config:type="boolean">true</config:config-item>
           <config:config-item config:name="EmbeddedDatabaseName" config:type="string"/>
           <config:config-item config:name="ProtectForm" config:type="boolean">false</config:config-item>
           <config:config-item config:name="PrintLeftPages" config:type="boolean">true</config:config-item>
           <config:config-item config:name="PrintProspect" config:type="boolean">false</config:config-item>
           <config:config-item config:name="PrintHiddenText" config:type="boolean">false</config:config-item>
           <config:config-item config:name="PrintRightPages" config:type="boolean">true</config:config-item>
           <config:config-item config:name="PrintFaxName" config:type="string"/>
           <config:config-item config:name="TabsRelativeToIndent" config:type="boolean">false</config:config-item>
           <config:config-item config:name="RedlineProtectionKey" config:type="base64Binary"/>
           <config:config-item config:name="PrintTextPlaceholder" config:type="boolean">false</config:config-item>
           <config:config-item config:name="AddFrameOffsets" config:type="boolean">true</config:config-item>
           <config:config-item config:name="FrameAutowidthWithMorePara" config:type="boolean">true</config:config-item>
           <config:config-item config:name="MathBaselineAlignment" config:type="boolean">true</config:config-item>
           <config:config-item config:name="ProtectBookmarks" config:type="boolean">false</config:config-item>
           <config:config-item config:name="IgnoreTabsAndBlanksForLineCalculation" config:type="boolean">true</config:config-item>
           <config:config-item config:name="ContinuousEndnotes" config:type="boolean">false</config:config-item>
           <config:config-item config:name="FieldAutoUpdate" config:type="boolean">true</config:config-item>
           <config:config-item config:name="EmptyDbFieldHidesPara" config:type="boolean">true</config:config-item>
           <config:config-item config:name="ApplyParagraphMarkFormatToNumbering" config:type="boolean">true</config:config-item>
           <config:config-item config:name="PrintEmptyPages" config:type="boolean">false</config:config-item>
           <config:config-item config:name="AddParaLineSpacingToTableCells" config:type="boolean">true</config:config-item>
           <config:config-item config:name="TabOverMargin" config:type="boolean">true</config:config-item>
           <config:config-item config:name="EmbedAsianScriptFonts" config:type="boolean">true</config:config-item>
           <config:config-item config:name="EmbedLatinScriptFonts" config:type="boolean">true</config:config-item>
           <config:config-item config:name="DisableOffPagePositioning" config:type="boolean">true</config:config-item>
           <config:config-item config:name="EmbedOnlyUsedFonts" config:type="boolean">false</config:config-item>
           <config:config-item config:name="PrintControls" config:type="boolean">true</config:config-item>
           <config:config-item config:name="SaveThumbnail" config:type="boolean">true</config:config-item>
           <config:config-item config:name="EmbedFonts" config:type="boolean">false</config:config-item>
           <config:config-item config:name="MsWordCompMinLineHeightByFly" config:type="boolean">false</config:config-item>
           <config:config-item config:name="SurroundTextWrapSmall" config:type="boolean">true</config:config-item>
           <config:config-item config:name="BackgroundParaOverDrawings" config:type="boolean">true</config:config-item>
           <config:config-item config:name="ClippedPictures" config:type="boolean">true</config:config-item>
           <config:config-item config:name="FloattableNomargins" config:type="boolean">true</config:config-item>
           <config:config-item config:name="UnbreakableNumberings" config:type="boolean">true</config:config-item>
           <config:config-item config:name="EmbedSystemFonts" config:type="boolean">false</config:config-item>
           <config:config-item config:name="TabOverflow" config:type="boolean">true</config:config-item>
           <config:config-item config:name="PrintTables" config:type="boolean">true</config:config-item>
           <config:config-item config:name="PrintDrawings" config:type="boolean">true</config:config-item>
           <config:config-item config:name="ConsiderTextWrapOnObjPos" config:type="boolean">true</config:config-item>
           <config:config-item config:name="PrintSingleJobs" config:type="boolean">false</config:config-item>
           <config:config-item config:name="SmallCapsPercentage66" config:type="boolean">false</config:config-item>
           <config:config-item config:name="CollapseEmptyCellPara" config:type="boolean">true</config:config-item>
           <config:config-item config:name="HeaderSpacingBelowLastPara" config:type="boolean">true</config:config-item>
           <config:config-item config:name="RsidRoot" config:type="int">1906756</config:config-item>
           <config:config-item config:name="PrinterSetup" config:type="base64Binary"/>
           <config:config-item config:name="CurrentDatabaseCommand" config:type="string"/>
           <config:config-item config:name="AlignTabStopPosition" config:type="boolean">true</config:config-item>
           <config:config-item config:name="ClipAsCharacterAnchoredWriterFlyFrames" config:type="boolean">false</config:config-item>
           <config:config-item config:name="DoNotCaptureDrawObjsOnPage" config:type="boolean">false</config:config-item>
           <config:config-item config:name="SaveGlobalDocumentLinks" config:type="boolean">false</config:config-item>
           <config:config-item config:name="CurrentDatabaseCommandType" config:type="int">0</config:config-item>
           <config:config-item config:name="LoadReadonly" config:type="boolean">false</config:config-item>
           <config:config-item config:name="DoNotResetParaAttrsForNumFont" config:type="boolean">false</config:config-item>
           <config:config-item config:name="StylesNoDefault" config:type="boolean">false</config:config-item>
           <config:config-item config:name="LinkUpdateMode" config:type="short">1</config:config-item>
           <config:config-item config:name="DoNotJustifyLinesWithManualBreak" config:type="boolean">false</config:config-item>
           <config:config-item config:name="PropLineSpacingShrinksFirstLine" config:type="boolean">true</config:config-item>
           <config:config-item config:name="TabAtLeftIndentForParagraphsInList" config:type="boolean">true</config:config-item>
           <config:config-item config:name="ProtectFields" config:type="boolean">false</config:config-item>
           <config:config-item config:name="UnxForceZeroExtLeading" config:type="boolean">false</config:config-item>
           <config:config-item config:name="CurrentDatabaseDataSource" config:type="string"/>
           <config:config-item config:name="UseFormerTextWrapping" config:type="boolean">false</config:config-item>
           <config:config-item config:name="PrintPaperFromSetup" config:type="boolean">false</config:config-item>
           <config:config-item config:name="UseFormerLineSpacing" config:type="boolean">false</config:config-item>
           <config:config-item config:name="AllowPrintJobCancel" config:type="boolean">true</config:config-item>
           <config:config-item config:name="SubtractFlysAnchoredAtFlys" config:type="boolean">false</config:config-item>
           <config:config-item config:name="AddParaSpacingToTableCells" config:type="boolean">true</config:config-item>
           <config:config-item config:name="AddExternalLeading" config:type="boolean">true</config:config-item>
           <config:config-item config:name="Rsid" config:type="int">2513019</config:config-item>
           <config:config-item config:name="AddVerticalFrameOffsets" config:type="boolean">true</config:config-item>
           <config:config-item config:name="TreatSingleColumnBreakAsPageBreak" config:type="boolean">true</config:config-item>
           <config:config-item config:name="IsLabelDocument" config:type="boolean">false</config:config-item>
           <config:config-item config:name="MsWordCompTrailingBlanks" config:type="boolean">true</config:config-item>
           <config:config-item config:name="PrinterPaperFromSetup" config:type="boolean">false</config:config-item>
           <config:config-item config:name="IgnoreFirstLineIndentInNumbering" config:type="boolean">false</config:config-item>
           <config:config-item config:name="PrintPageBackground" config:type="boolean">true</config:config-item>
           <config:config-item config:name="OutlineLevelYieldsNumbering" config:type="boolean">false</config:config-item>
           <config:config-item config:name="PrinterName" config:type="string"/>
           <config:config-item config:name="IsKernAsianPunctuation" config:type="boolean">false</config:config-item>
           <config:config-item config:name="PrinterIndependentLayout" config:type="string">high-resolution</config:config-item>
           <config:config-item config:name="TableRowKeep" config:type="boolean">true</config:config-item>
           <config:config-item config:name="UpdateFromTemplate" config:type="boolean">true</config:config-item>
           <config:config-item config:name="EmbedComplexScriptFonts" config:type="boolean">true</config:config-item>
           <config:config-item config:name="UseOldPrinterMetrics" config:type="boolean">false</config:config-item>
           <config:config-item config:name="InvertBorderSpacing" config:type="boolean">true</config:config-item>
           <config:config-item config:name="PrintProspectRTL" config:type="boolean">false</config:config-item>
           <config:config-item config:name="ApplyUserData" config:type="boolean">true</config:config-item>
           <config:config-item config:name="AddParaTableSpacingAtStart" config:type="boolean">true</config:config-item>
           <config:config-item config:name="SaveVersionOnClose" config:type="boolean">false</config:config-item>
           <config:config-item config:name="CharacterCompressionType" config:type="short">0</config:config-item>
           <config:config-item config:name="UseOldNumbering" config:type="boolean">false</config:config-item>
           <config:config-item config:name="UseFormerObjectPositioning" config:type="boolean">false</config:config-item>
           <config:config-item config:name="ChartAutoUpdate" config:type="boolean">true</config:config-item>
           <config:config-item config:name="AddParaTableSpacing" config:type="boolean">false</config:config-item>
          </config:config-item-set>
         </office:settings>
         <office:scripts>
          <office:script script:language="ooo:Basic">
           <ooo:libraries xmlns:ooo="http://openoffice.org/2004/office" xmlns:xlink="http://www.w3.org/1999/xlink">
            <ooo:library-embedded ooo:name="Standard"/>
           </ooo:libraries>
          </office:script>
         </office:scripts>
         <office:font-face-decls>
          <style:font-face style:name="IFAO-Grec Unicode" svg:font-family="&apos;IFAO-Grec Unicode&apos;"/>
          <style:font-face style:name="New Athena Unicode" svg:font-family="&apos;New Athena Unicode&apos;"/>
          <style:font-face style:name="Arial Unicode MS" svg:font-family="&apos;Arial Unicode MS&apos;" style:font-family-generic="system" style:font-pitch="variable"/>
          <style:font-face style:name="PingFang SC" svg:font-family="&apos;PingFang SC&apos;" style:font-family-generic="system" style:font-pitch="variable"/>
          <style:font-face style:name="Songti SC" svg:font-family="&apos;Songti SC&apos;" style:font-family-generic="system" style:font-pitch="variable"/>
         </office:font-face-decls>
         <office:styles>
          <style:default-style style:family="graphic">
           <style:graphic-properties svg:stroke-color="#3465a4" draw:fill-color="#729fcf" fo:wrap-option="no-wrap" draw:shadow-offset-x="0.1181in" draw:shadow-offset-y="0.1181in" draw:start-line-spacing-horizontal="0.1114in" draw:start-line-spacing-vertical="0.1114in" draw:end-line-spacing-horizontal="0.1114in" draw:end-line-spacing-vertical="0.1114in" style:flow-with-text="false"/>
           <style:paragraph-properties style:text-autospace="ideograph-alpha" style:line-break="strict" style:writing-mode="lr-tb" style:font-independent-line-spacing="false">
            <style:tab-stops/>
           </style:paragraph-properties>
           <style:text-properties style:use-window-font-color="true" loext:opacity="0%" style:font-name="IFAO-Grec Unicode" fo:font-size="12pt" fo:language="en" fo:country="US" style:letter-kerning="true" style:font-name-asian="Songti SC" style:font-size-asian="12pt" style:language-asian="zh" style:country-asian="CN" style:font-name-complex="Arial Unicode MS" style:font-size-complex="12pt" style:language-complex="hi" style:country-complex="IN"/>
          </style:default-style>
          <style:default-style style:family="paragraph">
           <style:paragraph-properties fo:hyphenation-ladder-count="no-limit" style:text-autospace="ideograph-alpha" style:punctuation-wrap="hanging" style:line-break="strict" style:tab-stop-distance="0.4925in" style:writing-mode="lr-tb"/>
           <style:text-properties style:use-window-font-color="true" loext:opacity="0%" style:font-name="IFAO-Grec Unicode" fo:font-size="12pt" fo:language="en" fo:country="US" style:letter-kerning="true" style:font-name-asian="Songti SC" style:font-size-asian="12pt" style:language-asian="zh" style:country-asian="CN" style:font-name-complex="Arial Unicode MS" style:font-size-complex="12pt" style:language-complex="hi" style:country-complex="IN" fo:hyphenate="false" fo:hyphenation-remain-char-count="2" fo:hyphenation-push-char-count="2" loext:hyphenation-no-caps="false"/>
          </style:default-style>
          <style:default-style style:family="table">
           <style:table-properties table:border-model="collapsing"/>
          </style:default-style>
          <style:default-style style:family="table-row">
           <style:table-row-properties fo:keep-together="auto"/>
          </style:default-style>
          <style:style style:name="Standard" style:family="paragraph" style:default-outline-level="" style:class="text">
           <style:paragraph-properties fo:margin-left="0in" fo:margin-right="0in" fo:margin-top="0in" fo:margin-bottom="0in" fo:line-height="130%" style:contextual-spacing="false" fo:text-align="justify" style:justify-single-word="false" fo:orphans="2" fo:widows="2" fo:hyphenation-ladder-count="no-limit" fo:text-indent="0in" style:auto-text-indent="false" style:writing-mode="lr-tb">
            <style:tab-stops/>
           </style:paragraph-properties>
           <style:text-properties style:use-window-font-color="true" loext:opacity="0%" style:font-name="IFAO-Grec Unicode" fo:font-family="&apos;IFAO-Grec Unicode&apos;" fo:font-size="12pt" fo:language="en" fo:country="US" style:letter-kerning="true" style:font-name-asian="Songti SC" style:font-family-asian="&apos;Songti SC&apos;" style:font-family-generic-asian="system" style:font-pitch-asian="variable" style:font-size-asian="12pt" style:language-asian="zh" style:country-asian="CN" style:font-name-complex="IFAO-Grec Unicode" style:font-family-complex="&apos;IFAO-Grec Unicode&apos;" style:font-family-generic-complex="system" style:font-pitch-complex="variable" style:font-size-complex="12pt" style:language-complex="hi" style:country-complex="IN" fo:hyphenate="false" fo:hyphenation-remain-char-count="2" fo:hyphenation-push-char-count="2" loext:hyphenation-no-caps="false"/>
          </style:style>

          <style:style style:name="Heading" style:family="paragraph" style:parent-style-name="Standard" style:next-style-name="Text_20_body" style:default-outline-level="" style:class="text">
           <style:paragraph-properties fo:margin-top="0.1665in" fo:margin-bottom="0.0835in" style:contextual-spacing="false" fo:keep-with-next="always"/>
           <style:text-properties style:font-name="IFAO-Grec Unicode" fo:font-family="&apos;IFAO-Grec Unicode&apos;" fo:font-size="14pt" style:font-name-asian="PingFang SC" style:font-family-asian="&apos;PingFang SC&apos;" style:font-family-generic-asian="system" style:font-pitch-asian="variable" style:font-size-asian="14pt" style:font-name-complex="Arial Unicode MS" style:font-family-complex="&apos;Arial Unicode MS&apos;" style:font-family-generic-complex="system" style:font-pitch-complex="variable" style:font-size-complex="14pt"/>
          </style:style>
          <style:style style:name="Text_20_body" style:display-name="Text body" style:family="paragraph" style:parent-style-name="Standard" style:default-outline-level="" style:class="text">
           <style:paragraph-properties fo:margin-top="0in" fo:margin-bottom="0in" style:contextual-spacing="false" fo:line-height="100%"/>
          </style:style>
          <style:style style:name="List" style:family="paragraph" style:parent-style-name="Text_20_body" style:default-outline-level="" style:class="list">
           <style:text-properties style:font-name="IFAO-Grec Unicode" fo:font-family="&apos;IFAO-Grec Unicode&apos;" style:font-name-complex="Arial Unicode MS" style:font-family-complex="&apos;Arial Unicode MS&apos;" style:font-family-generic-complex="system" style:font-pitch-complex="variable"/>
          </style:style>
          <style:style style:name="Caption" style:family="paragraph" style:parent-style-name="Standard" style:default-outline-level="" style:class="extra">
           <style:paragraph-properties fo:margin-top="0.0835in" fo:margin-bottom="0.0835in" style:contextual-spacing="false" text:number-lines="false" text:line-number="0"/>
           <style:text-properties style:font-name="IFAO-Grec Unicode" fo:font-family="&apos;IFAO-Grec Unicode&apos;" fo:font-size="12pt" fo:font-style="italic" style:font-size-asian="12pt" style:font-style-asian="italic" style:font-name-complex="Arial Unicode MS" style:font-family-complex="&apos;Arial Unicode MS&apos;" style:font-family-generic-complex="system" style:font-pitch-complex="variable" style:font-size-complex="12pt" style:font-style-complex="italic"/>
          </style:style>
          <style:style style:name="Index" style:family="paragraph" style:parent-style-name="Standard" style:default-outline-level="" style:class="index">
           <style:paragraph-properties text:number-lines="false" text:line-number="0"/>
           <style:text-properties style:font-name="IFAO-Grec Unicode" fo:font-family="&apos;IFAO-Grec Unicode&apos;" style:font-name-complex="Arial Unicode MS" style:font-family-complex="&apos;Arial Unicode MS&apos;" style:font-family-generic-complex="system" style:font-pitch-complex="variable"/>
          </style:style>
          <style:style style:name="GlossTableLemma" style:family="paragraph" style:parent-style-name="Standard" style:default-outline-level="">
           <style:paragraph-properties fo:margin-left="0.4in" fo:margin-right="0in" fo:line-height="100%" fo:text-align="start" style:justify-single-word="false" fo:orphans="0" fo:widows="0" fo:text-indent="-0.4in" style:auto-text-indent="false" style:writing-mode="lr-tb">
            <style:tab-stops/>
           </style:paragraph-properties>
          </style:style>
          <style:style style:name="GlossTableDef" style:family="paragraph" style:parent-style-name="Text_20_body" style:default-outline-level="">
           <style:paragraph-properties fo:margin-top="0in" fo:margin-bottom="0in" style:contextual-spacing="false" fo:line-height="100%" fo:text-align="start" style:justify-single-word="false" fo:orphans="0" fo:widows="0" style:writing-mode="lr-tb"/>
          </style:style>
          <style:style style:name="GlossInlineSections" style:family="paragraph" style:parent-style-name="Standard" style:default-outline-level="">
           <style:paragraph-properties fo:line-height="150%" fo:text-align="start" style:justify-single-word="false" style:writing-mode="lr-tb"/>
          </style:style>
          <style:style style:name="Table_20_Contents" style:display-name="Table Contents" style:family="paragraph" style:parent-style-name="Standard" style:class="extra">
           <style:paragraph-properties fo:orphans="0" fo:widows="0" text:number-lines="false" text:line-number="0"/>
          </style:style>
          <style:style style:name="PIndented" style:family="paragraph" style:parent-style-name="Standard">
           <style:paragraph-properties fo:text-align="justify" style:justify-single-word="false" fo:text-indent="0.5in" style:auto-text-indent="false"/>
           <style:text-properties officeooo:paragraph-rsid="002949d7"/>
          </style:style>
          <style:style style:name="Hanging_20_indent" style:display-name="Hanging indent" style:family="paragraph" style:parent-style-name="Text_20_body" style:class="text">
           <style:paragraph-properties fo:margin-left="0.3937in" fo:margin-right="0in" fo:text-indent="-0.1965in" style:auto-text-indent="false">
            <style:tab-stops>
             <style:tab-stop style:position="0in"/>
            </style:tab-stops>
           </style:paragraph-properties>
          </style:style>
          <text:outline-style style:name="Outline">
           <text:outline-level-style text:level="1" style:num-format="">
            <style:list-level-properties text:list-level-position-and-space-mode="label-alignment">
             <style:list-level-label-alignment text:label-followed-by="listtab"/>
            </style:list-level-properties>
           </text:outline-level-style>
           <text:outline-level-style text:level="2" style:num-format="">
            <style:list-level-properties text:list-level-position-and-space-mode="label-alignment">
             <style:list-level-label-alignment text:label-followed-by="listtab"/>
            </style:list-level-properties>
           </text:outline-level-style>
           <text:outline-level-style text:level="3" style:num-format="">
            <style:list-level-properties text:list-level-position-and-space-mode="label-alignment">
             <style:list-level-label-alignment text:label-followed-by="listtab"/>
            </style:list-level-properties>
           </text:outline-level-style>
           <text:outline-level-style text:level="4" style:num-format="">
            <style:list-level-properties text:list-level-position-and-space-mode="label-alignment">
             <style:list-level-label-alignment text:label-followed-by="listtab"/>
            </style:list-level-properties>
           </text:outline-level-style>
           <text:outline-level-style text:level="5" style:num-format="">
            <style:list-level-properties text:list-level-position-and-space-mode="label-alignment">
             <style:list-level-label-alignment text:label-followed-by="listtab"/>
            </style:list-level-properties>
           </text:outline-level-style>
           <text:outline-level-style text:level="6" style:num-format="">
            <style:list-level-properties text:list-level-position-and-space-mode="label-alignment">
             <style:list-level-label-alignment text:label-followed-by="listtab"/>
            </style:list-level-properties>
           </text:outline-level-style>
           <text:outline-level-style text:level="7" style:num-format="">
            <style:list-level-properties text:list-level-position-and-space-mode="label-alignment">
             <style:list-level-label-alignment text:label-followed-by="listtab"/>
            </style:list-level-properties>
           </text:outline-level-style>
           <text:outline-level-style text:level="8" style:num-format="">
            <style:list-level-properties text:list-level-position-and-space-mode="label-alignment">
             <style:list-level-label-alignment text:label-followed-by="listtab"/>
            </style:list-level-properties>
           </text:outline-level-style>
           <text:outline-level-style text:level="9" style:num-format="">
            <style:list-level-properties text:list-level-position-and-space-mode="label-alignment">
             <style:list-level-label-alignment text:label-followed-by="listtab"/>
            </style:list-level-properties>
           </text:outline-level-style>
           <text:outline-level-style text:level="10" style:num-format="">
            <style:list-level-properties text:list-level-position-and-space-mode="label-alignment">
             <style:list-level-label-alignment text:label-followed-by="listtab"/>
            </style:list-level-properties>
           </text:outline-level-style>
          </text:outline-style>
          <text:notes-configuration text:note-class="footnote" style:num-format="1" text:start-value="0" text:footnotes-position="page" text:start-numbering-at="document"/>
          <text:notes-configuration text:note-class="endnote" style:num-format="i" text:start-value="0"/>
          <text:linenumbering-configuration text:number-lines="false" text:offset="0.1965in" style:num-format="1" text:number-position="left" text:increment="5"/>
          <style:default-page-layout>
           <style:page-layout-properties style:writing-mode="lr-tb" style:layout-grid-standard-mode="true"/>
          </style:default-page-layout>
         </office:styles>
         <office:automatic-styles>
          <style:style style:name="GlossTable" style:family="table">
           <style:table-properties style:width="7.1799in" fo:margin-left="-0.2597in" fo:margin-top="0in" fo:margin-bottom="0in" table:align="left" fo:background-color="transparent" style:may-break-between-rows="false" style:writing-mode="lr-tb">
            <style:background-image/>
           </style:table-properties>
          </style:style>
          <style:style style:name="GlossTable.A" style:family="table-column">
           <style:table-column-properties style:column-width="0.2563in"/>
          </style:style>
          <style:style style:name="GlossTable.B" style:family="table-column">
           <style:table-column-properties style:column-width="3.4236in"/>
          </style:style>
          <style:style style:name="GlossTable.C" style:family="table-column">
           <style:table-column-properties style:column-width="3.5in"/>
          </style:style>
          <style:style style:name="GlossTableCell" style:family="table-cell">
           <style:table-cell-properties fo:padding-left="0in" fo:padding-right="0.1201in" fo:padding-top="0.1097in" fo:padding-bottom="0in" fo:border="none"/>
          </style:style>
          <style:style style:name="GlossTableRow" style:family="table-row">
           <style:table-row-properties fo:keep-together="always"/>
          </style:style>
          <style:style style:name="VerseTable" style:family="table">
           <style:table-properties style:width="6.925in" style:may-break-between-rows="false" table:align="margins" style:writing-mode="lr-tb"/>
          </style:style>
          <style:style style:name="VerseTable.A" style:family="table-column">
           <style:table-column-properties style:column-width="1.2139in" style:rel-column-width="1748*"/>
          </style:style>
          <style:style style:name="VerseTable.B" style:family="table-column">
           <style:table-column-properties style:column-width="4.0625in" style:rel-column-width="5850*"/>
          </style:style>
          <style:style style:name="VerseTable.C" style:family="table-column">
           <style:table-column-properties style:column-width="1.6486in" style:rel-column-width="2374*"/>
          </style:style>
          <style:style style:name="VerseTable.A1" style:family="table-cell">
           <style:table-cell-properties fo:padding="0.0201in" fo:border="none" style:writing-mode="page"/>
          </style:style>
          <style:style style:name="P1" style:family="paragraph" style:parent-style-name="GlossInlineSections">
           <style:paragraph-properties fo:text-align="justify" style:justify-single-word="false"/>
          </style:style>
          <style:style style:name="P2" style:family="paragraph" style:parent-style-name="GlossTableDef">
           <style:paragraph-properties fo:orphans="0" fo:widows="0"/>
          </style:style>
          <style:style style:name="P3" style:family="paragraph" style:parent-style-name="GlossTableLemma">
           <style:paragraph-properties fo:margin-left="0.5in" fo:margin-right="0in" fo:line-height="100%" fo:text-align="start" style:justify-single-word="false" fo:orphans="0" fo:widows="0" fo:text-indent="-0.5in" style:auto-text-indent="false" style:writing-mode="lr-tb"/>
          </style:style>
          <style:style style:name="P4" style:family="paragraph" style:parent-style-name="GlossTableLemma">
           <style:paragraph-properties fo:margin-left="0.5in" fo:margin-right="0in" fo:line-height="100%" fo:text-align="end" style:justify-single-word="false" fo:orphans="0" fo:widows="0" fo:text-indent="-0.5in" style:auto-text-indent="false" style:writing-mode="lr-tb"/>
          </style:style>
          <style:style style:name="P5" style:family="paragraph" style:parent-style-name="Standard">
           <style:paragraph-properties fo:line-height="150%" fo:text-align="start" style:justify-single-word="false" style:writing-mode="lr-tb"/>
          </style:style>
          <style:style style:name="WorkTitleCenter" style:family="paragraph" style:parent-style-name="Standard">
           <style:paragraph-properties fo:line-height="150%" fo:text-align="center" style:justify-single-word="false" style:writing-mode="lr-tb"/>
           <style:text-properties style:font-name="IFAO-Grec Unicode" fo:font-weight="bold" style:font-weight-asian="bold" style:font-weight-complex="bold"/>
          </style:style>
          <style:style style:name="P7" style:family="paragraph" style:parent-style-name="Standard">
           <style:paragraph-properties fo:line-height="150%" fo:text-align="start" style:justify-single-word="false" style:writing-mode="lr-tb"/>
           <style:text-properties style:font-name="IFAO-Grec Unicode"/>
          </style:style>
          <style:style style:name="P8" style:family="paragraph" style:parent-style-name="GlossTableLemma">
           <style:paragraph-properties fo:margin-left="0in" fo:margin-right="0in" fo:line-height="100%" fo:text-align="end" style:justify-single-word="false" fo:orphans="0" fo:widows="0" fo:text-indent="0in" style:auto-text-indent="false" style:writing-mode="lr-tb"/>
          </style:style>
          <style:style style:name="PageBreakStyle" style:family="paragraph" style:parent-style-name="Standard">
           <style:paragraph-properties fo:text-align="justify" style:justify-single-word="false" fo:break-before="page"/>
           <style:text-properties officeooo:paragraph-rsid="0026e103"/>
          </style:style>
          <style:style style:name="T1" style:family="text">
           <style:text-properties style:font-name="IFAO-Grec Unicode" fo:font-weight="bold" style:font-weight-asian="bold" style:font-weight-complex="bold"/>
          </style:style>
          <style:style style:name="T2" style:family="text">
           <style:text-properties fo:font-style="italic" style:font-style-asian="italic" style:font-style-complex="italic"/>
          </style:style>
          <style:style style:name="T3" style:family="text">
           <style:text-properties style:text-position="super 58%"/>
          </style:style>
          <style:style style:name="P18" style:family="paragraph" style:parent-style-name="GlossInlineSections">
           <style:paragraph-properties fo:text-align="center" style:justify-single-word="false"/>
           <style:text-properties officeooo:rsid="00283989" officeooo:paragraph-rsid="00283989"/>
          </style:style>
          <style:style style:name="FooterStyle" style:family="paragraph" style:parent-style-name="Standard">
           <style:paragraph-properties fo:text-align="center" style:justify-single-word="false"/>
           <style:text-properties officeooo:rsid="00283989" officeooo:paragraph-rsid="00283989"/>
          </style:style>
          <style:style style:name="HeaderRight" style:family="paragraph" style:parent-style-name="Header">
           <style:paragraph-properties fo:text-align="end" style:justify-single-word="false"/>
           <style:text-properties officeooo:rsid="00283989" officeooo:paragraph-rsid="00283989"/>
          </style:style>
          <style:style style:name="HeaderLeft" style:family="paragraph" style:parent-style-name="Header">
           <style:text-properties officeooo:rsid="00283989" officeooo:paragraph-rsid="00283989"/>
          </style:style>
          <style:page-layout style:name="pm1">
           <style:page-layout-properties fo:page-width="8.5in" fo:page-height="11in" style:num-format="1" style:print-orientation="portrait" fo:margin-top="0.7874in" fo:margin-bottom="0.7874in" fo:margin-left="0.7874in" fo:margin-right="0.7874in" style:writing-mode="lr-tb" style:layout-grid-color="#c0c0c0" style:layout-grid-lines="136" style:layout-grid-base-height="0.0693in" style:layout-grid-ruby-height="0in" style:layout-grid-mode="none" style:layout-grid-ruby-below="false" style:layout-grid-print="false" style:layout-grid-display="false" style:layout-grid-base-width="0.1665in" style:layout-grid-snap-to="true" style:footnote-max-height="0in">
            <style:footnote-sep style:width="0.0071in" style:distance-before-sep="0.0398in" style:distance-after-sep="0.0398in" style:line-style="solid" style:adjustment="left" style:rel-width="25%" style:color="#000000"/>
           </style:page-layout-properties>
           <style:header-style/>
           <style:footer-style/>
          </style:page-layout>
         </office:automatic-styles>
         <office:master-styles>
          <style:master-page style:name="Standard" style:page-layout-name="pm1">
          <style:header>
           <text:p text:style-name="HeaderRight">%MAIN_TITLE%</text:p>
          </style:header>
          <style:header-left>
           <text:p text:style-name="HeaderLeft">LGI - UPPER LEVEL GREEK</text:p>
          </style:header-left>
          <style:header-first>
           <text:p text:style-name="HeaderLeft"></text:p>
          </style:header-first>
          <style:footer>
          <text:p text:style-name="FooterStyle"><text:bookmark-start text:name="PageNumWizard_FOOTER_Default Page Style1"/><text:page-number text:select-page="current">1</text:page-number><text:bookmark-end text:name="PageNumWizard_FOOTER_Default Page Style1"/></text:p>
          </style:footer>
          <style:footer-first>
           <text:p text:style-name="FooterStyle"><text:bookmark-start text:name="PageNumWizard_FOOTER_Default Page Style1"/><text:page-number text:select-page="current">1</text:page-number><text:bookmark-end text:name="PageNumWizard_FOOTER_Default Page Style1"/></text:p>
          </style:footer-first>
         </style:master-page>
         </office:master-styles>
         <office:body>
          <office:text>
           <text:sequence-decls>
            <text:sequence-decl text:display-outline-level="0" text:name="Illustration"/>
            <text:sequence-decl text:display-outline-level="0" text:name="Table"/>
            <text:sequence-decl text:display-outline-level="0" text:name="Text"/>
            <text:sequence-decl text:display-outline-level="0" text:name="Drawing"/>
            <text:sequence-decl text:display-outline-level="0" text:name="Figure"/>
           </text:sequence-decls>
"###;

        start
            .replace("%MAIN_TITLE%", title)
            .replace("%PAGE_NUM%", start_page.to_string().as_str())
    }

    fn make_index(&self, _arrowed_words_index: &[ArrowedWordsIndex]) -> String {
        /*
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
        */
        //latex
        String::from("")
    }

    fn blank_page(&self) -> String {
        String::from(
            r##"
<text:p text:style-name="PageBreakStyle"/>
"##,
        )
    }
}
