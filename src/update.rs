use crate::{GlossArrow, GlossUuid, GlosserError, Sequence, WordUuid};

impl Sequence {
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
