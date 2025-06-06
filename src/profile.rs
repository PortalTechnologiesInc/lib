use nostr::{event::Kind, filter::Filter, key::PublicKey, nips::nip01::Metadata};

use crate::{
    protocol::model::Timestamp,
    router::{
        Conversation, ConversationError, ConversationMessage, Response,
        adapters::{ConversationWithNotification, one_shot::OneShotSender},
    },
};

pub struct FetchProfileInfoConversation {
    pubkey: PublicKey,
    expires_at: Timestamp,
}

impl FetchProfileInfoConversation {
    pub fn new(pubkey: PublicKey) -> Self {
        Self {
            pubkey,
            expires_at: Timestamp::now_plus_seconds(5),
        }
    }
}

impl Conversation for FetchProfileInfoConversation {
    fn init(&mut self) -> Result<crate::router::Response, crate::router::ConversationError> {
        Ok(Response::new().filter(
            Filter::new()
                .author(self.pubkey)
                .kind(Kind::Metadata)
                .limit(1),
        ))
    }

    fn on_message(
        &mut self,
        message: crate::router::ConversationMessage,
    ) -> Result<crate::router::Response, crate::router::ConversationError> {
        if let ConversationMessage::Cleartext(event) = message {
            let metadata: Result<Metadata, _> = serde_json::from_value(event.content);
            if let Ok(metadata) = metadata {
                return Ok(Response::new().notify(Profile::from(metadata)).finish());
            }
        } else if let ConversationMessage::EndOfStoredEvents = message {
            return Ok(Response::new().notify(Option::<Profile>::None).finish());
        }

        Ok(Response::new().notify(Option::<Profile>::None).finish())
    }

    fn is_expired(&self) -> bool {
        self.expires_at > Timestamp::now()
    }
}

impl ConversationWithNotification for FetchProfileInfoConversation {
    type Notification = Option<Profile>;
}

pub struct SetProfileConversation {
    profile: Profile,
}

impl SetProfileConversation {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl OneShotSender for SetProfileConversation {
    type Error = ConversationError;

    fn send(
        state: &mut crate::router::adapters::one_shot::OneShotSenderAdapter<Self>,
    ) -> Result<Response, Self::Error> {
        let metadata: Metadata = state.profile.clone().into();
        Ok(Response::new()
            .broadcast_unencrypted(Kind::Metadata, Default::default(), metadata)
            .finish())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "bindings", derive(uniffi::Record))]
pub struct Profile {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub picture: Option<String>,
    pub nip05: Option<String>,
}

impl From<nostr::nips::nip01::Metadata> for Profile {
    fn from(metadata: nostr::nips::nip01::Metadata) -> Self {
        Self {
            name: metadata.name,
            display_name: metadata.display_name,
            picture: metadata.picture,
            nip05: metadata.nip05,
        }
    }
}
impl Into<nostr::nips::nip01::Metadata> for Profile {
    fn into(self) -> nostr::nips::nip01::Metadata {
        nostr::nips::nip01::Metadata {
            name: self.name,
            display_name: self.display_name,
            picture: self.picture,
            nip05: self.nip05,
            ..Default::default()
        }
    }
}
