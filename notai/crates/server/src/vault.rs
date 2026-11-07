//! Obsidian canlı klasörü (Gökhan, 2026-11-07): `NOTAI_VAULT_DIR` ayarlıysa veri her
//! değiştiğinde `NotAI/` alt klasörü yeniden yazılır. Yalnız NotAI'nin yazdığı dosyalar
//! (`.notai/manifest.json`) güncellenir/silinir; kullanıcının kendi notlarına dokunulmaz.
//! İçerik değişmeyen dosya yeniden yazılmaz (Obsidian ve senkron araçları gereksiz tetiklenmez).

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use thiserror::Error;

use crate::state::AppState;

const SEQ_KEY: &str = "vault_seq";
const MANIFEST: &str = ".notai/manifest.json";

#[derive(Debug, Error)]
pub enum VaultError {
    #[error(transparent)]
    Store(#[from] notai_store::StoreError),
    #[error("kasa dosyası yazılamadı ({path}): {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("güvensiz kasa yolu: {0}")]
    UnsafePath(String),
}

fn io(path: &Path) -> impl FnOnce(std::io::Error) -> VaultError + '_ {
    move |source| VaultError::Io {
        path: path.display().to_string(),
        source,
    }
}

/// Göreli yol kasa kökünün dışına çıkamaz ("..", mutlak yol yok).
fn safe_join(root: &Path, rel: &str) -> Result<PathBuf, VaultError> {
    let p = Path::new(rel);
    if p.components().all(|c| matches!(c, Component::Normal(_))) {
        Ok(root.join(p))
    } else {
        Err(VaultError::UnsafePath(rel.to_owned()))
    }
}

/// Kasayı günceller. Veri son yazımdan beri değişmediyse hiçbir şey yapmaz (`None`);
/// değiştiyse yazılan/silinen dosya sayısını döndürür. `force`: değişmemiş olsa da yaz.
pub async fn write_vault(state: &AppState, force: bool) -> Result<Option<usize>, VaultError> {
    let Some(root) = state.cfg.vault_dir.clone() else {
        return Ok(None);
    };
    let seq = state.store.change_seq().await?;
    let last = state
        .store
        .kv_get(SEQ_KEY)
        .await?
        .and_then(|s| s.parse::<i64>().ok());
    if !force && last == Some(seq) {
        return Ok(None);
    }
    let snap = state.store.export_snapshot().await?;
    let files = notai_core::vault::render(
        &snap.topics,
        &snap.entries,
        &snap.tasks,
        &snap.facts,
        state.cfg.tz,
    );
    let changed = tokio::task::spawn_blocking(move || write_files(&root, &files))
        .await
        .map_err(|e| VaultError::Io {
            path: "kasa".into(),
            source: std::io::Error::other(e.to_string()),
        })??;
    state.store.kv_set(SEQ_KEY, &seq.to_string()).await?;
    Ok(Some(changed))
}

/// Zamanlayıcı turu için: yalnız değişiklik varsa yazar.
pub async fn sync_vault(state: &AppState) -> Result<Option<usize>, VaultError> {
    write_vault(state, false).await
}

fn write_files(root: &Path, files: &[notai_core::vault::VaultFile]) -> Result<usize, VaultError> {
    let manifest_path = root.join(MANIFEST);
    let old: BTreeSet<String> = std::fs::read_to_string(&manifest_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let mut new = BTreeSet::new();
    let mut changed = 0;
    for f in files {
        let path = safe_join(root, &f.path)?;
        new.insert(f.path.clone());
        if std::fs::read_to_string(&path).ok().as_deref() == Some(f.content.as_str()) {
            continue;
        }
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(io(dir))?;
        }
        // Yarım dosya kalmasın: önce geçici dosyaya yaz, sonra yerine taşı.
        let tmp = path.with_extension("md.notai-tmp");
        std::fs::write(&tmp, &f.content).map_err(io(&tmp))?;
        std::fs::rename(&tmp, &path).map_err(io(&path))?;
        changed += 1;
    }
    // Artık üretilmeyen (ör. yeniden adlandırılan konu) NotAI dosyalarını sil; boş kalan
    // klasörleri temizle. Kullanıcının dosyaları manifestte olmadığı için dokunulmaz.
    for gone in old.difference(&new) {
        let path = safe_join(root, gone)?;
        if path.exists() {
            std::fs::remove_file(&path).map_err(io(&path))?;
            changed += 1;
        }
        let mut dir = path.parent().map(Path::to_path_buf);
        while let Some(d) = dir {
            if d == root || std::fs::remove_dir(&d).is_err() {
                break;
            }
            dir = d.parent().map(Path::to_path_buf);
        }
    }
    if let Some(dir) = manifest_path.parent() {
        std::fs::create_dir_all(dir).map_err(io(dir))?;
    }
    let json = serde_json::to_string_pretty(&new).unwrap_or_else(|_| "[]".into());
    std::fs::write(&manifest_path, json).map_err(io(&manifest_path))?;
    Ok(changed)
}
