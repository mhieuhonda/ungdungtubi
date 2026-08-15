//! Handlers cho trang Cài Đặt — Giai đoạn 18 (v0.9.14).
//!
//! Routes:
//!   - GET  /cai-dat          — Trang cài đặt cá nhân
//!   - POST /cai-dat/cap-nhat — Cập nhật cài đặt

use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use axum_extra::extract::CookieJar;
use askama::Template;
use serde::Deserialize;
use sqlx::FromRow;

use crate::AppState;
use crate::handlers::get_user_from_session;
use crate::models::user::User;

/// User settings struct (đồng bộ với migration 017).
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct UserSettings {
    pub user_id: uuid::Uuid,
    pub profile_visibility: String,
    pub show_balance: bool,
    pub show_activity: bool,
    pub show_email: bool,
    pub notify_friends: bool,
    pub notify_mail: bool,
    pub notify_dm: bool,
    pub notify_group: bool,
    pub notify_system: bool,
    pub theme: String,
    pub language: String,
    pub auto_join_global_chat: bool,
    pub chat_sound_enabled: bool,
    pub niem_sound_enabled: bool,
    pub niem_auto_convert_k: bool,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            user_id: uuid::Uuid::nil(),
            profile_visibility: "public".into(),
            show_balance: true,
            show_activity: true,
            show_email: false,
            notify_friends: true,
            notify_mail: true,
            notify_dm: true,
            notify_group: true,
            notify_system: true,
            theme: "lotus".into(),
            language: "vi".into(),
            auto_join_global_chat: false,
            chat_sound_enabled: true,
            niem_sound_enabled: true,
            niem_auto_convert_k: true,
        }
    }
}

/// Form cập nhật cài đặt (từ POST /cai-dat/cap-nhat).
#[derive(Debug, Deserialize)]
pub struct SettingsUpdateForm {
    pub profile_visibility: String,
    pub show_balance: Option<String>,
    pub show_activity: Option<String>,
    pub show_email: Option<String>,
    pub notify_friends: Option<String>,
    pub notify_mail: Option<String>,
    pub notify_dm: Option<String>,
    pub notify_group: Option<String>,
    pub notify_system: Option<String>,
    pub theme: String,
    pub language: String,
    pub auto_join_global_chat: Option<String>,
    pub chat_sound_enabled: Option<String>,
    pub niem_sound_enabled: Option<String>,
    pub niem_auto_convert_k: Option<String>,
}

/// Template cho trang /cai-dat.
#[derive(Template)]
#[template(path = "cai-dat/index.html")]
pub struct CaiDatTemplate {
    pub user: Option<User>,
    pub active_page: String,
    pub settings: UserSettings,
    pub error: Option<String>,
    pub success: Option<String>,
}

/// GET /cai-dat — Trang cài đặt cá nhân.
pub async fn cai_dat_index(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap?next=/cai-dat").into_response();
    };

    let settings = fetch_user_settings(&state.pool, user.id).await;

    let html = CaiDatTemplate {
        user: Some(user),
        active_page: "cai_dat".into(),
        settings,
        error: None,
        success: None,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (cai-dat): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

/// POST /cai-dat/cap-nhat — Cập nhật cài đặt.
pub async fn cai_dat_cap_nhat(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<SettingsUpdateForm>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap").into_response();
    };

    // Validate profile_visibility
    let profile_visibility = form.profile_visibility.trim().to_string();
    if !matches!(profile_visibility.as_str(), "public" | "friends" | "private") {
        return render_settings_error(
            &state.pool,
            user,
            "Chế độ hiển thị hồ sơ không hợp lệ.",
        )
        .await;
    }

    // Validate theme
    let theme = form.theme.trim().to_string();
    if !matches!(theme.as_str(), "lotus" | "dark" | "minimal") {
        return render_settings_error(&state.pool, user, "Theme không hợp lệ.").await;
    }

    // Validate language
    let language = form.language.trim().to_string();
    if !matches!(language.as_str(), "vi" | "en" | "zh") {
        return render_settings_error(&state.pool, user, "Ngôn ngữ không hợp lệ.").await;
    }

    // Convert Option<String> ("on" từ checkbox) → bool
    let show_balance = form.show_balance.is_some();
    let show_activity = form.show_activity.is_some();
    let show_email = form.show_email.is_some();
    let notify_friends = form.notify_friends.is_some();
    let notify_mail = form.notify_mail.is_some();
    let notify_dm = form.notify_dm.is_some();
    let notify_group = form.notify_group.is_some();
    let notify_system = form.notify_system.is_some();
    let auto_join_global_chat = form.auto_join_global_chat.is_some();
    let chat_sound_enabled = form.chat_sound_enabled.is_some();
    let niem_sound_enabled = form.niem_sound_enabled.is_some();
    let niem_auto_convert_k = form.niem_auto_convert_k.is_some();

    // Upsert vào DB
    match sqlx::query(
        "INSERT INTO user_settings (
            user_id, profile_visibility, show_balance, show_activity, show_email,
            notify_friends, notify_mail, notify_dm, notify_group, notify_system,
            theme, language, auto_join_global_chat, chat_sound_enabled,
            niem_sound_enabled, niem_auto_convert_k
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
        ON CONFLICT (user_id) DO UPDATE SET
            profile_visibility = EXCLUDED.profile_visibility,
            show_balance = EXCLUDED.show_balance,
            show_activity = EXCLUDED.show_activity,
            show_email = EXCLUDED.show_email,
            notify_friends = EXCLUDED.notify_friends,
            notify_mail = EXCLUDED.notify_mail,
            notify_dm = EXCLUDED.notify_dm,
            notify_group = EXCLUDED.notify_group,
            notify_system = EXCLUDED.notify_system,
            theme = EXCLUDED.theme,
            language = EXCLUDED.language,
            auto_join_global_chat = EXCLUDED.auto_join_global_chat,
            chat_sound_enabled = EXCLUDED.chat_sound_enabled,
            niem_sound_enabled = EXCLUDED.niem_sound_enabled,
            niem_auto_convert_k = EXCLUDED.niem_auto_convert_k,
            updated_at = NOW()",
    )
    .bind(user.id)
    .bind(&profile_visibility)
    .bind(show_balance)
    .bind(show_activity)
    .bind(show_email)
    .bind(notify_friends)
    .bind(notify_mail)
    .bind(notify_dm)
    .bind(notify_group)
    .bind(notify_system)
    .bind(&theme)
    .bind(&language)
    .bind(auto_join_global_chat)
    .bind(chat_sound_enabled)
    .bind(niem_sound_enabled)
    .bind(niem_auto_convert_k)
    .execute(&state.pool)
    .await
    {
        Ok(_) => {
            log::info!("⚙️ User {} cập nhật cài đặt", user.display_name);
        }
        Err(e) => {
            log::error!("❌ Lỗi cập nhật settings: {e}");
            return render_settings_error(
                &state.pool,
                user,
                &format!("Lỗi database: {e}"),
            )
            .await;
        }
    }

    // Re-fetch settings để render
    let settings = fetch_user_settings(&state.pool, user.id).await;

    let html = CaiDatTemplate {
        user: Some(user),
        active_page: "cai_dat".into(),
        settings,
        error: None,
        success: Some("Đã lưu cài đặt. Nguyện công đức vô lượng.".into()),
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (cai-dat success): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });

    Html(html).into_response()
}

// ─── Helpers ──────────────────────────────────────────────────────────────

/// Fetch user settings — trả về default nếu chưa có row.
async fn fetch_user_settings(pool: &sqlx::PgPool, user_id: uuid::Uuid) -> UserSettings {
    match sqlx::query_as::<_, UserSettings>(
        "SELECT user_id, profile_visibility, show_balance, show_activity, show_email,
                notify_friends, notify_mail, notify_dm, notify_group, notify_system,
                theme, language, auto_join_global_chat, chat_sound_enabled,
                niem_sound_enabled, niem_auto_convert_k
         FROM user_settings WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    {
        Ok(Some(s)) => s,
        Ok(None) => {
            // User chưa có row settings → seed default
            let _ = sqlx::query("INSERT INTO user_settings (user_id) VALUES ($1) ON CONFLICT DO NOTHING")
                .bind(user_id)
                .execute(pool)
                .await;
            UserSettings { user_id, ..Default::default() }
        }
        Err(e) => {
            log::warn!("⚠️ Lỗi fetch user_settings: {e}");
            UserSettings { user_id, ..Default::default() }
        }
    }
}

/// Render trang settings với error.
async fn render_settings_error(pool: &sqlx::PgPool, user: User, error: &str) -> Response {
    let settings = fetch_user_settings(pool, user.id).await;
    let html = CaiDatTemplate {
        user: Some(user),
        active_page: "cai_dat".into(),
        settings,
        error: Some(error.into()),
        success: None,
    }
    .render()
    .unwrap_or_else(|e| {
        log::error!("Template render error (cai-dat error): {e}");
        format!("<html><body><h1>Lỗi render template</h1><pre>{e}</pre></body></html>")
    });
    Html(html).into_response()
}
