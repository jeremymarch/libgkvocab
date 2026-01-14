# libgkvocabdb - a new implementation of Glosser

This is a new implementation of Glosser as a library without dependency on PostgreSQL.  The logic which was previously expressed in SQL is now implemented in custom algorithms.  Gloss sorting is handled by the ICU4X library.  It also includes work-in-progress of a new lemmatizer based on Morpheus.

The data now lives as xml files in a git repo: gkvocab_data.
