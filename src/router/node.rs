use std::collections::HashSet;

use crate::router::conversation::ConversationId;
use nostr::Filter;


pub struct RelayNode {
    pub conversations: HashSet<ConversationId>,
}

impl RelayNode {
    pub fn new() -> Self {
        RelayNode {
            conversations: HashSet::new(),
        }
    }
}

pub struct FilterNode {
    pub filter: Filter,
    pub conversations: HashSet<ConversationId>,
}

impl FilterNode {
    pub fn new(filter: Filter) -> Self {
        FilterNode {
            filter,
            conversations: HashSet::new(),
        }
    }
}
