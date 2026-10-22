use super::*;
use secret_service::{EncryptionType, SecretService};
use std::collections::HashMap;

fn attributes(id: &str) -> HashMap<&str, &str> {
    HashMap::from([
        ("application", "cosmic-desktop-cards"),
        ("purpose", "google-calendar"),
        ("credential", id),
    ])
}
async fn service() -> Result<SecretService<'static>, Problem> {
    if store::preview() {
        return Err(Problem::Preview);
    }
    tokio::time::timeout(
        Duration::from_secs(5),
        SecretService::connect(EncryptionType::Dh),
    )
    .await
    .map_err(|_| Problem::Keyring)?
    .map_err(|_| Problem::Keyring)
}
/// Only an explicit Connect/Retry/Disconnect may open an unlock prompt.
/// Background polling never interrupts the desktop with a keyring dialog.
pub(super) async fn ready() -> Result<(), Problem> {
    let service = service().await?;
    let collection = match service.get_default_collection().await {
        Ok(collection) => collection,
        Err(_) => service
            .create_collection("Halcyon", "default")
            .await
            .map_err(|_| Problem::Keyring)?,
    };
    if collection.is_locked().await.map_err(|_| Problem::Keyring)? {
        collection.unlock().await.map_err(|_| Problem::Locked)?;
    }
    Ok(())
}
pub(super) async fn read(id: &str, unlock: bool) -> Result<Credential, Problem> {
    let service = service().await?;
    let found = service
        .search_items(attributes(id))
        .await
        .map_err(|_| Problem::Keyring)?;
    let item = if let Some(item) = found.unlocked.first() {
        item
    } else if let Some(item) = found.locked.first() {
        if !unlock {
            return Err(Problem::Locked);
        }
        tokio::time::timeout(Duration::from_secs(60), item.unlock())
            .await
            .map_err(|_| Problem::Locked)?
            .map_err(|_| Problem::Locked)?;
        item
    } else {
        return Err(Problem::SignIn);
    };
    let secret = item.get_secret().await.map_err(|_| Problem::Locked)?;
    serde_json::from_slice(&secret).map_err(|_| Problem::SignIn)
}
pub(super) async fn save(id: &str, credential: &Credential) -> Result<(), Problem> {
    let service = service().await?;
    let collection = service
        .get_default_collection()
        .await
        .map_err(|_| Problem::Keyring)?;
    if collection.is_locked().await.map_err(|_| Problem::Keyring)? {
        return Err(Problem::Locked);
    }
    let bytes = serde_json::to_vec(credential).map_err(|_| Problem::Storage)?;
    collection
        .create_item(
            "Halcyon · Google Calendar",
            attributes(id),
            &bytes,
            true,
            "application/json",
        )
        .await
        .map_err(|_| Problem::Keyring)?;
    Ok(())
}
pub(super) async fn remove(id: &str) -> Result<(), Problem> {
    let service = service().await?;
    let found = service
        .search_items(attributes(id))
        .await
        .map_err(|_| Problem::Keyring)?;
    for item in found.unlocked.iter().chain(found.locked.iter()) {
        if item.is_locked().await.map_err(|_| Problem::Keyring)? {
            tokio::time::timeout(Duration::from_secs(60), item.unlock())
                .await
                .map_err(|_| Problem::Locked)?
                .map_err(|_| Problem::Locked)?;
        }
        item.delete().await.map_err(|_| Problem::Keyring)?;
    }
    Ok(())
}
