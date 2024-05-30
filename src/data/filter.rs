use fuzzy_matcher::FuzzyMatcher;
/// Describes a way to filter notes by their contained tags and/or title
#[derive(Debug, Default, Clone)]
pub struct Filter {
    /// Wether or not all specified tags must be contained in the note in order to match the filter, or only any (=at least one) of them.
    pub any: bool,
    /// The tags to include and exclude by, hash included.
    pub tags: Vec<(String, bool)>,
    /// The links to look for or exclude, already converted to ids.
    pub links: Vec<(String, bool)>,
    /// The words to search the note title for. Will be fuzzy matched with the note title.
    pub title: String,
}

impl Filter {
    pub fn new(filter_string: &str, any: bool) -> Self {
        let mut tags = Vec::new();
        let mut links = Vec::new();
        let mut title = String::new();

        // Go through words
        for word in filter_string.split_whitespace() {
            if word.starts_with("!#") {
                tags.push((word.trim_start_matches("!").to_string(), false));
                continue;
            }
            if word.starts_with('#') {
                tags.push((word.to_string(), true));
                continue;
            }
            if word.starts_with("!>") {
                links.push((
                    super::name_to_id(word.trim_start_matches("!>")).to_string(),
                    false,
                ));
                continue;
            }
            if word.starts_with('>') {
                links.push((
                    super::name_to_id(word.trim_start_matches(">")).to_string(),
                    true,
                ));
                continue;
            }
            // if nothing else fits
            title.push_str(word);
        }

        // check for any or all tags
        Self {
            any,
            tags,
            links,
            title,
        }
    }

    pub fn apply(&self, note: &super::Note) -> Option<i64> {
        // === === TAGS === ===

        let mut any = false;
        let mut all = true;
        for (tag, included) in self.tags.iter() {
            if note
                // go over all tags
                .tags
                .iter()
                // split each tag into..
                .map(|tag| {
                    // an iterator of substring starting at 0 and going to every appearance to /
                    tag.match_indices("/")
                        .map(|(index, _match)| &tag[0..index])
                        // and appended just a substring that is the whole tag
                        .chain(std::iter::once(tag.as_str()))
                })
                // flatten this so we have just an iterator over (sub)strs
                .flatten()
                // check if any of these substring is the searched tag
                .any(|subtag| subtag == tag)
            // now compare this to our expectation
            //  - inclusion: We _want_ one of them to be equal
            //  - exclusion: We _dont_ want one of them to be equal
             == *included
            {
                // this did match our expectation (one of them is equal in case of inclusion or none of them is equal in case of exclusion)
                // so at least one condition (this one) is true
                any = true;
            } else {
                // this did not match our expectation (none of them is equal in case of inclusion or one of them is equal in case of exclusion)
                // so not all conditions can be true
                all = false;
            }
        }

        // === === LINKS === ===

        // go through all links
        for (link, included) in self.links.iter() {
            // if the links is contained and we want it to be contained or not contained and we want it to be not contained
            if note.links.contains(link) == *included {
                // at least one condition (this one) is true
                any = true;
            } else {
                // else, at least one condition is false, so not all of them are true
                all = false;
            }
        }

        // if there are no tag or link conditions, always go to the next step
        if !(self.tags.is_empty() && self.links.is_empty())  &&
            // else, check if we wanted all conditions or any and compare to the relevant variable
            ((!self.any && !all) || (self.any && !any))
        {
            return None;
        }

        // If nothing has triggerd an exclusion criterion, return the fuzzy match score
        let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();
        return matcher.fuzzy_match(&note.name, &self.title);
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filters() {
        let index = crate::data::NoteIndex::new(
            std::path::Path::new("./tests/common/notes/"),
            &crate::config::Config::default(),
        );

        assert_eq!(index.inner.len(), 11);

        // let index = std::rc::Rc::new(std::cell::RefCell::new(index));

        let linux = index.inner.get("linux").unwrap();
        let win = index.inner.get("windows").unwrap();
        let osx = index.inner.get("osx").unwrap();

        // === Filter 1 ===

        let filter1 = Filter {
            any: false,
            tags: vec![("#os".to_string(), true), ("#os/win".to_string(), false)],
            links: vec![],
            title: String::new(),
        };

        let filter2 = Filter::new("!#lietheo #diffgeo >Manifold !>atlas", false);

        assert_eq!(
            filter2.tags,
            vec![
                ("#lietheo".to_string(), false),
                ("#diffgeo".to_string(), true)
            ]
        );
        assert_eq!(
            filter2.links,
            vec![("manifold".to_string(), true), ("atlas".to_string(), false)]
        );
        assert_eq!(filter2.title, "");

        assert!(filter1.apply(linux).is_some());
        assert!(filter1.apply(osx).is_some());
        assert!(filter1.apply(win).is_none());

        let liegroup = index.inner.get("lie-group").unwrap();
        let chart = index.inner.get("chart").unwrap();
        let manifold = index.inner.get("manifold").unwrap();
        let smoothmap = index.inner.get("smooth-map").unwrap();
        let topology = index.inner.get("topology").unwrap();

        assert!(filter2.apply(liegroup).is_none());
        assert!(filter2.apply(chart).is_some());
        assert!(filter2.apply(manifold).is_none());
        assert!(filter2.apply(smoothmap).is_none());
        assert!(filter2.apply(topology).is_none());
    }
}
