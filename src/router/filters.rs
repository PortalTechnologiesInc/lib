use nostr::filter::Filter;

pub fn can_be_merged(filter1: &Filter, filter2: &Filter) -> bool {
    filter1.kinds == filter2.kinds
}

pub fn merge_filters(filter1: &Filter, filter2: &Filter) -> Filter {
    let mut cloned = filter1.clone();

    // Merge authors
    if let Some(authors) = &mut cloned.authors {
        if let Some(authors2) = &filter2.authors {
            authors.extend(authors2);
        }
    }

    cloned
}