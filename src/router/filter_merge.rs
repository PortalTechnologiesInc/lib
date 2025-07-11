use nostr::filter::Filter;

/// Check if two filters can be merged
pub fn can_merge_filters(filter: &Filter, new: &Filter) -> bool {

    // if filter has kinds, new filter must have the same kinds
    if let Some(filter_kinds) = &filter.kinds {
        if let Some(new_kinds) = &new.kinds {
            if filter_kinds != new_kinds {
                return false;
            }
        }
    }

    // if filter has search, new filter must have the same search
    if let Some(filter_search) = &filter.search {
        if let Some(new_search) = &new.search {
            if filter_search != new_search {
                return false;
            }
        }
    }

    // if filter has since, new filter must have the same since
    if let Some(filter_since) = &filter.since {
        if let Some(new_since) = &new.since {
            if filter_since != new_since {
                return false;
            }
        }
    }

    // if filter has until, new filter must have the same until
    if let Some(filter_until) = &filter.until {
        if let Some(new_until) = &new.until {
            if filter_until != new_until {
                return false;
            }
        }
    }

    // iif filter has limit dont match
    if let Some(_filter_limit) = &filter.limit {
        return false;
    }
    true
}


/// Merge two filters
pub fn merge_filters(filter: &Filter, new: &Filter) -> Filter {

    let mut merged_filter = filter.clone();

    // Merge authors if present
    if let Some(new_authors) = &new.authors {
        merged_filter = merged_filter.authors(new_authors.clone());
    }

    // Merge tags if present
    for (tag, values) in new.generic_tags.iter() {
        merged_filter =merged_filter.custom_tags(tag.clone(), values.clone());
    }

    merged_filter
}