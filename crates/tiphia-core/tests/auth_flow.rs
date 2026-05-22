mod support;

use tiphia_core::{
    entities::users::{UserRole, UserStatus},
    services::{
        auth::{self, BootstrapAdminInput, LoginInput},
        settings::{self, SiteSettings},
        users::{self, CreateUserInput, UpdateUserInput},
    },
};

#[tokio::test]
async fn bootstrap_login_and_disabled_user_token_is_rejected() {
    let state = support::state().await;

    let token = auth::bootstrap_admin(
        &state,
        BootstrapAdminInput {
            username: "admin".to_owned(),
            email: "admin@example.com".to_owned(),
            password: "long-enough-password".to_owned(),
            display_name: Some("Admin".to_owned()),
        },
    )
    .await
    .expect("bootstrap admin");

    assert_eq!(token.user.username, "admin");
    assert_eq!(token.user.role, UserRole::Root);

    let login = auth::login(
        &state,
        LoginInput {
            account: "admin@example.com".to_owned(),
            password: "long-enough-password".to_owned(),
            captcha: None,
        },
    )
    .await
    .expect("login");

    assert!(!login.access_token.is_empty());

    users::update(
        &state,
        &token.user,
        token.user.id,
        UpdateUserInput {
            email: None,
            display_name: None,
            role: None,
            status: Some(UserStatus::Disabled),
        },
    )
    .await
    .expect_err("root cannot disable self");

    let author = users::create(
        &state,
        &token.user,
        CreateUserInput {
            username: "author".to_owned(),
            email: "author@example.com".to_owned(),
            password: "long-enough-password".to_owned(),
            display_name: "Author".to_owned(),
            role: UserRole::Author,
        },
    )
    .await
    .expect("create author");

    users::update(
        &state,
        &token.user,
        author.id,
        UpdateUserInput {
            email: None,
            display_name: None,
            role: None,
            status: Some(UserStatus::Disabled),
        },
    )
    .await
    .expect("disable author");

    assert!(
        auth::current_user(&state, &login.access_token)
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn registration_can_be_enabled_from_settings() {
    let state = support::state().await;

    let disabled = auth::register(
        &state,
        auth::RegisterInput {
            username: "reader".to_owned(),
            email: "reader@example.com".to_owned(),
            password: "long-enough-password".to_owned(),
            display_name: None,
            captcha: None,
        },
    )
    .await;
    assert!(disabled.is_err());

    let site_settings = SiteSettings {
        registration_enabled: true,
        ..SiteSettings::default()
    };
    settings::update(&state, site_settings)
        .await
        .expect("enable registration");

    let token = auth::register(
        &state,
        auth::RegisterInput {
            username: "reader".to_owned(),
            email: "reader@example.com".to_owned(),
            password: "long-enough-password".to_owned(),
            display_name: Some("Reader".to_owned()),
            captcha: None,
        },
    )
    .await
    .expect("register reader");

    assert_eq!(token.user.role, UserRole::Author);
}
