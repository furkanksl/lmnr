use sodiumoxide::{
    crypto::aead::xchacha20poly1305_ietf::{gen_nonce, open, seal, Key, Nonce},
    hex,
};

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::db;

pub async fn insert_storage_state(
    pool: &PgPool,
    user_id: &Uuid,
    storage_state: &String,
) -> Result<()> {
    let key_hex = std::env::var("AEAD_SECRET_KEY").unwrap();
    let key = Key::from_slice(hex::decode(key_hex).unwrap().as_slice()).unwrap();

    let nonce = gen_nonce();
    let encrypted = seal(&storage_state.as_bytes(), None, &nonce, &key);

    db::user_storage_states::insert_user_storage_state(
        pool,
        user_id,
        &vec![hex::encode(encrypted)],
        &vec![hex::encode(nonce)],
    )
    .await?;

    Ok(())
}

pub async fn get_storage_state(pool: &PgPool, user_id: &Uuid) -> Result<Option<String>> {
    let states = db::user_storage_states::get_user_storage_state(pool, user_id).await?;

    if states.is_empty() {
        return Ok(None);
    }

    let encrypted_state = states.first().unwrap();

    let key_hex = std::env::var("AEAD_SECRET_KEY").unwrap();
    let key = Key::from_slice(hex::decode(key_hex).unwrap().as_slice()).unwrap();

    let encrypted = hex::decode(encrypted_state.cookies.clone()).or(Err(anyhow::anyhow!(
        "Failed to decode hex value for cookie",
    )))?;
    let nonce_bytes = hex::decode(encrypted_state.nonce.clone()).or(Err(anyhow::anyhow!(
        "Failed to decode hex nonce for cookie",
    )))?;

    let nonce = Nonce::from_slice(nonce_bytes.as_slice()).ok_or(anyhow::anyhow!(
        "Failed to convert nonce bytes to Nonce for cookie",
    ))?;
    let decrypted =
        open(encrypted.as_slice(), None, &nonce, &key).expect("Failed to decrypt cookie");

    Ok(Some(String::from_utf8(decrypted).unwrap()))
}
