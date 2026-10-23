use axum::{
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{header, HeaderValue},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use tokio_util::io::ReaderStream;
use utoipa::IntoParams;

use super::created;
use crate::{
    auth::{access, AdminUser, AuthUser},
    errors::{AppError, AppResult},
    models::{submission::FileMeta, user::User},
    repositories::resources::{self as resource_repo, ResourceFilter},
    services::storage,
    AppState,
};

#[derive(sqlx::FromRow)]
struct FileRow {
    id: String,
    uploaded_by: String,
    original_name: String,
    content_type: String,
    size_bytes: i64,
    storage_key: String,
}

/// Decides whether `user` may read the file, based on what references it.
async fn can_read(state: &AppState, user: &User, f: &FileRow) -> AppResult<bool> {
    if user.is_admin() || f.uploaded_by == user.id {
        return Ok(true);
    }
    // Attachment of the student's own submission.
    let own: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM submission_attachments sa JOIN submissions s ON s.id = sa.submission_id WHERE sa.file_id = ? AND s.student_id = ?",
    )
    .bind(&f.id)
    .bind(&user.id)
    .fetch_one(&state.pool)
    .await?;
    if own > 0 {
        return Ok(true);
    }
    // A resource the student can see.
    let resource_ids: Vec<String> =
        sqlx::query_scalar("SELECT id FROM resources WHERE file_id = ?").bind(&f.id).fetch_all(&state.pool).await?;
    let today = state.today();
    for rid in resource_ids {
        let filter = ResourceFilter { id: Some(rid), ..Default::default() };
        if !resource_repo::list(&state.pool, user, &filter, &today).await?.is_empty() {
            return Ok(true);
        }
    }
    // Cover image of a visible course.
    let courses: Vec<String> =
        sqlx::query_scalar("SELECT id FROM courses WHERE cover_file_id = ?").bind(&f.id).fetch_all(&state.pool).await?;
    for cid in courses {
        if access::course_for(&state.pool, user, &cid).await.is_ok() {
            return Ok(true);
        }
    }
    Ok(false)
}

#[derive(Deserialize, IntoParams)]
pub struct DownloadQuery {
    /// Force `Content-Disposition: attachment`.
    #[serde(default)]
    pub download: bool,
}

fn content_disposition(kind: &str, name: &str) -> String {
    let ascii: String =
        name.chars().map(|c| if c.is_ascii_graphic() || c == ' ' { c } else { '_' }).filter(|c| *c != '"' && *c != '\\').collect();
    let encoded: String = name
        .bytes()
        .map(|b| if b.is_ascii_alphanumeric() || b"-._~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") })
        .collect();
    format!("{kind}; filename=\"{ascii}\"; filename*=UTF-8''{encoded}")
}

#[utoipa::path(get, path = "/api/v1/files/{id}", tag = "files", params(("id" = String, Path), DownloadQuery),
    responses((status = 200, description = "File content"), (status = 404)))]
pub async fn download(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
    Query(q): Query<DownloadQuery>,
) -> AppResult<Response> {
    let f: FileRow = sqlx::query_as("SELECT id, uploaded_by, original_name, content_type, size_bytes, storage_key FROM files WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound("File"))?;
    if !can_read(&state, &user, &f).await? {
        return Err(AppError::NotFound("File"));
    }
    let path = storage::path_for(&state.config.storage_dir, &f.storage_key);
    let file = tokio::fs::File::open(&path).await.map_err(|_| AppError::NotFound("File"))?;

    let is_pdf = f.content_type == "application/pdf";
    let is_image = f.content_type.starts_with("image/");
    let is_text = storage::is_text_type(&f.content_type);
    let inline = !q.download && (is_pdf || is_image || is_text);
    // Text is always served as plain text so nothing can execute in the browser.
    let served_type = if is_text { "text/plain; charset=utf-8".to_string() } else { f.content_type.clone() };
    let csp = if is_pdf {
        "default-src 'none'; object-src 'self'; frame-ancestors 'self'"
    } else {
        "default-src 'none'; img-src 'self'; style-src 'unsafe-inline'; frame-ancestors 'self'; sandbox"
    };

    let mut res = Body::from_stream(ReaderStream::new(file)).into_response();
    let h = res.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_str(&served_type).map_err(|e| AppError::Internal(e.into()))?);
    h.insert(header::CONTENT_LENGTH, HeaderValue::from(f.size_bytes as u64));
    h.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&content_disposition(if inline { "inline" } else { "attachment" }, &f.original_name))
            .map_err(|e| AppError::Internal(e.into()))?,
    );
    h.insert(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static(csp));
    h.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("SAMEORIGIN"));
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("private, max-age=300"));
    Ok(res)
}

#[utoipa::path(post, path = "/api/v1/files", tag = "files",
    request_body(content_type = "multipart/form-data", description = "Field `file`"), responses((status = 201, body = FileMeta)))]
pub async fn upload(State(state): State<AppState>, admin: AdminUser, mut mp: Multipart) -> AppResult<Response> {
    let field = mp
        .next_field()
        .await
        .map_err(|e| AppError::bad_request(format!("Invalid upload: {e}")))?
        .ok_or_else(|| AppError::bad_request("No file was uploaded"))?;
    let policy = storage::UploadPolicy { allowed_exts: vec![], max_bytes: state.config.max_upload_mb * 1024 * 1024 };
    let meta = storage::store_field(&state, field, &admin.id, &policy).await?;
    Ok(created(meta))
}

#[cfg(test)]
mod tests {
    use super::content_disposition;

    #[test]
    fn disposition_is_header_safe() {
        let v = content_disposition("attachment", "Ölçüm \"verisi\".csv");
        assert!(v.starts_with("attachment; filename=\"_l_\u{fc}m verisi.csv\"") || v.contains("filename*=UTF-8''"));
        assert!(!v.contains('\n'));
        assert!(v.contains("%C3%96"));
    }
}
