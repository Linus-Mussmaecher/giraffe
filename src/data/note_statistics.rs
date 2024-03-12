use super::note::Note;
use std::collections::HashMap;

/// A data struct containing statistical information about a (subset of a) user's notes.
#[derive(Debug, Clone)]
pub struct NoteStatistics {
    /// The total amount of words in the notes.
    /// What is a word and what not mirrors the definition from Note.words.
    pub word_count_total: usize,
    /// The total amount of characters, including whitespace, in the notes.
    pub char_count_total: usize,
    /// The total amount of notes tracked.
    pub note_count_total: usize,
    /// The total amount of _unique_ tags tracked.
    pub tag_count_total: usize,
    /// The total amount of (non-unique) links between notes. Does not count external links.
    pub link_count_total: usize,
    /// A HashMap of all notes matching the given filter used to create this struct.
    /// Provided alongside are their inlinks (global) and inlinks (local, i.e. of notes also matching the filter).
    pub filtered_ids: Vec<(String, usize, usize)>,
}

impl NoteStatistics {
    /// Creates a new set of statistics from the subset of the passed index that matches the given filter.
    pub fn new_with_filters(index: &HashMap<String, Note>, filter: Filter) -> Self {
        // Filter the index
        let filtered_index = index
            .iter()
            .filter(|entry| {
                // Check if any or all the tags specified in the filter are in the note.
                let mut any_tag = filter.tags.is_empty();
                let mut all_tags = true;
                for tag in filter.tags.iter() {
                    if entry.1.tags.contains(tag) {
                        any_tag = true;
                    } else {
                        all_tags = false;
                    }
                }

                // Check if any or all of the words specified in the filter are in the note title.
                let mut any_word = filter.title_words.is_empty();
                for word in filter.title_words.iter() {
                    if entry
                        .1
                        .name
                        .to_lowercase()
                        .contains(&word.to_lowercase().to_string())
                    {
                        any_word = true;
                    }
                }

                // Compare with the filter settings
                (filter.all_tags && all_tags || !filter.all_tags && any_tag) && any_word
            })
            .collect::<HashMap<&String, &Note>>();

        // Create a new hash map with tags and their usage
        let mut tags = HashMap::new();
        for (_, note) in filtered_index.iter() {
            // for every tag found in a note
            for tag in note.tags.iter() {
                // either create a new entry in the hash map or increment an existing entry by one
                match tags.get_mut(tag) {
                    Some(val) => *val += 1,
                    None => {
                        tags.insert(tag.clone(), 1 as usize);
                    }
                }
            }
        }

        // Create a new hash map with note names and the amount they are linked to from other notes.
        // Considers only those notes that match the filter.
        let mut inlinks = HashMap::new();
        for (_, note) in filtered_index.iter() {
            // for every link found within a note
            for link in note.links.iter() {
                // either create a new entry in the hash map or increment an existing entry by one.
                // Note that this does count self-links
                match inlinks.get_mut(link) {
                    Some(val) => *val += 1,
                    None => {
                        inlinks.insert(link.clone(), 1 as usize);
                    }
                }
            }
        }

        // Create a new hash map with note names and the amount they are linked to from other notes.
        // Always considers all notes.
        let mut inlinks_global = HashMap::new();
        for (_, note) in index.iter() {
            // for every link found within a note
            for link in note.links.iter() {
                // either create a new entry in the hash map or increment an existing entry by one.
                // Note that this does count self-links
                match inlinks_global.get_mut(link) {
                    Some(val) => *val += 1,
                    None => {
                        inlinks_global.insert(link.clone(), 1 as usize);
                    }
                }
            }
        }

        Self {
            // Sum up all word counts of notes
            word_count_total: filtered_index.values().map(|note| note.words).sum(),
            // Sum up all char counts of notes.
            char_count_total: filtered_index.values().map(|note| note.characters).sum(),
            // Simply take the lenght of the (filtered) index.
            note_count_total: filtered_index.len(),
            // Take the created tag-usage map and check its length.
            tag_count_total: tags.len(),
            // Take the sum of the length of links-lists from each individual note.
            link_count_total: filtered_index.values().map(|note| note.links.len()).sum(),
            filtered_ids: filtered_index
                .into_keys()
                .map(|id| {
                    (
                        id.clone(),
                        inlinks_global.get(id).copied().unwrap_or(0),
                        inlinks.get(id).copied().unwrap_or(0),
                    )
                })
                .collect(),
        }
    }
}

/// Describes a way to filter notes by their contained tags and/or title
#[derive(Debug, Default, Clone)]
pub struct Filter {
    /// Wether or not all specified tags must be contained in the note in order to match the filter, or only any (=at least one) of them.
    pub all_tags: bool,
    /// The tags to filter by, hash included.
    pub tags: Vec<String>,
    /// The words to search the note title for. No fuzzy matching, must fit completetely.
    pub title_words: Vec<String>,
}
