use nostr::filter::Filter;

pub fn can_be_merged(filter1: &Filter, filter2: &Filter) -> bool {
    filter1.kinds == filter2.kinds && filter1.generic_tags == filter2.generic_tags
}

pub fn merge_filters(filter1: &Filter, filter2: &Filter) -> Filter {
    let mut cloned = filter1.clone();

    // Merge ids
    if let Some(ids) = &mut cloned.ids {
        if let Some(ids2) = &filter2.ids {
            ids.extend(ids2.iter().cloned());
        }
    } else if let Some(ids2) = &filter2.ids {
        cloned.ids = Some(ids2.clone());
    }
    // Merge authors
    if let Some(authors) = &mut cloned.authors {
        if let Some(authors2) = &filter2.authors {
            authors.extend(authors2.iter().cloned());
        }
    } else if let Some(authors2) = &filter2.authors {
        cloned.authors = Some(authors2.clone());
    }
    // Merge since (take the minimum if both are Some)
    match (cloned.since, filter2.since) {
        (Some(s1), Some(s2)) => cloned.since = Some(std::cmp::min(s1, s2)),
        (None, Some(s2)) => cloned.since = Some(s2),
        _ => {}
    }

    // Merge until (take the maximum if both are Some)
    match (cloned.until, filter2.until) {
        (Some(u1), Some(u2)) => cloned.until = Some(std::cmp::max(u1, u2)),
        (None, Some(u2)) => cloned.until = Some(u2),
        _ => {}
    }

    cloned
}
