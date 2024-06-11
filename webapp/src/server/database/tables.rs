use super::config::SERVER_DATA_PATH;
use anyhow::Context;
use std::sync::LazyLock;

pub static TABLES_DB: LazyLock<sled::Db> =
    LazyLock::new(|| sled::open(format!("{SERVER_DATA_PATH}/tables.sled")).unwrap());

use super::super::api::user::UserProfile;

pub static USER_PROFILE_DB: LazyLock<typed_sled::Tree<uuid::Uuid, UserProfile>> =
    LazyLock::new(|| {
        typed_sled::Tree::<uuid::Uuid, UserProfile>::open(&TABLES_DB, "user_profile_v1")
    });

pub fn get_user_profile(uuid: &uuid::Uuid) -> anyhow::Result<UserProfile> {
    Ok(USER_PROFILE_DB
        .get(uuid)
        .context("operation failed")?
        .context("user profile not found")?)
}

pub fn get_or_create_user_profile(uuid: &uuid::Uuid) -> anyhow::Result<UserProfile> {
    if let Ok(u) = get_user_profile(uuid) {
        return Ok(u);
    }
    let new: UserProfile = UserProfile::default();
    USER_PROFILE_DB.insert(uuid, &new).context("cannot write")?;
    Ok(new)
}