//! Handlers cho Nhắc Nhở Tu Học Hàng Ngày — Giai đoạn 55 (v0.9.45).
//!
//! Routes:
//!   GET  /cai-dat/nhac-nho                  — Trang cài đặt nhắc nhở
//!   POST /cai-dat/nhac-nho/cap-nhat          — Lưu cài đặt nhắc nhở
//!   GET  /api/nhac-nho/preferences           — JSON API lấy preferences
//!   POST /api/nhac-nho/test-reminder          — Gửi test notification
//!
//! Background task:
//!   Mỗi giờ check users có daily_niem_reminder=true, chưa niệm hôm nay,
//!   chưa được remind trong ngày hôm nay → insert notification.

use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use sqlx::FromRow;

use crate::AppState;
use crate::handlers::get_user_from_session;
use crate::models::user::User;

#[derive(Debug, Clone, FromRow)]
pub struct NotificationPreferences {
    pub user_id: uuid::Uuid,
    pub daily_niem_reminder: bool,
    pub streak_warning: bool,
    pub email_reminders: bool,
    pub reminder_hour: i16,
    pub reminder_channel: String,
}

#[derive(Debug, Deserialize)]
pub struct ReminderPrefsForm {
    pub daily_niem_reminder: Option<String>,
    pub streak_warning: Option<String>,
    pub email_reminders: Option<String>,
    pub reminder_hour: i16,
    pub reminder_channel: String,
}

/// GET /cai-dat/nhac-nho — Trang cài đặt nhắc nhở.
pub async fn reminder_settings_index(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap?next=/cai-dat/nhac-nho").into_response();
    };

    let prefs: NotificationPreferences = sqlx::query_as(
        "SELECT user_id, daily_niem_reminder, streak_warning, email_reminders,
                reminder_hour, reminder_channel
         FROM notification_preferences WHERE user_id = $1"
    )
    .bind(user.id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(NotificationPreferences {
        user_id: user.id,
        daily_niem_reminder: true,
        streak_warning: true,
        email_reminders: false,
        reminder_hour: 20,
        reminder_channel: "app".into(),
    });

    let html = render_reminder_settings(&user, &prefs, None, None);
    Html(html).into_response()
}

/// POST /cai-dat/nhac-nho/cap-nhat — Lưu cài đặt nhắc nhở.
pub async fn reminder_settings_update(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<ReminderPrefsForm>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap?next=/cai-dat/nhac-nho").into_response();
    };

    if form.reminder_hour < 0 || form.reminder_hour > 23 {
        let prefs = NotificationPreferences {
            user_id: user.id,
            daily_niem_reminder: form.daily_niem_reminder.is_some(),
            streak_warning: form.streak_warning.is_some(),
            email_reminders: form.email_reminders.is_some(),
            reminder_hour: 20,
            reminder_channel: form.reminder_channel.clone(),
        };
        let html = render_reminder_settings(
            &user,
            &prefs,
            Some("Giờ nhắc phải từ 0 đến 23."),
            None,
        );
        return Html(html).into_response();
    }

    if !matches!(form.reminder_channel.as_str(), "app" | "email" | "both") {
        let prefs = NotificationPreferences {
            user_id: user.id,
            daily_niem_reminder: form.daily_niem_reminder.is_some(),
            streak_warning: form.streak_warning.is_some(),
            email_reminders: form.email_reminders.is_some(),
            reminder_hour: form.reminder_hour,
            reminder_channel: "app".into(),
        };
        let html = render_reminder_settings(
            &user,
            &prefs,
            Some("Kênh nhắc không hợp lệ."),
            None,
        );
        return Html(html).into_response();
    }

    // Upsert
    let daily = form.daily_niem_reminder.is_some();
    let streak = form.streak_warning.is_some();
    let email = form.email_reminders.is_some();

    let result = sqlx::query(
        "INSERT INTO notification_preferences
            (user_id, daily_niem_reminder, streak_warning, email_reminders, reminder_hour, reminder_channel)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (user_id) DO UPDATE SET
            daily_niem_reminder = EXCLUDED.daily_niem_reminder,
            streak_warning      = EXCLUDED.streak_warning,
            email_reminders     = EXCLUDED.email_reminders,
            reminder_hour       = EXCLUDED.reminder_hour,
            reminder_channel    = EXCLUDED.reminder_channel,
            updated_at          = NOW()"
    )
    .bind(user.id)
    .bind(daily)
    .bind(streak)
    .bind(email)
    .bind(form.reminder_hour)
    .bind(&form.reminder_channel)
    .execute(&state.pool)
    .await;

    if let Err(e) = result {
        log::error!("❌ Lỗi lưu notification_preferences: {e}");
        let prefs = NotificationPreferences {
            user_id: user.id,
            daily_niem_reminder: daily,
            streak_warning: streak,
            email_reminders: email,
            reminder_hour: form.reminder_hour,
            reminder_channel: form.reminder_channel,
        };
        let html = render_reminder_settings(
            &user,
            &prefs,
            Some("Không thể lưu cài đặt. Vui lòng thử lại."),
            None,
        );
        return Html(html).into_response();
    }

    let prefs = NotificationPreferences {
        user_id: user.id,
        daily_niem_reminder: daily,
        streak_warning: streak,
        email_reminders: email,
        reminder_hour: form.reminder_hour,
        reminder_channel: form.reminder_channel,
    };
    let html = render_reminder_settings(
        &user,
        &prefs,
        None,
        Some("Đã lưu cài đặt nhắc nhở. Nguyện công đức vô lượng."),
    );
    Html(html).into_response()
}

/// GET /api/nhac-nho/preferences — JSON API lấy preferences.
pub async fn api_reminder_preferences(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return axum::response::Json(serde_json::json!({
            "success": false, "message": "Vui lòng đăng nhập."
        }))
        .into_response();
    };

    let prefs: NotificationPreferences = sqlx::query_as(
        "SELECT user_id, daily_niem_reminder, streak_warning, email_reminders,
                reminder_hour, reminder_channel
         FROM notification_preferences WHERE user_id = $1"
    )
    .bind(user.id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(NotificationPreferences {
        user_id: user.id,
        daily_niem_reminder: true,
        streak_warning: true,
        email_reminders: false,
        reminder_hour: 20,
        reminder_channel: "app".into(),
    });

    axum::response::Json(serde_json::json!({
        "success": true,
        "preferences": {
            "daily_niem_reminder": prefs.daily_niem_reminder,
            "streak_warning": prefs.streak_warning,
            "email_reminders": prefs.email_reminders,
            "reminder_hour": prefs.reminder_hour,
            "reminder_channel": prefs.reminder_channel
        }
    }))
    .into_response()
}

/// POST /api/nhac-nho/test-reminder — Gửi test notification.
pub async fn api_test_reminder(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return axum::response::Json(serde_json::json!({
            "success": false, "message": "Vui lòng đăng nhập."
        }))
        .into_response();
    };

    let result = sqlx::query(
        "INSERT INTO notifications (user_id, type, title, body, is_read, created_at)
         VALUES ($1, 'reminder', 'Test Reminder', 'Đây là nhắc nhở test từ Ứng Dụng Từ Bi 🪷', false, NOW())"
    )
    .bind(user.id)
    .execute(&state.pool)
    .await;

    if let Err(e) = result {
        log::error!("❌ Lỗi gửi test reminder: {e}");
        return axum::response::Json(serde_json::json!({
            "success": false, "message": "Không thể gửi test reminder."
        }))
        .into_response();
    }

    axum::response::Json(serde_json::json!({
        "success": true,
        "message": "Đã gửi test reminder. Kiểm tra trong /ban-be/thong-bao."
    }))
    .into_response()
}

/// Render trang cài đặt nhắc nhở.
fn render_reminder_settings(
    user: &User,
    prefs: &NotificationPreferences,
    error: Option<&str>,
    success: Option<&str>,
) -> String {
    let daily_checked = if prefs.daily_niem_reminder { "checked" } else { "" };
    let streak_checked = if prefs.streak_warning { "checked" } else { "" };
    let email_checked = if prefs.email_reminders { "checked" } else { "" };
    let channel_app = if prefs.reminder_channel == "app" { "selected" } else { "" };
    let channel_email = if prefs.reminder_channel == "email" { "selected" } else { "" };
    let channel_both = if prefs.reminder_channel == "both" { "selected" } else { "" };

    format!(
        r#"<!DOCTYPE html>
<html lang="vi">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Nhắc Nhở Tu Học — Ứng Dụng Từ Bi</title>
<script src="https://cdn.tailwindcss.com"></script>
</head>
<body class="bg-gray-50 min-h-screen">
<div class="max-w-2xl mx-auto px-4 py-8">
    <div class="text-center mb-6">
        <span class="text-5xl">🪷</span>
        <h1 class="text-2xl font-bold text-emerald-800 mt-2">Nhắc Nhở Tu Học Hàng Ngày</h1>
        <p class="text-gray-500 text-sm mt-1">Giai đoạn 55 — v0.9.45</p>
    </div>

    {error_banner}
    {success_banner}

    <div class="bg-white rounded-xl shadow p-6 space-y-6">
        <form action="/cai-dat/nhac-nho/cap-nhat" method="POST" class="space-y-5">

            <label class="flex items-start gap-3 p-4 border rounded-lg hover:bg-gray-50 cursor-pointer">
                <input type="checkbox" name="daily_niem_reminder" value="on" class="mt-1 w-5 h-5" {daily_checked}>
                <div>
                    <div class="font-semibold text-gray-800">🔔 Nhắc niệm Phật hàng ngày</div>
                    <p class="text-xs text-gray-500 mt-1">Nếu hôm nay bạn chưa niệm Phật, hệ thống sẽ nhắc vào giờ bạn chọn.</p>
                </div>
            </label>

            <label class="flex items-start gap-3 p-4 border rounded-lg hover:bg-gray-50 cursor-pointer">
                <input type="checkbox" name="streak_warning" value="on" class="mt-1 w-5 h-5" {streak_checked}>
                <div>
                    <div class="font-semibold text-gray-800">🔥 Cảnh báo streak sắp gãy</div>
                    <p class="text-xs text-gray-500 mt-1">Khi chuỗi ngày niệm của bạn bị gián đoạn, hệ thống sẽ nhắc nhở nhẹ nhàng.</p>
                </div>
            </label>

            <label class="flex items-start gap-3 p-4 border rounded-lg hover:bg-gray-50 cursor-pointer">
                <input type="checkbox" name="email_reminders" value="on" class="mt-1 w-5 h-5" {email_checked}>
                <div>
                    <div class="font-semibold text-gray-800">📧 Nhận nhắc qua email</div>
                    <p class="text-xs text-gray-500 mt-1">Gửi nhắc nhở đến <strong>{email}</strong>. Tắt nếu không muốn nhận email.</p>
                </div>
            </label>

            <div>
                <label class="block text-sm font-semibold text-gray-700 mb-2">⏰ Giờ nhắc</label>
                <input type="number" name="reminder_hour" min="0" max="23" value="{hour}"
                    class="w-24 px-3 py-2 border rounded-lg">
                <span class="text-xs text-gray-500 ml-2">giờ (0-23) — giờ địa phương</span>
            </div>

            <div>
                <label class="block text-sm font-semibold text-gray-700 mb-2">📡 Kênh nhắc</label>
                <select name="reminder_channel" class="px-3 py-2 border rounded-lg">
                    <option value="app" {channel_app}>Trong app (notifications)</option>
                    <option value="email" {channel_email}>Email</option>
                    <option value="both" {channel_both}>Cả hai</option>
                </select>
            </div>

            <div class="flex gap-3 pt-2">
                <button type="submit" class="bg-emerald-600 text-white px-6 py-2 rounded-lg hover:bg-emerald-700 transition">
                    💾 Lưu cài đặt
                </button>
                <a href="/cai-dat" class="bg-gray-200 px-6 py-2 rounded-lg hover:bg-gray-300 transition">
                    ← Về cài đặt
                </a>
            </div>
        </form>

        <hr class="border-gray-200">
        <div>
            <h3 class="font-semibold text-gray-800 mb-2">Test nhắc nhở</h3>
            <p class="text-xs text-gray-500 mb-2">Gửi một notification test để kiểm tra hệ thống.</p>
            <button onclick="sendTestReminder()" class="bg-indigo-600 text-white px-4 py-2 rounded-lg hover:bg-indigo-700 transition text-sm">
                📨 Gửi test reminder
            </button>
            <div id="test-result" class="mt-2 text-sm"></div>
        </div>
    </div>

    <div class="text-center mt-6">
        <a href="/cai-dat" class="text-sm text-gray-500 hover:text-emerald-700">← Về cài đặt</a>
    </div>
</div>

<script>
async function sendTestReminder() {{
    const btn = event.target;
    btn.disabled = true;
    btn.textContent = 'Đang gửi...';
    const result = document.getElementById('test-result');
    try {{
        const res = await fetch('/api/nhac-nho/test-reminder', {{ method: 'POST' }});
        const data = await res.json();
        result.innerHTML = data.success
            ? '<span class="text-emerald-700">✅ ' + data.message + '</span>'
            : '<span class="text-red-700">⚠️ ' + data.message + '</span>';
    }} catch(e) {{
        result.innerHTML = '<span class="text-red-700">⚠️ Lỗi mạng.</span>';
    }}
    btn.disabled = false;
    btn.textContent = '📨 Gửi test reminder';
}}
</script>
</body>
</html>"#,
        email = user.email,
        hour = prefs.reminder_hour,
        error_banner = error.map(|m| format!(
            "<div class='bg-red-50 border border-red-200 text-red-800 rounded-xl p-4 text-sm mb-4'>⚠️ {m}</div>"
        )).unwrap_or_default(),
        success_banner = success.map(|m| format!(
            "<div class='bg-green-50 border border-green-200 text-green-800 rounded-xl p-4 text-sm mb-4'>✅ {m}</div>"
        )).unwrap_or_default(),
    )
}
