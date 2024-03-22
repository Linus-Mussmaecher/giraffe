use fuzzy_matcher::FuzzyMatcher;

use super::note::Note;
use std::collections::HashMap;

/// A struct describing statistics to a note in relation to a containing environment.
#[derive(Debug, Clone)]
pub struct NoteEnvStatistics {
    /// The notes id
    pub id: String,
    /// The fuzzy match score of this note with the filter used to create the environment
    pub match_score: i64,
    /// The amount of links pointing to this note from anywhere.
    pub inlinks_global: usize,
    /// The amount of links pointing to this note from other notes within the environment.
    pub inlinks_local: usize,
    /// The amount of links going out from this note to other notes within the environment.
    pub outlinks_local: usize,
    /// The amount of links going out from this note to other notes.
    /// Differs from the length of the 'links' table by not counting broken links.
    pub outlinks_global: usize,
    /// The amount of links originating from this note that do not have a valid target anywhere.
    pub broken_links: usize,
}

impl NoteEnvStatistics {
    /// Creates a new instance of NoteEnvStatistics with only the two passed fields filled out.
    fn new_empty(id: String, match_score: i64) -> Self {
        Self {
            id,
            match_score,
            inlinks_global: 0,
            inlinks_local: 0,
            outlinks_local: 0,
            outlinks_global: 0,
            broken_links: 0,
        }
    }
}

/// A data struct containing statistical information about a (subset of a) user's notes.
/// This subset is called an 'environment' and is described by a filter passed to the constructor.
#[derive(Debug, Clone)]
pub struct EnvironmentStats {
    /// The total amount of words in the notes in this environment.
    /// What is a word and what not mirrors the definition from Note.words.
    pub word_count_total: usize,
    /// The total amount of characters, including whitespace, in the notes of this environment.
    pub char_count_total: usize,
    /// The total amount of notes in this environment.
    pub note_count_total: usize,
    /// The total amount of _unique_ tags in this environment.
    pub tag_count_total: usize,
    /// Total amount of links from a note within the environment to another note within the environment.
    pub local_local_links: usize,
    /// Total amount of links from a note within the environment to any note.
    pub local_global_links: usize,
    /// Total amount of links from any note to a note within the environment.
    pub global_local_links: usize,
    /// A vector of all notes within the environment.
    pub filtered_stats: Vec<NoteEnvStatistics>,
    /// Counts how many links among notes within the environment do not have a valid target anywhere.
    pub broken_links: usize,
}

impl EnvironmentStats {
    /// Creates a new set of statistics from the subset of the passed index that matches the given filter.
    pub fn new_with_filters(index: &HashMap<String, Note>, filter: Filter) -> Self {
        // Create fuzzy matcher
        let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();

        // Filter the index -> Create an iterator
        let mut filtered_index = index
            .iter()
            .filter_map(|(id, note)| {
                // Check if any or all the tags specified in the filter are in the note.
                let mut any_tag = filter.tags.is_empty();
                let mut all_tags = true;
                for tag in filter.tags.iter() {
                    if note.tags.contains(tag) {
                        any_tag = true;
                    } else {
                        all_tags = false;
                    }
                }

                if !(filter.all_tags && all_tags || !filter.all_tags && any_tag) {
                    return None;
                }

                // Check if the rest of the filter fuzzy matches the note title.

                matcher.fuzzy_match(&note.name, &filter.title).map(|score| {
                    (
                        id.clone(),
                        (NoteEnvStatistics::new_empty(id.clone(), score), note),
                    )
                })
            })
            .collect::<HashMap<_, _>>();

        // Count links by iterating over unfiltered index
        for (id, note) in index.iter() {
            // remember if source is from withing the environment
            let local_source = filtered_index.contains_key(id);
            // keep track of found local targets
            let mut local_targets = 0;
            // keep track of found targets
            let mut global_targets = 0;

            // then go over its links
            for link in &note.links {
                if let Some((target, _)) = filtered_index.get_mut(link) {
                    // always count up global inlink count of target
                    target.inlinks_global += 1;
                    // if id of source is also in filtered index, also count up local inlink count of target
                    if local_source {
                        target.inlinks_local += 1;
                    }
                    // since this target was in the environment, increment the counter
                    local_targets += 1;
                    global_targets += 1;
                } else {
                    // else, check if target exists at all
                    if index.contains_key(link) {
                        global_targets += 1;
                    }
                }
            }
            // add the found local targets to the statistics (this could be an assignment).
            if let Some((source, _)) = filtered_index.get_mut(id) {
                source.outlinks_local += local_targets;
                source.outlinks_global += global_targets;
                source.broken_links = note.links.len() - global_targets;
            }
        }

        Self {
            // Word count: Just map over the stats.
            word_count_total: filtered_index.values().map(|(_, stats)| stats.words).sum(),
            // Char count: Just map over the stats
            char_count_total: filtered_index
                .values()
                .map(|(_, stats)| stats.characters)
                .sum(),
            // Total notes: Just the length of the filtered index.
            note_count_total: filtered_index.len(),
            // Total tags: Collect all tag vectors of notes into a HashSet, then take its length.
            tag_count_total: filtered_index
                .values()
                .map(|(_, stats)| &stats.tags)
                .flatten()
                .collect::<std::collections::HashSet<_>>()
                .len(),
            // Local-Local links: Check outgoing local links of all notes. Could also check incoming local links of all notes.
            local_local_links: filtered_index
                .values()
                .map(|(env_stats, _)| env_stats.outlinks_local)
                .sum(),
            // Local-Global links: Check outgoing global links of all notes.
            local_global_links: filtered_index
                .values()
                .map(|(env_stats, _)| env_stats.outlinks_global)
                .sum(),
            // Global-Local Links: Check incoming links of all notes.
            global_local_links: filtered_index
                .values()
                .map(|(env_stats, _)| env_stats.inlinks_global)
                .sum(),
            // Broken links: Again, just sum it up.
            broken_links: filtered_index
                .values()
                .map(|(env_stats, _)| env_stats.broken_links)
                .sum(),
            // Finally, reduce the vector to just the env stats
            filtered_stats: {
                let mut fs = filtered_index
                    .into_values()
                    .map(|(env_stats, _)| env_stats)
                    .collect::<Vec<_>>();

                // Default sort: By match score, descending.
                fs.sort_by_cached_key(|env_stats| env_stats.match_score);
                fs.reverse();

                fs
            },
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
    /// The words to search the note title for. Will be fuzzy matched with the note title.
    pub title: String,
}
