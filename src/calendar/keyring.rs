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
pub(super) async fn read<T: serde::de::DeserializeOwned>(
    id: &str,
    unlock: bool,
) -> Result<T, Problem> {
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
pub(super) async fn save<T: Serialize>(id: &str, credential: &T) -> Result<(), Problem> {
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
            "Halcyon · Calendar",
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

/// Creation may outlive a cancelled caller at the D-Bus service. Keep both the
/// journal and lease until that request actually completes, even if the outer
/// subscription timeout drops its future. A dropped result releases the lease
/// and leaves the unused ID available for retry cleanup.
pub(super) async fn save_prepared<T: Serialize + Send + Sync + 'static>(
    id: String,
    credential: T,
) -> Result<store::CredentialLease, Problem> {
    let save_id = id.clone();
    prepare_with(id, async move { save(&save_id, &credential).await }).await
}
pub(super) async fn prepare_with(
    id: String,
    creation: impl std::future::Future<Output = Result<(), Problem>> + Send + 'static,
) -> Result<store::CredentialLease, Problem> {
    let lease = store::prepare_credential(&id).await?;
    tokio::spawn(async move {
        creation.await?;
        Ok(lease)
    })
    .await
    .map_err(|_| Problem::Keyring)?
}
