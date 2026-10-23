//! Private file storage with type allow-listing and content validation.
//!
//! Files are written under `STORAGE_DIR/files/<prefix>/<uuid>` with no extension and are
//! only reachable through the authorization-checked `/api/v1/files/:id` endpoint.

use std::path::{Path, PathBuf};

use axum::extract::multipart::Field;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use crate::{
    errors::{AppError, AppResult},
    models::submission::FileMeta,
    util::{new_id, now},
    AppState,
};

#[derive(Clone, Copy)]
enum Check {
    Pdf,
    Image(&'static str),
    Text,
    Zip,
}

/// (extension, canonical content type, validation)
const TYPES: &[(&str, &str, Check)] = &[
    ("pdf", "application/pdf", Check::Pdf),
    ("png", "image/png", Check::Image("image/png")),
    ("jpg", "image/jpeg", Check::Image("image/jpeg")),
    ("jpeg", "image/jpeg", Check::Image("image/jpeg")),
    ("gif", "image/gif", Check::Image("image/gif")),
    ("webp", "image/webp", Check::Image("image/webp")),
    ("csv", "text/csv", Check::Text),
    ("tsv", "text/tab-separated-values", Check::Text),
    ("txt", "text/plain", Check::Text),
    ("md", "text/markdown", Check::Text),
    ("json", "application/json", Check::Text),
    ("ipynb", "application/x-ipynb+json", Check::Text),
    ("py", "text/x-python", Check::Text),
    ("c", "text/x-c", Check::Text),
    ("h", "text/x-c", Check::Text),
    ("cpp", "text/x-c++", Check::Text),
    ("hpp", "text/x-c++", Check::Text),
    ("ino", "text/x-arduino", Check::Text),
    ("rs", "text/x-rust", Check::Text),
    ("js", "text/javascript", Check::Text),
    ("ts", "text/x-typescript", Check::Text),
    ("m", "text/x-matlab", Check::Text),
    ("v", "text/x-verilog", Check::Text),
    ("sv", "text/x-systemverilog", Check::Text),
    ("vhd", "text/x-vhdl", Check::Text),
    ("cir", "text/x-spice", Check::Text),
    ("net", "text/x-spice", Check::Text),
    ("kicad_sch", "text/x-kicad", Check::Text),
    ("kicad_pcb", "text/x-kicad", Check::Text),
    ("kicad_pro", "application/json", Check::Text),
    ("zip", "application/zip", Check::Zip),
    ("docx", "application/vnd.openxmlformats-officedocument.wordprocessingml.document", Check::Zip),
    ("xlsx", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", Check::Zip),
    ("pptx", "application/vnd.openxmlformats-officedocument.presentationml.presentation", Check::Zip),
];

pub fn supported_extensions() -> Vec<&'static str> {
    TYPES.iter().map(|(e, _, _)| *e).collect()
}

pub fn normalize_ext(ext: &str) -> String {
    ext.trim().trim_start_matches('.').to_ascii_lowercase()
}

fn lookup(ext: &str) -> Option<(&'static str, Check)> {
    TYPES.iter().find(|(e, _, _)| *e == ext).map(|(_, ct, c)| (*ct, *c))
}

pub fn is_text_type(content_type: &str) -> bool {
    content_type.starts_with("text/") || content_type.contains("json")
}

/// Strips paths, control characters and unusual symbols from a client-supplied filename.
pub fn sanitize_filename(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or("");
    let mut out = String::with_capacity(base.len());
    for c in base.chars() {
        if c.is_alphanumeric() || matches!(c, '.' | '-' | '_' | ' ' | '(' | ')') {
            out.push(c);
        } else if !c.is_control() {
            out.push('_');
        }
    }
    let trimmed = out.trim().trim_matches('.').trim();
    let mut s: String = trimmed.chars().collect();
    if s.chars().count() > 120 {
        // Keep the extension when truncating.
        let ext = Path::new(&s).extension().and_then(|e| e.to_str()).map(|e| e.to_string());
        let mut head: String = s.chars().take(100).collect();
        if let Some(ext) = ext {
            head.push('.');
            head.push_str(&ext.chars().take(15).collect::<String>());
        }
        s = head;
    }
    if s.is_empty() || s.starts_with('.') {
        format!("file{s}")
    } else {
        s
    }
}

pub struct UploadPolicy {
    /// Restrict to these extensions (normalized). Empty = all supported types.
    pub allowed_exts: Vec<String>,
    pub max_bytes: u64,
}

fn files_dir(root: &Path) -> PathBuf {
    root.join("files")
}

pub fn path_for(root: &Path, storage_key: &str) -> PathBuf {
    files_dir(root).join(&storage_key[..2]).join(storage_key)
}

/// Streams a multipart field to private storage after validating type, size and content.
pub async fn store_field(state: &AppState, mut field: Field<'_>, uploader_id: &str, policy: &UploadPolicy) -> AppResult<FileMeta> {
    let original = sanitize_filename(field.file_name().unwrap_or("upload"));
    let ext = Path::new(&original).extension().and_then(|e| e.to_str()).map(normalize_ext).unwrap_or_default();
    let (content_type, check) = lookup(&ext).ok_or_else(|| AppError::bad_request(format!("Files of type '.{ext}' are not supported")))?;
    if !policy.allowed_exts.is_empty() && !policy.allowed_exts.iter().any(|a| a == &ext) {
        return Err(AppError::bad_request(format!(
            "Only these file types are accepted: {}",
            policy.allowed_exts.iter().map(|e| format!(".{e}")).collect::<Vec<_>>().join(", ")
        )));
    }
    let max_bytes = policy.max_bytes.min(state.config.max_upload_mb * 1024 * 1024);

    let root = &state.config.storage_dir;
    let tmp_dir = root.join("tmp");
    tokio::fs::create_dir_all(&tmp_dir).await?;
    let storage_key = uuid::Uuid::new_v4().simple().to_string();
    let tmp_path = tmp_dir.join(&storage_key);

    let result = async {
        let mut file = tokio::fs::File::create(&tmp_path).await?;
        let mut hasher = Sha256::new();
        let mut size: u64 = 0;
        let mut head: Vec<u8> = Vec::with_capacity(8192);
        while let Some(chunk) = field.chunk().await.map_err(|e| AppError::bad_request(format!("Upload failed: {e}")))? {
            size += chunk.len() as u64;
            if size > max_bytes {
                return Err(AppError::PayloadTooLarge(format!("File exceeds the maximum size of {} MB", max_bytes / (1024 * 1024))));
            }
            if head.len() < 8192 {
                let take = (8192 - head.len()).min(chunk.len());
                head.extend_from_slice(&chunk[..take]);
            }
            hasher.update(&chunk);
            file.write_all(&chunk).await?;
        }
        file.flush().await?;
        if size == 0 {
            return Err(AppError::bad_request("The uploaded file is empty"));
        }
        validate_content(check, &head, &tmp_path).await?;
        Ok((size, hex::encode(hasher.finalize())))
    }
    .await;

    let (size, sha) = match result {
        Ok(v) => v,
        Err(e) => {
            let _ = tokio::fs::remove_file(&tmp_path).await;
            return Err(e);
        }
    };

    let final_path = path_for(root, &storage_key);
    if let Some(parent) = final_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::rename(&tmp_path, &final_path).await?;

    let meta = FileMeta {
        id: new_id(),
        original_name: original,
        content_type: content_type.to_string(),
        size_bytes: size as i64,
        created_at: now(),
    };
    let insert = sqlx::query(
        "INSERT INTO files (id, uploaded_by, original_name, content_type, size_bytes, sha256, storage_key, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&meta.id)
    .bind(uploader_id)
    .bind(&meta.original_name)
    .bind(&meta.content_type)
    .bind(meta.size_bytes)
    .bind(&sha)
    .bind(&storage_key)
    .bind(&meta.created_at)
    .execute(&state.pool)
    .await;
    if let Err(e) = insert {
        let _ = tokio::fs::remove_file(&final_path).await;
        return Err(e.into());
    }
    Ok(meta)
}

async fn validate_content(check: Check, head: &[u8], path: &Path) -> AppResult<()> {
    let ok = match check {
        Check::Pdf => head.starts_with(b"%PDF-"),
        Check::Image(expected) => infer::get(head).is_some_and(|t| t.mime_type() == expected),
        Check::Zip => head.starts_with(b"PK\x03\x04"),
        Check::Text => {
            let bytes = tokio::fs::read(path).await?;
            !bytes.contains(&0) && std::str::from_utf8(&bytes).is_ok()
        }
    };
    if ok {
        Ok(())
    } else {
        Err(AppError::bad_request("The file content does not match its extension"))
    }
}

/// Deletes a file's metadata and binary when nothing references it any more.
pub async fn delete_if_orphaned(state: &AppState, file_id: &str) -> AppResult<()> {
    let refs: i64 = sqlx::query_scalar(
        "SELECT (SELECT COUNT(*) FROM submission_attachments WHERE file_id = ?1)
              + (SELECT COUNT(*) FROM resources WHERE file_id = ?1)
              + (SELECT COUNT(*) FROM courses WHERE cover_file_id = ?1)
              + (SELECT COUNT(*) FROM submission_revisions WHERE attachments LIKE '%' || ?1 || '%')",
    )
    .bind(file_id)
    .fetch_one(&state.pool)
    .await?;
    if refs > 0 {
        return Ok(());
    }
    let key: Option<String> =
        sqlx::query_scalar("SELECT storage_key FROM files WHERE id = ?").bind(file_id).fetch_optional(&state.pool).await?;
    if let Some(key) = key {
        sqlx::query("DELETE FROM files WHERE id = ?").bind(file_id).execute(&state.pool).await?;
        let _ = tokio::fs::remove_file(path_for(&state.config.storage_dir, &key)).await;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_filenames() {
        assert_eq!(sanitize_filename("../../etc/passwd"), "passwd");
        assert_eq!(sanitize_filename("C:\\Users\\x\\report.pdf"), "report.pdf");
        assert_eq!(sanitize_filename("a<b>c.txt"), "a_b_c.txt");
        assert_eq!(sanitize_filename(".hidden"), "hidden");
        assert_eq!(sanitize_filename(""), "file");
        let long = format!("{}.csv", "x".repeat(300));
        let s = sanitize_filename(&long);
        assert!(s.ends_with(".csv") && s.chars().count() <= 120);
    }

    #[test]
    fn extensions_are_normalized() {
        assert_eq!(normalize_ext(".PDF"), "pdf");
        assert!(lookup("svg").is_none(), "SVG must not be accepted (script injection risk)");
    }
}
