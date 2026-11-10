use crate::glosses::Gloss;
use crate::{GlossArrow, GlossUuid, GlosserError, Sequence, WordUuid};
use uuid::Uuid;

//the user editable fields of a gloss
pub struct GlossEdit<'a> {
    pub lemma: &'a str,
    pub sort_key: &'a str,
    pub pos: &'a str,
    pub def: &'a str,
    pub note: &'a str,
}

impl GlossEdit<'_> {
    fn check(&self) -> Result<(), GlosserError> {
        if self.lemma.trim().is_empty() || self.pos.trim().is_empty() || self.def.trim().is_empty()
        {
            return Err(GlosserError::GlossMissingField);
        }
        Ok(())
    }
}

impl Sequence {
    pub fn update_gloss(
        &mut self,
        gloss_uuid: GlossUuid,
        edit: &GlossEdit,
        updated: &str,
        updated_user: &str,
    ) -> Result<(), GlosserError> {
        edit.check()?;
        for glosses in &mut self.glosses {
            for gloss in &mut glosses.gloss {
                if gloss.uuid == gloss_uuid {
                    gloss.lemma = edit.lemma.to_string();
                    gloss.sort_key = edit.sort_key.to_string();
                    gloss.pos = edit.pos.to_string();
                    gloss.def = edit.def.to_string();
                    gloss.note = edit.note.to_string();
                    gloss.updated = updated.to_string();
                    gloss.updated_user = updated_user.to_string();
                    return Ok(());
                }
            }
        }
        Err(GlosserError::UpdateGlossNotFound)
    }

    //new glosses are added to the sequence's first gloss file
    pub fn add_gloss(
        &mut self,
        edit: &GlossEdit,
        updated: &str,
        updated_user: &str,
    ) -> Result<GlossUuid, GlosserError> {
        edit.check()?;
        let Some(glosses) = self.glosses.first_mut() else {
            return Err(GlosserError::AddGlossNoGlossFile);
        };
        let uuid = Uuid::new_v4();
        glosses.gloss.push(Gloss {
            uuid,
            parent_id: None,
            lemma: edit.lemma.to_string(),
            sort_key: edit.sort_key.to_string(),
            morpheus_key: None,
            def: edit.def.to_string(),
            pos: edit.pos.to_string(),
            unit: 0,
            note: edit.note.to_string(),
            updated: updated.to_string(),
            status: 1,
            updated_user: updated_user.to_string(),
        });
        Ok(uuid)
    }

    pub fn set_gloss(
        &mut self,
        word_uuid: WordUuid,
        gloss_uuid: Option<GlossUuid>,
    ) -> Result<(), GlosserError> {
        let mut found = false;
        //the word must not be arrowed when changing its gloss
        for a in &self.sequence_description.arrowed_words {
            if a.word_uuid == word_uuid {
                return Err(GlosserError::SetGlossWordIsArrowed);
            }
        }

        'outer: for text in &mut self.texts {
            for word in &mut text.words {
                if word.uuid == word_uuid {
                    word.gloss_uuid = gloss_uuid;
                    found = true;
                    break 'outer;
                }
            }
        }
        if found {
            Ok(())
        } else {
            Err(GlosserError::SetGlossWordNotFound)
        }
    }

    pub fn arrow_word(
        &mut self,
        word_uuid: WordUuid,
        gloss_uuid: GlossUuid,
        add: bool,
    ) -> Result<(), GlosserError> {
        //check that word_uuid is actually set to gloss_uuid in the text
        'outer: for text in &mut self.texts {
            for word in &mut text.words {
                if word.uuid == word_uuid {
                    if word.gloss_uuid != Some(gloss_uuid) {
                        return Err(GlosserError::ArrowWordWrongGloss);
                    } else {
                        break 'outer;
                    }
                }
            }
        }
        let mut found = false;
        if !add {
            //if add is false, we can always remove an arrow.
            //only unarrows if word_uuid AND gloss_uuid match
            let orig_len = self.sequence_description.arrowed_words.len();
            self.sequence_description
                .arrowed_words
                .retain(|a| !(a.word_uuid == word_uuid && a.gloss_uuid == gloss_uuid));
            if self.sequence_description.arrowed_words.len() == orig_len - 1 {
                found = true;
            }
        } else {
            //if add is true:
            //we have to be sure this word_uuid isn't already arrowed
            //AND we have to be sure this gloss isn't already arrowed on another word_uuid,
            for a in &self.sequence_description.arrowed_words {
                if a.word_uuid == word_uuid {
                    return Err(GlosserError::ArrowWordWordAlreadyArrowed);
                } else if a.gloss_uuid == gloss_uuid {
                    return Err(GlosserError::ArrowWordGlossAlreadyArrowed);
                }
            }
            self.sequence_description.arrowed_words.push(GlossArrow {
                gloss_uuid,
                word_uuid,
            });
            found = true;
        }
        if found {
            Ok(())
        } else {
            Err(GlosserError::ArrowWordNotFound)
        }
    }
}
