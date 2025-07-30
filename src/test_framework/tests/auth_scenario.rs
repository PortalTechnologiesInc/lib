use crate::sdk::auth::KeyHandshakeEvent;
use crate::{
    app::auth::{
        AuthChallengeListenerConversation, AuthResponseConversation, KeyHandshakeConversation,
    },
    protocol::{
        LocalKeypair,
        key_handshake::KeyHandshakeUrl,
        model::{
            Nonce, Timestamp,
            auth::{AuthResponseEvent, AuthResponseStatus},
        },
        subkey::{PrivateSubkeyManager, SubkeyMetadata},
    },
    router::{
        MultiKeyListenerAdapter, MultiKeySenderAdapter, adapters::one_shot::OneShotSenderAdapter,
    },
    sdk::auth::{AuthChallengeSenderConversation, KeyHandshakeReceiverConversation},
    test_framework::{ScenarioBuilder, logger::init_logger},
    utils::random_string,
};
use nostr::Keys;

#[tokio::test]
async fn test_auth_flow() {
    init_logger();

    // Create keys for service and client
    let service_keys = Keys::generate();
    let client_keys = Keys::generate();

    // Create the auth init URL
    let token = random_string(32);
    let url = KeyHandshakeUrl {
        main_key: service_keys.public_key().into(),
        relays: vec!["simulated".to_string()],
        token: token.clone(),
        subkey: None,
    };

    // Create the network with both nodes
    let network = ScenarioBuilder::new()
        .with_node(
            "service".to_string(),
            LocalKeypair::new(service_keys.clone(), None),
        )
        .await
        .with_node(
            "client".to_string(),
            LocalKeypair::new(client_keys.clone(), None),
        )
        .await
        .run()
        .await;

    // Get the routers
    let service_router = network.get_node("service").unwrap();
    let client_router = network.get_node("client").unwrap();

    // 1. Service sets up to receive auth init
    let mut service_notifications = service_router
        .add_and_subscribe(Box::new(MultiKeyListenerAdapter::new(
            KeyHandshakeReceiverConversation::new(service_keys.public_key(), token.clone()),
            None,
        )))
        .await
        .unwrap();

    // 2. Client sets up to listen for auth challenge
    let mut challenge_notifications = client_router
        .add_and_subscribe(Box::new(MultiKeyListenerAdapter::new(
            AuthChallengeListenerConversation::new(client_keys.public_key()),
            None,
        )))
        .await
        .unwrap();

    // 3. Client initiates auth flow
    let key_handshake = KeyHandshakeConversation::new(url, vec!["simulated".to_string()]);
    client_router
        .add_conversation(Box::new(OneShotSenderAdapter::new_with_user(
            key_handshake.url.send_to(),
            key_handshake
                .url
                .subkey
                .map(|s| vec![s.into()])
                .unwrap_or_default(),
            key_handshake,
        )))
        .await
        .unwrap();

    // 4. Service receives auth init
    let key_handshake_event: KeyHandshakeEvent =
        service_notifications.next().await.unwrap().unwrap();
    assert_eq!(key_handshake_event.main_key, client_keys.public_key());

    // 5. Service sends auth challenge
    let mut auth_response_event = service_router
        .add_and_subscribe(Box::new(MultiKeySenderAdapter::new_with_user(
            key_handshake_event.main_key,
            vec![],
            AuthChallengeSenderConversation::new(service_keys.public_key(), None),
        )))
        .await
        .unwrap();

    // 6. Client receives auth challenge
    let auth_challenge_event: crate::app::auth::AuthChallengeEvent =
        challenge_notifications.next().await.unwrap().unwrap();
    assert_eq!(
        auth_challenge_event.service_key,
        service_keys.public_key().into()
    );
    assert_eq!(
        auth_challenge_event.recipient,
        service_keys.public_key().into()
    );

    // 7. Clients accepts requrest
    let approve = AuthResponseConversation::new(
        auth_challenge_event.clone(),
        None,
        AuthResponseStatus::Approved {
            granted_permissions: vec![],
            session_token: "ABC".to_string(),
        },
    );
    client_router
        .add_conversation(Box::new(OneShotSenderAdapter::new_with_user(
            auth_challenge_event.recipient.into(),
            vec![],
            approve,
        )))
        .await
        .unwrap();

    // 8. Wait for auth response notification
    let auth_response_event: AuthResponseEvent = auth_response_event.next().await.unwrap().unwrap();
    assert_eq!(
        auth_response_event.user_key,
        client_keys.public_key().into()
    );
    assert_eq!(
        auth_response_event.recipient,
        client_keys.public_key().into()
    );
}

#[tokio::test]
async fn test_auth_with_subkey_client() {
    init_logger();

    // Create keys for service and client
    let service_keys = Keys::generate();
    let client_keys_master = Keys::generate();
    let client_keys = client_keys_master
        .create_subkey(&SubkeyMetadata {
            name: "test subkey".to_string(),
            nonce: Nonce::new(rand::random()),
            valid_from: Timestamp::now(),
            expires_at: Timestamp::now_plus_seconds(3600),
            permissions: vec![],
            version: 1,
        })
        .unwrap();

    let (client_keys, client_subkey_proof) = client_keys.split();
    log::info!(
        "client_master: {:?}, client_subkey: {:?}",
        client_keys_master.public_key(),
        client_keys.public_key()
    );

    // Create the network with both nodes
    let network = ScenarioBuilder::new()
        .with_node(
            "service".to_string(),
            LocalKeypair::new(service_keys.clone(), None),
        )
        .await
        .with_node(
            "client".to_string(),
            LocalKeypair::new(client_keys.clone(), Some(client_subkey_proof.clone())),
        )
        .await
        .run()
        .await;

    // Get the routers
    let service_router = network.get_node("service").unwrap();
    let client_router = network.get_node("client").unwrap();

    // 1. Client sets up to listen for auth challenge
    let mut challenge_notifications = client_router
        .add_and_subscribe(Box::new(MultiKeyListenerAdapter::new(
            AuthChallengeListenerConversation::new(client_keys.public_key()),
            Some(client_subkey_proof.clone()),
        )))
        .await
        .unwrap();

    // 2. Service sends auth challenge (we explicitly don't set the subkey here so that the client has to negotiate it)
    let mut auth_response_event = service_router
        .add_and_subscribe(Box::new(MultiKeySenderAdapter::new_with_user(
            client_keys_master.public_key(),
            vec![],
            AuthChallengeSenderConversation::new(service_keys.public_key(), None),
        )))
        .await
        .unwrap();

    // 3. Client receives auth challenge
    let auth_challenge_event: crate::app::auth::AuthChallengeEvent =
        challenge_notifications.next().await.unwrap().unwrap();
    assert_eq!(
        auth_challenge_event.service_key,
        service_keys.public_key().into()
    );
    assert_eq!(
        auth_challenge_event.recipient,
        service_keys.public_key().into()
    );

    // 4. Clients accepts requrest
    let approve = AuthResponseConversation::new(
        auth_challenge_event.clone(),
        Some(client_subkey_proof),
        AuthResponseStatus::Approved {
            granted_permissions: vec![],
            session_token: "ABC".to_string(),
        },
    );
    client_router
        .add_conversation(Box::new(OneShotSenderAdapter::new_with_user(
            auth_challenge_event.recipient.into(),
            vec![],
            approve,
        )))
        .await
        .unwrap();

    // 5. Wait for auth response notification
    let auth_response_event: AuthResponseEvent = auth_response_event.next().await.unwrap().unwrap();
    assert_eq!(
        auth_response_event.user_key,
        client_keys_master.public_key().into()
    );
    assert_eq!(
        auth_response_event.recipient,
        client_keys.public_key().into()
    );
}

#[tokio::test]
async fn test_auth_with_subkey_service() {
    init_logger();

    // Create keys for service and client
    let service_keys_master = Keys::generate();
    let client_keys = Keys::generate();
    let service_keys = service_keys_master
        .create_subkey(&SubkeyMetadata {
            name: "test subkey".to_string(),
            nonce: Nonce::new(rand::random()),
            valid_from: Timestamp::now(),
            expires_at: Timestamp::now_plus_seconds(3600),
            permissions: vec![],
            version: 1,
        })
        .unwrap();

    let (service_keys, service_subkey_proof) = service_keys.split();
    log::info!(
        "service_master: {:?}, service_subkey: {:?}",
        service_keys_master.public_key(),
        service_keys.public_key()
    );

    // Create the network with both nodes
    let network = ScenarioBuilder::new()
        .with_node(
            "service".to_string(),
            LocalKeypair::new(service_keys.clone(), Some(service_subkey_proof.clone())),
        )
        .await
        .with_node(
            "client".to_string(),
            LocalKeypair::new(client_keys.clone(), None),
        )
        .await
        .run()
        .await;

    // Get the routers
    let service_router = network.get_node("service").unwrap();
    let client_router = network.get_node("client").unwrap();

    // 1. Client sets up to listen for auth challenge
    let mut challenge_notifications = client_router
        .add_and_subscribe(Box::new(MultiKeyListenerAdapter::new(
            AuthChallengeListenerConversation::new(client_keys.public_key()),
            None,
        )))
        .await
        .unwrap();

    // 2. Service sends auth challenge (we explicitly don't set the subkey here so that the client has to negotiate it)
    let mut auth_response_event = service_router
        .add_and_subscribe(Box::new(MultiKeySenderAdapter::new_with_user(
            client_keys.public_key(),
            vec![],
            AuthChallengeSenderConversation::new(
                service_keys.public_key(),
                Some(service_subkey_proof.clone()),
            ),
        )))
        .await
        .unwrap();

    // 3. Client receives auth challenge
    let auth_challenge_event: crate::app::auth::AuthChallengeEvent =
        challenge_notifications.next().await.unwrap().unwrap();
    assert_eq!(
        auth_challenge_event.service_key,
        service_keys_master.public_key().into()
    );
    assert_eq!(
        auth_challenge_event.recipient,
        service_keys.public_key().into()
    );

    // 4. Clients accepts requrest
    let approve = AuthResponseConversation::new(
        auth_challenge_event.clone(),
        None,
        AuthResponseStatus::Approved {
            granted_permissions: vec![],
            session_token: "ABC".to_string(),
        },
    );
    client_router
        .add_conversation(Box::new(OneShotSenderAdapter::new_with_user(
            auth_challenge_event.recipient.into(),
            vec![],
            approve,
        )))
        .await
        .unwrap();

    // 5. Wait for auth response notification
    let auth_response_event: AuthResponseEvent = auth_response_event.next().await.unwrap().unwrap();
    assert_eq!(
        auth_response_event.user_key,
        client_keys.public_key().into()
    );
    assert_eq!(
        auth_response_event.recipient,
        client_keys.public_key().into()
    );
}
