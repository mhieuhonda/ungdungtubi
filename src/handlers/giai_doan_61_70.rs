//! Handlers cho Giai đoạn 61-70 (v0.9.46) — 10 stages mới.
//!
//! Giai đoạn 61: Tượng Phật Ủng Hộ + Bảng Kính Nguyện
//! Giai đoạn 62: Vòng Quay May Mắn
//! Giai đoạn 63: Bao Lì Xì Từ Bi
//! Giai đoạn 64: Tinh Khí Thần + Kho Đạo Cụ
//! Giai đoạn 65: Nhà Vườn (Lotus Garden)
//! Giai đoạn 66: Đại Sảnh + Cộng Tu
//! Giai đoạn 67: Nhà Truyền Tống
//! Giai đoạn 68: Sự Kiện Phật Lịch
//! Giai đoạn 69: Huy Hiệu Thành Tích
//! Giai đoạn 70: Bảng Vinh Danh
//!
//! Routes tổng hợp (xem src/main.rs):
//!   GET  /tuong-phat/ung-ho             — Xem quảng cáo nhận lượt quay (Stage 61)
//!   GET  /tuong-phat/kinh-nguyen        — Bảng kính nguyện công khai (Stage 61)
//!   POST /api/tuong-phat/ung-ho         — Hook xem ad xong, ghi lượt quay
//!   GET  /vong-quay-may-man             — Trang vòng quay (Stage 62)
//!   POST /api/vong-quay/quay            — Quay vòng (transaction-safe)
//!   GET  /api/vong-quay/prizes          — JSON prize list
//!   GET  /bao-li-xi                     — Trang bao lì xì (Stage 63)
//!   POST /api/bao-li-xi/tao             — Tạo bao lì xì (trừ K)
//!   POST /api/bao-li-xi/{id}/nhan       — Nhận bao lì xì (cộng K, atomic)
//!   GET  /kho-dao-cu                    — Trang kho đạo cụ + Tinh Khí Thần (Stage 64)
//!   POST /api/kho-dao-cu/{code}/dung   — Sử dụng đạo cụ (vd: nuốt Tinh Thể +1 TKThần)
//!   GET  /nha-vuon                      — Trang nhà vườn (Stage 65)
//!   POST /api/nha-vuon/trong/{slot}/{plant_code} — Trồng cây
//!   POST /api/nha-vuon/tuoi/{slot}      — Tưới nước (tăng trưởng nhanh)
//!   POST /api/nha-vuon/thuhoach/{slot}  — Thu hoạch (+A)
//!   GET  /dai-sanh                      — Trang đại sảnh + phiên cộng tu (Stage 66)
//!   POST /dai-sanh/tao-phien            — Tạo phiên cộng tu
//!   POST /dai-sanh/{session_id}/tham-gia/{seat} — Tham gia cộng tu
//!   GET  /nha-truyen-tong               — Trang nhà truyền tống (Stage 67)
//!   POST /api/nha-truyen-tong/di        — Truyền tống đến user/group theo ID
//!   GET  /su-kien                       — Trang sự kiện Phật lịch (Stage 68)
//!   POST /api/su-kien/{event_id}/nhan   — Nhận thưởng sự kiện (atomic)
//!   GET  /api/huy-hieu/{user_id}        — JSON huy hiệu user (Stage 69)
//!   POST /api/huy-hieu/check            — Auto-check + award huy hiệu cho user
//!   GET  /bang-vinh-danh                — Trang bảng vinh danh (Stage 70)

use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Json, Redirect, Response},
    Form,
};
use axum_extra::extract::CookieJar;
use chrono::{DateTime, Datelike, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::AppState;
use crate::handlers::get_user_from_session;

// ════════════════════════════════════════════════════════════════════════
// STAGE 61 — Tượng Phật Ủng Hộ + Bảng Kính Nguyện
// ════════════════════════════════════════════════════════════════════════

const DAILY_SUPPORT_MAX: i16 = 10;

/// GET /tuong-phat/ung-ho — Trang Ủng Hộ (xem quảng cáo).
/// User có thể xem tối đa 10 quảng cáo/ngày — mỗi lượt = 1 lượt quay Vòng May Mắn.
pub async fn tuong_phat_ung_ho_page(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap?next=/tuong-phat/ung-ho").into_response();
    };

    // Số lượt Ủng Hộ đã dùng hôm nay + lượt quay chưa dùng
    let today_support: i16 = sqlx::query_scalar(
        "SELECT COALESCE(support_count, 0) FROM buddha_daily_uses WHERE user_id = $1 AND use_date = CURRENT_DATE"
    )
    .bind(user.id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten()
    .unwrap_or(0);

    let unused_spins: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM lucky_spin_grants WHERE user_id = $1 AND is_used = false"
    )
    .bind(user.id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);

    let html = format!(r##"<!DOCTYPE html>
<html lang="vi"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0, viewport-fit=cover">
<title>🙏 Ủng Hộ — Tượng Phật — Ứng Dụng Từ Bi</title>
<link rel="stylesheet" href="/static/css/app.css">
<script src="https://cdn.tailwindcss.com"></script>
</head><body class="bg-amber-50 min-h-screen">
<section class="max-w-3xl mx-auto px-4 py-8">
  <nav class="text-xs text-gray-500 mb-4"><a href="/khong-gian" class="hover:text-amber-700">🌍 Không Gian</a> / 🙏 Ủng Hộ</nav>
  <div class="bg-gradient-to-br from-amber-100 to-yellow-50 rounded-2xl p-6 shadow-lg border border-amber-200">
    <h1 class="text-2xl font-bold text-amber-900 mb-2">🙏 Tượng Phật — Ủng Hộ</h1>
    <p class="text-sm text-amber-800 mb-6">Xem quảng cáo để nhận lượt Quay Vòng May Mắn. Tối đa {max_ad}/ngày.</p>
    <div class="grid grid-cols-2 gap-4 mb-6">
      <div class="bg-white rounded-xl p-4 text-center border border-amber-200">
        <div class="text-3xl font-bold text-amber-700">{today}</div>
        <div class="text-xs text-amber-600 mt-1">Đã ủng hộ hôm nay</div>
      </div>
      <div class="bg-white rounded-xl p-4 text-center border border-amber-200">
        <div class="text-3xl font-bold text-emerald-700">{unused}</div>
        <div class="text-xs text-emerald-600 mt-1">Lượt quay chưa dùng</div>
      </div>
    </div>
    <div class="bg-white rounded-xl p-5 border border-amber-200">
      <h2 class="font-semibold text-amber-900 mb-3">📺 Xem quảng cáo</h2>
      <p class="text-sm text-gray-600 mb-4">Demo: 1 quảng cáo = 1 lượt quay Vòng May Mắn. (Tích hợp ad network thực tế ở giai đoạn sau.)</p>
      <button id="watch-ad-btn" onclick="watchAd()" class="w-full px-5 py-3 rounded-xl bg-amber-600 hover:bg-amber-700 text-white font-semibold transition">
        ▶️ Xem Quảng Cáo (còn {remaining_ad} lượt)
      </button>
      <div id="ad-result" class="mt-3 text-sm text-center"></div>
    </div>
    <div class="mt-4 flex flex-col sm:flex-row gap-3">
      <a href="/vong-quay-may-man" class="flex-1 text-center px-4 py-2.5 rounded-xl bg-indigo-600 hover:bg-indigo-700 text-white font-semibold transition">🎯 Đến Vòng Quay May Mắn</a>
      <a href="/tuong-phat/kinh-nguyen" class="flex-1 text-center px-4 py-2.5 rounded-xl bg-white border border-amber-300 text-amber-700 font-semibold hover:bg-amber-50 transition">📜 Bảng Kính Nguyện</a>
    </div>
  </div>
</section>
<script>
async function watchAd() {{
  const btn = document.getElementById('watch-ad-btn');
  const res = document.getElementById('ad-result');
  btn.disabled = true; btn.textContent = '⏳ Đang xem quảng cáo...';
  res.textContent = ''; res.className = 'mt-3 text-sm text-center';
  try {{
    const r = await fetch('/api/tuong-phat/ung-ho', {{ method: 'POST' }});
    const d = await r.json();
    if (d.ok) {{
      res.textContent = '✅ ' + d.message;
      res.className = 'mt-3 text-sm text-center text-emerald-700 font-semibold';
      setTimeout(() => location.reload(), 1200);
    }} else {{
      res.textContent = '⚠️ ' + (d.error || 'Lỗi không xác định');
      res.className = 'mt-3 text-sm text-center text-red-700';
      btn.disabled = false; btn.textContent = '▶️ Xem Quảng Cáo';
    }}
  }} catch (e) {{
    res.textContent = '⚠️ Lỗi mạng: ' + e.message;
    res.className = 'mt-3 text-sm text-center text-red-700';
    btn.disabled = false; btn.textContent = '▶️ Xem Quảng Cáo';
  }}
}}
</script>
</body></html>"##,
        max_ad = DAILY_SUPPORT_MAX,
        today = today_support,
        remaining_ad = (DAILY_SUPPORT_MAX - today_support).max(0),
        unused = unused_spins,
    );

    Html(html).into_response()
}

/// POST /api/tuong-phat/ung-ho — Hook sau khi user xem ad xong.
/// Ghi +1 vào buddha_daily_uses.support_count + INSERT lucky_spin_grants (1 lượt quay).
/// Transaction-safe: nếu INSERT grant fail, không tăng support_count.
pub async fn api_tuong_phat_ung_ho(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({"ok": false, "error": "unauthorized"})).into_response();
    };

    let mut tx = match state.pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            return Json(serde_json::json!({"ok": false, "error": format!("DB tx: {e}")})).into_response();
        }
    };

    // Upsert daily_uses
    let today_support: i16 = sqlx::query_scalar(
        "INSERT INTO buddha_daily_uses (user_id, use_date, support_count)
         VALUES ($1, CURRENT_DATE, 1)
         ON CONFLICT (user_id, use_date)
         DO UPDATE SET support_count = buddha_daily_uses.support_count + 1,
                       updated_at = NOW()
         RETURNING support_count"
    )
    .bind(user.id)
    .fetch_one(&mut *tx)
    .await
    .unwrap_or(0);

    if today_support > DAILY_SUPPORT_MAX {
        let _ = tx.rollback().await;
        return Json(serde_json::json!({
            "ok": false,
            "error": "Đã đạt giới hạn ủng hộ 10 lượt/ngày"
        })).into_response();
    }

    // INSERT lucky_spin_grant
    if let Err(e) = sqlx::query(
        "INSERT INTO lucky_spin_grants (user_id, source) VALUES ($1, 'ad_watch')"
    )
    .bind(user.id)
    .execute(&mut *tx)
    .await
    {
        let _ = tx.rollback().await;
        return Json(serde_json::json!({"ok": false, "error": format!("INSERT grant: {e}")})).into_response();
    }

    if let Err(e) = tx.commit().await {
        return Json(serde_json::json!({"ok": false, "error": format!("commit: {e}")})).into_response();
    }

    Json(serde_json::json!({
        "ok": true,
        "message": format!("Đã nhận 1 lượt quay! Tổng ủng hộ hôm nay: {}/{}", today_support, DAILY_SUPPORT_MAX)
    })).into_response()
}

/// GET /tuong-phat/kinh-nguyen — Bảng Kính Nguyện công khai.
/// Hiển thị 50 vow gần đây (prayer/repentance/dedication) của tất cả user.
pub async fn tuong_phat_kinh_nguyen_page(State(state): State<AppState>, jar: CookieJar) -> Response {
    let _user = get_user_from_session(&state.pool, &jar).await;

    #[derive(FromRow)]
    struct VowRow {
        vow_type: String,
        content: String,
        display_name: String,
        created_at: DateTime<Utc>,
    }

    let rows: Vec<VowRow> = sqlx::query_as(
        "SELECT b.vow_type, b.content, COALESCE(u.display_name, 'Ẩn danh') AS display_name, b.created_at
         FROM kinh_nguyen_board b
         JOIN users u ON u.id = b.user_id
         WHERE b.is_public = true
         ORDER BY b.created_at DESC LIMIT 50"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let rows_html = rows.iter().map(|r| {
        let (icon, color) = match r.vow_type.as_str() {
            "prayer" => ("🙏", "border-amber-200 bg-amber-50"),
            "repentance" => ("💧", "border-sky-200 bg-sky-50"),
            "dedication" => ("🌸", "border-pink-200 bg-pink-50"),
            _ => ("🪷", "border-gray-200 bg-gray-50"),
        };
        let time_ago = format_ago(r.created_at);
        let content = html_escape(&r.content);
        let name = html_escape(&r.display_name);
        format!(r#"<div class="rounded-xl border p-3 {color} flex items-start gap-3">
          <span class="text-2xl">{icon}</span>
          <div class="flex-1 min-w-0"><div class="text-sm text-gray-800 break-words">{content}</div>
          <div class="text-xs text-gray-500 mt-1">— {name} · {time_ago}</div></div>
        </div>"#)
    }).collect::<Vec<_>>().join("");

    let html = format!(r##"<!DOCTYPE html>
<html lang="vi"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0, viewport-fit=cover">
<title>📜 Bảng Kính Nguyện — Ứng Dụng Từ Bi</title>
<link rel="stylesheet" href="/static/css/app.css">
<script src="https://cdn.tailwindcss.com"></script>
</head><body class="bg-amber-50 min-h-screen">
<section class="max-w-3xl mx-auto px-4 py-8">
  <nav class="text-xs text-gray-500 mb-4"><a href="/khong-gian" class="hover:text-amber-700">🌍 Không Gian</a> / 📜 Kính Nguyện</nav>
  <div class="bg-white rounded-2xl p-5 sm:p-6 shadow-lg border border-amber-200 mb-6">
    <h1 class="text-xl sm:text-2xl font-bold text-amber-900 mb-1">📜 Bảng Kính Nguyện</h1>
    <p class="text-sm text-gray-600">Những lời cầu nguyện, sám hối, hồi hướng của cộng đồng Từ Bi. Nam Mô A Di Đà Phật. 🙏</p>
  </div>
  <div class="space-y-3">{rows_html}</div>
  {empty}
</section>
</body></html>"##,
        rows_html = rows_html,
        empty = if rows.is_empty() { r#"<div class="text-center py-12 text-gray-400"><span class="text-5xl block mb-3">🪷</span>Chưa có lời kính nguyện nào.</div>"# } else { "" }
    );

    Html(html).into_response()
}

// ════════════════════════════════════════════════════════════════════════
// STAGE 62 — Vòng Quay May Mắn
// ════════════════════════════════════════════════════════════════════════

#[derive(FromRow, Serialize)]
struct LuckyWheelPrize {
    id: i64,
    code: String,
    label: String,
    emoji: String,
    reward_type: String,
    reward_amount: i64,
    weight: f64,
    is_active: bool,
}

/// GET /vong-quay-may-man — Trang Vòng Quay.
pub async fn vong_quay_may_man_page(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap?next=/vong-quay-may-man").into_response();
    };

    let prizes: Vec<LuckyWheelPrize> = sqlx::query_as(
        "SELECT id, code, label, emoji, reward_type, reward_amount, weight, is_active
         FROM lucky_wheel_prizes WHERE is_active = true ORDER BY weight DESC"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let unused_spins: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM lucky_spin_grants WHERE user_id = $1 AND is_used = false"
    )
    .bind(user.id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);

    let free_quota_used: i16 = sqlx::query_scalar(
        "SELECT COALESCE(free_spins_used, 0) FROM lucky_wheel_daily_quota WHERE user_id = $1 AND quota_date = CURRENT_DATE"
    )
    .bind(user.id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten()
    .unwrap_or(0);

    let free_remaining: i16 = (1 - free_quota_used).max(0);
    let total_remaining = free_remaining as i64 + unused_spins;

    let prizes_json = serde_json::to_string(&prizes).unwrap_or_else(|_| "[]".to_string());

    let html = format!(r##"<!DOCTYPE html>
<html lang="vi"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0, viewport-fit=cover">
<title>🎯 Vòng Quay May Mắn — Ứng Dụng Từ Bi</title>
<link rel="stylesheet" href="/static/css/app.css">
<script src="https://cdn.tailwindcss.com"></script>
</head><body class="bg-indigo-50 min-h-screen">
<section class="max-w-2xl mx-auto px-4 py-8">
  <nav class="text-xs text-gray-500 mb-4"><a href="/khong-gian" class="hover:text-indigo-700">🌍 Không Gian</a> / 🎯 Vòng Quay</nav>
  <div class="bg-white rounded-2xl p-5 sm:p-6 shadow-lg border border-indigo-200">
    <h1 class="text-xl sm:text-2xl font-bold text-indigo-900 mb-1">🎯 Vòng Quay May Mắn</h1>
    <p class="text-sm text-gray-600 mb-4">Quay để nhận A · K · Tinh Thể · Đạo cụ · và nhiều phần thưởng khác!</p>
    <div class="bg-indigo-50 rounded-xl p-3 mb-4 flex items-center justify-between">
      <span class="text-sm text-indigo-800 font-medium">Lượt quay còn lại:</span>
      <span class="text-2xl font-bold text-indigo-700" id="spin-count">{total}</span>
    </div>
    <div class="bg-gradient-to-br from-yellow-100 to-amber-200 rounded-2xl p-8 mb-4 text-center">
      <div class="text-7xl mb-2">🎡</div>
      <div id="spin-result" class="text-lg font-semibold text-amber-900 min-h-[2rem]">Bấm nút bên dưới để quay!</div>
    </div>
    <button id="spin-btn" onclick="doSpin()" {disabled}
            class="w-full px-5 py-3 rounded-xl bg-indigo-600 hover:bg-indigo-700 text-white font-semibold transition disabled:opacity-50 disabled:cursor-not-allowed">
      🎯 QUAY NGAY
    </button>
    <div class="mt-4 text-center">
      <a href="/tuong-phat/ung-ho" class="text-sm text-amber-700 hover:text-amber-900">🙏 Xem quảng cáo để nhận thêm lượt quay →</a>
    </div>
    <div class="mt-6 pt-4 border-t border-gray-100">
      <h2 class="text-sm font-bold text-gray-700 mb-2">🎁 Bảng phần thưởng</h2>
      <div class="grid grid-cols-2 gap-2 text-xs" id="prize-list"></div>
    </div>
  </div>
</section>
<script>
const prizes = {prizes_json};
const prizeList = document.getElementById('prize-list');
prizes.forEach(p => {{
  const div = document.createElement('div');
  div.className = 'flex items-center gap-2 p-2 rounded-lg bg-gray-50';
  div.innerHTML = '<span class="text-lg">' + p.emoji + '</span><div><div class="font-semibold text-gray-800">' + p.label + '</div><div class="text-gray-500">' + p.weight + '%</div></div>';
  prizeList.appendChild(div);
}});
async function doSpin() {{
  const btn = document.getElementById('spin-btn');
  const result = document.getElementById('spin-result');
  const count = document.getElementById('spin-count');
  btn.disabled = true; btn.textContent = '⏳ Đang quay...';
  result.textContent = '🎲 Đang quay...';
  try {{
    const r = await fetch('/api/vong-quay/quay', {{ method: 'POST' }});
    const d = await r.json();
    if (d.ok) {{
      result.innerHTML = d.prize.emoji + ' ' + d.prize.label + (d.reward_text ? '<br><span class="text-sm">' + d.reward_text + '</span>' : '');
      count.textContent = d.remaining;
      if (d.remaining == 0) {{ btn.textContent = 'Hết lượt quay'; }}
      else {{ btn.disabled = false; btn.textContent = '🎯 QUAY NGAY'; }}
    }} else {{
      result.textContent = '⚠️ ' + (d.error || 'Lỗi');
      btn.disabled = false; btn.textContent = '🎯 QUAY NGAY';
    }}
  }} catch (e) {{
    result.textContent = '⚠️ Lỗi mạng: ' + e.message;
    btn.disabled = false; btn.textContent = '🎯 QUAY NGAY';
  }}
}}
</script>
</body></html>"##,
        total = total_remaining,
        disabled = if total_remaining == 0 { "disabled" } else { "" },
        prizes_json = prizes_json
    );

    Html(html).into_response()
}

/// POST /api/vong-quay/quay — Quay vòng. Transaction-safe.
pub async fn api_vong_quay_quay(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({"ok": false, "error": "unauthorized"})).into_response();
    };

    let mut tx = match state.pool.begin().await {
        Ok(tx) => tx,
        Err(e) => return Json(serde_json::json!({"ok": false, "error": format!("DB: {e}")})).into_response(),
    };

    // 1. Upsert daily quota, try free spin first
    let free_used: i16 = sqlx::query_scalar(
        "INSERT INTO lucky_wheel_daily_quota (user_id, quota_date, free_spins_used)
         VALUES ($1, CURRENT_DATE, 1)
         ON CONFLICT (user_id, quota_date)
         DO UPDATE SET free_spins_used = lucky_wheel_daily_quota.free_spins_used + 1
         RETURNING free_spins_used"
    )
    .bind(user.id)
    .fetch_one(&mut *tx)
    .await
    .unwrap_or(99);  // large = no free spin

    let mut source = "free_daily";

    if free_used > 1 {
        // Free quota already used → try grant from ad_watch
        let grant: Option<(i64,)> = sqlx::query_as(
            "SELECT id FROM lucky_spin_grants WHERE user_id = $1 AND is_used = false
             ORDER BY created_at ASC LIMIT 1 FOR UPDATE"
        )
        .bind(user.id)
        .fetch_optional(&mut *tx)
        .await
        .ok()
        .flatten();

        if let Some((id,)) = grant {
            let _ = sqlx::query(
                "UPDATE lucky_spin_grants SET is_used = true, used_at = NOW() WHERE id = $1"
            )
            .bind(id)
            .execute(&mut *tx)
            .await;
            source = "ad_watch";
            // Rollback the quota increment since we're using a grant instead
            let _ = sqlx::query(
                "UPDATE lucky_wheel_daily_quota SET free_spins_used = free_spins_used - 1
                 WHERE user_id = $1 AND quota_date = CURRENT_DATE"
            )
            .bind(user.id)
            .execute(&mut *tx)
            .await;
        } else {
            let _ = tx.rollback().await;
            return Json(serde_json::json!({
                "ok": false,
                "error": "Hết lượt quay! Xem quảng cáo tại trang Ủng Hộ để nhận thêm."
            })).into_response();
        }
    }

    // 2. Load all active prizes with weights
    let prizes: Vec<(i64, String, String, String, i64, f64)> = sqlx::query_as(
        "SELECT id, code, label, emoji, reward_amount, weight FROM lucky_wheel_prizes WHERE is_active = true"
    )
    .fetch_all(&mut *tx)
    .await
    .unwrap_or_default();

    if prizes.is_empty() {
        let _ = tx.rollback().await;
        return Json(serde_json::json!({"ok": false, "error": "Chưa cấu hình phần thưởng"})).into_response();
    }

    let total_weight: f64 = prizes.iter().map(|p| p.5).sum();
    let mut rand_val = rand::random::<f64>() * total_weight;
    let mut chosen: Option<&(i64, String, String, String, i64, f64)> = None;
    for p in &prizes {
        rand_val -= p.5;
        if rand_val <= 0.0 {
            chosen = Some(p);
            break;
        }
    }
    let chosen = chosen.unwrap_or(prizes.last().unwrap());

    let (prize_id, ref prize_code, ref prize_label, ref prize_emoji, prize_amount, _) = *chosen;

    // 3. Apply reward — fetch reward_type separately since tuple order is (id, code, label, emoji, amount, weight)
    let reward_type: String = sqlx::query_scalar(
        "SELECT reward_type FROM lucky_wheel_prizes WHERE id = $1"
    )
    .bind(prize_id)
    .fetch_one(&mut *tx)
    .await
    .unwrap_or_else(|_| "nothing".to_string());

    let mut reward_text = String::new();
    let rt = reward_type.as_str();
    match rt {
        "a" => {
            let _ = sqlx::query("UPDATE users SET a_balance = a_balance + $2, updated_at = NOW() WHERE id = $1")
                .bind(user.id).bind(prize_amount).execute(&mut *tx).await;
            let _ = sqlx::query("INSERT INTO balance_transactions (user_id, currency, amount, direction, reason) VALUES ($1, 'a', $2, 'in', 'vong_quay_may_man')")
                .bind(user.id).bind(prize_amount).execute(&mut *tx).await;
            reward_text = format!("+{} A (Niệm Lực)", prize_amount);
        }
        "k" => {
            let _ = sqlx::query("UPDATE users SET k_balance = k_balance + $2, updated_at = NOW() WHERE id = $1")
                .bind(user.id).bind(prize_amount).execute(&mut *tx).await;
            let _ = sqlx::query("INSERT INTO balance_transactions (user_id, currency, amount, direction, reason) VALUES ($1, 'k', $2, 'in', 'vong_quay_may_man')")
                .bind(user.id).bind(prize_amount).execute(&mut *tx).await;
            reward_text = format!("+{} K (Tiền app)", prize_amount);
        }
        "bi" => {
            let _ = sqlx::query("UPDATE users SET bi_balance = COALESCE(bi_balance, 0) + $2, updated_at = NOW() WHERE id = $1")
                .bind(user.id).bind(prize_amount).execute(&mut *tx).await;
            // v0.9.47 fix: log balance_transactions cho "bi" case (trước đây thiếu)
            let _ = sqlx::query("INSERT INTO balance_transactions (user_id, currency, amount, direction, reason) VALUES ($1, 'bi', $2, 'in', 'vong_quay_may_man')")
                .bind(user.id).bind(prize_amount).execute(&mut *tx).await;
            reward_text = format!("+{} Bi (Tiền Từ Bi)", prize_amount);
        }
        "item" => {
            // Add to inventory
            let item_id: i64 = sqlx::query_scalar("SELECT id FROM system_items WHERE code = $1")
                .bind(prize_code).fetch_one(&mut *tx).await.unwrap_or(0);
            if item_id > 0 {
                let _ = sqlx::query(
                    "INSERT INTO user_inventories (user_id, item_id, quantity)
                     VALUES ($1, $2, 1)
                     ON CONFLICT (user_id, item_id) DO UPDATE SET quantity = user_inventories.quantity + 1, updated_at = NOW()"
                )
                .bind(user.id).bind(item_id).execute(&mut *tx).await;
            }
            reward_text = format!("Nhận 1 {}", prize_label);
        }
        _ => {
            reward_text = "Chưa may mắn lần này. Hãy quay lại!".to_string();
        }
    }

    // 4. Log spin
    let _ = sqlx::query(
        "INSERT INTO lucky_wheel_spins (user_id, prize_id, source, reward_given, reward_amount)
         VALUES ($1, $2, $3, $4, $5)"
    )
    .bind(user.id).bind(prize_id).bind(source).bind(&reward_type).bind(prize_amount)
    .execute(&mut *tx).await;

    if let Err(e) = tx.commit().await {
        return Json(serde_json::json!({"ok": false, "error": format!("commit: {e}")})).into_response();
    }

    // 5. Count remaining
    let remaining: i64 = sqlx::query_scalar(
        "SELECT
          (SELECT COUNT(*)::BIGINT FROM lucky_spin_grants WHERE user_id = $1 AND is_used = false)
          +
          GREATEST(0, 1 - COALESCE((SELECT free_spins_used FROM lucky_wheel_daily_quota WHERE user_id = $1 AND quota_date = CURRENT_DATE), 0))::BIGINT"
    )
    .bind(user.id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);

    Json(serde_json::json!({
        "ok": true,
        "prize": {
            "code": prize_code,
            "label": prize_label,
            "emoji": prize_emoji,
            "reward_type": reward_type,
            "reward_amount": prize_amount
        },
        "reward_text": reward_text,
        "remaining": remaining
    })).into_response()
}

// ════════════════════════════════════════════════════════════════════════
// STAGE 63 — Bao Lì Xì Từ Bi
// ════════════════════════════════════════════════════════════════════════

#[derive(FromRow, Serialize)]
struct RedEnvelope {
    id: i64,
    creator_name: String,
    envelope_type: String,
    total_k: i64,
    remaining_k: i64,
    total_claims: i16,
    max_claims: i16,
    message: Option<String>,
    created_at: DateTime<Utc>,
}

/// GET /bao-li-xi — Trang Bao Lì Xì.
pub async fn bao_li_xi_page(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap?next=/bao-li-xi").into_response();
    };

    let envelopes: Vec<RedEnvelope> = sqlx::query_as(
        "SELECT r.id, COALESCE(u.display_name, 'Ẩn danh') AS creator_name, r.envelope_type,
                r.total_k, r.remaining_k, r.total_claims, r.max_claims, r.message, r.created_at
         FROM red_envelopes r
         JOIN users u ON u.id = r.creator_id
         WHERE r.is_active = true AND r.remaining_k > 0 AND r.expires_at > NOW()
         ORDER BY r.created_at DESC LIMIT 30"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let env_html = envelopes.iter().map(|e| {
        let type_label = match e.envelope_type.as_str() {
            "dai_bi_100k" => "🧧 Bao Lì Xì Đại Bi (100K)",
            _ => "🧧 Bao Lì Xì Từ Bi (10K)",
        };
        let pct = if e.total_k > 0 { e.remaining_k * 100 / e.total_k } else { 0 };
        let msg = e.message.as_ref().map(|m| format!("<div class='text-xs text-gray-500 italic'>\"{}\"</div>", html_escape(m))).unwrap_or_default();
        format!(r#"<div class="bg-white rounded-2xl p-4 border border-red-200 shadow-sm">
          <div class="flex items-start justify-between mb-2">
            <div><div class="font-bold text-red-700">{type_label}</div>
            <div class="text-xs text-gray-500">từ {creator}</div></div>
            <div class="text-right"><div class="text-lg font-bold text-red-700">{remaining}K</div>
            <div class="text-xs text-gray-500">còn lại / {total}K</div></div>
          </div>
          {msg}
          <div class="w-full bg-gray-100 rounded-full h-2 mb-3"><div class="bg-red-500 h-2 rounded-full" style="width:{pct}%"></div></div>
          <div class="text-xs text-gray-500 mb-3">👥 {claims}/{max_claims} người nhận · {time_ago}</div>
          <button onclick="claim({id})" class="w-full px-3 py-2 rounded-lg bg-red-600 hover:bg-red-700 text-white text-sm font-semibold transition">🧧 Nhận Lì Xì</button>
        </div>"#, 
            type_label = type_label, creator = html_escape(&e.creator_name),
            remaining = e.remaining_k, total = e.total_k, msg = msg, pct = pct,
            claims = e.total_claims, max_claims = e.max_claims, time_ago = format_ago(e.created_at),
            id = e.id)
    }).collect::<Vec<_>>().join("");

    let html = format!(r##"<!DOCTYPE html>
<html lang="vi"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0, viewport-fit=cover">
<title>🧧 Bao Lì Xì Từ Bi — Ứng Dụng Từ Bi</title>
<link rel="stylesheet" href="/static/css/app.css">
<script src="https://cdn.tailwindcss.com"></script>
</head><body class="bg-red-50 min-h-screen">
<section class="max-w-3xl mx-auto px-4 py-8">
  <nav class="text-xs text-gray-500 mb-4"><a href="/" class="hover:text-red-700">🏠 Trang chủ</a> / 🧧 Bao Lì Xì</nav>
  <div class="bg-gradient-to-br from-red-600 to-red-700 rounded-2xl p-6 text-white shadow-lg mb-6">
    <h1 class="text-2xl font-bold mb-1">🧧 Bao Lì Xì Từ Bi</h1>
    <p class="text-sm opacity-90">Tạo bao lì xì chia sẻ K cho cộng đồng. Mỗi người nhận được 1 phần ngẫu nhiên.</p>
    <div class="mt-4 flex flex-col sm:flex-row gap-2">
      <button onclick="create(10)" class="px-4 py-2 rounded-lg bg-white text-red-700 font-semibold hover:bg-amber-50 transition">🧧 Tạo Bao 10K</button>
      <button onclick="create(100)" class="px-4 py-2 rounded-lg bg-amber-400 text-red-900 font-semibold hover:bg-amber-300 transition">🧧 Tạo Bao Đại Bi 100K</button>
    </div>
    <div id="create-result" class="text-sm mt-2"></div>
  </div>
  <h2 class="font-bold text-red-800 mb-3">Bao lì xì đang mở</h2>
  <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">{env_html}</div>
  {empty}
</section>
<script>
async function create(amount) {{
  const res = document.getElementById('create-result');
  res.textContent = '⏳ Đang tạo...'; res.className = 'text-sm mt-2 text-white';
  const r = await fetch('/api/bao-li-xi/tao', {{ method: 'POST', headers: {{'Content-Type':'application/x-www-form-urlencoded'}}, body: 'amount_k=' + amount }});
  const d = await r.json();
  if (d.ok) {{ res.textContent = '✅ ' + d.message; res.className = 'text-sm mt-2 text-amber-200'; setTimeout(()=>location.reload(), 1000); }}
  else {{ res.textContent = '⚠️ ' + (d.error||'Lỗi'); res.className = 'text-sm mt-2 text-yellow-200'; }}
}}
async function claim(id) {{
  const r = await fetch('/api/bao-li-xi/' + id + '/nhan', {{ method: 'POST' }});
  const d = await r.json();
  alert(d.ok ? '🧧 Bạn nhận được ' + d.amount_k + ' K!' : '⚠️ ' + (d.error||'Lỗi'));
  if (d.ok) location.reload();
}}
</script>
</body></html>"##,
        env_html = env_html,
        empty = if envelopes.is_empty() { r#"<div class="text-center py-12 text-gray-400"><span class="text-5xl block mb-3">🧧</span>Chưa có bao lì xì nào. Hãy tạo bao đầu tiên!</div>"# } else { "" }
    );

    Html(html).into_response()
}

/// POST /api/bao-li-xi/tao — Tạo bao lì xì (trừ K từ creator).
#[derive(Deserialize)]
pub struct BaoLiXiCreateForm {
    pub amount_k: i64,
}

pub async fn api_bao_li_xi_tao(State(state): State<AppState>, jar: CookieJar, Form(form): Form<BaoLiXiCreateForm>) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({"ok": false, "error": "unauthorized"})).into_response();
    };

    let amount = form.amount_k;
    if amount != 10 && amount != 100 {
        return Json(serde_json::json!({"ok": false, "error": "Mệnh giá chỉ 10K hoặc 100K"})).into_response();
    }

    let mut tx = match state.pool.begin().await {
        Ok(tx) => tx,
        Err(e) => return Json(serde_json::json!({"ok": false, "error": format!("DB: {e}")})).into_response(),
    };

    // Atomic check-and-subtract
    let new_balance: Option<i64> = sqlx::query_scalar(
        "UPDATE users SET k_balance = k_balance - $2, updated_at = NOW()
         WHERE id = $1 AND k_balance >= $2
         RETURNING k_balance"
    )
    .bind(user.id)
    .bind(amount)
    .fetch_optional(&mut *tx)
    .await
    .ok()
    .flatten();

    if new_balance.is_none() {
        let _ = tx.rollback().await;
        return Json(serde_json::json!({"ok": false, "error": "Không đủ K"})).into_response();
    }

    let env_type = if amount == 100 { "dai_bi_100k" } else { "tubi_10k" };
    let max_claims: i16 = if amount == 100 { 50 } else { 10 };

    if let Err(e) = sqlx::query(
        "INSERT INTO red_envelopes (creator_id, envelope_type, total_k, remaining_k, max_claims)
         VALUES ($1, $2, $3, $3, $4)"
    )
    .bind(user.id).bind(env_type).bind(amount).bind(max_claims)
    .execute(&mut *tx).await
    {
        let _ = tx.rollback().await;
        return Json(serde_json::json!({"ok": false, "error": format!("insert: {e}")})).into_response();
    }

    let _ = sqlx::query(
        "INSERT INTO balance_transactions (user_id, currency, amount, direction, reason)
         VALUES ($1, 'k', $2, 'out', 'bao_li_xi_tao')"
    )
    .bind(user.id).bind(amount).execute(&mut *tx).await;

    if let Err(e) = tx.commit().await {
        return Json(serde_json::json!({"ok": false, "error": format!("commit: {e}")})).into_response();
    }

    Json(serde_json::json!({
        "ok": true,
        "message": format!("Đã tạo bao lì xì {}K — cảm ơn lòng từ bi của bạn! 🧧", amount)
    })).into_response()
}

/// POST /api/bao-li-xi/{id}/nhan — Nhận bao lì xì (atomic).
pub async fn api_bao_li_xi_nhan(State(state): State<AppState>, jar: CookieJar, Path(env_id): Path<i64>) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({"ok": false, "error": "unauthorized"})).into_response();
    };

    let mut tx = match state.pool.begin().await {
        Ok(tx) => tx,
        Err(e) => return Json(serde_json::json!({"ok": false, "error": format!("DB: {e}")})).into_response(),
    };

    // Lock envelope row
    let env: Option<(i64, i16)> = sqlx::query_as(
        "SELECT remaining_k, max_claims FROM red_envelopes
         WHERE id = $1 AND is_active = true AND remaining_k > 0 AND expires_at > NOW()
         FOR UPDATE"
    )
    .bind(env_id)
    .fetch_optional(&mut *tx)
    .await
    .ok()
    .flatten();

    let Some((remaining_k, max_claims)) = env else {
        let _ = tx.rollback().await;
        return Json(serde_json::json!({"ok": false, "error": "Bao lì xì đã hết hoặc đã hết hạn"})).into_response();
    };

    // Check user hasn't claimed yet
    let already: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM red_envelope_claims WHERE envelope_id = $1 AND user_id = $2"
    )
    .bind(env_id).bind(user.id)
    .fetch_one(&mut *tx)
    .await
    .unwrap_or(0);

    if already > 0 {
        let _ = tx.rollback().await;
        return Json(serde_json::json!({"ok": false, "error": "Bạn đã nhận bao lì xì này rồi"})).into_response();
    }

    // Check max_claims not exceeded
    let total_claims: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM red_envelope_claims WHERE envelope_id = $1"
    )
    .bind(env_id)
    .fetch_one(&mut *tx)
    .await
    .unwrap_or(0);

    if total_claims >= max_claims as i64 {
        let _ = tx.rollback().await;
        return Json(serde_json::json!({"ok": false, "error": "Bao lì xì đã đủ lượt nhận"})).into_response();
    }

    // Calculate random amount (between 1 and remaining_k, but at least 1K)
    let amount = if remaining_k <= 1 {
        1
    } else {
        // Distribute remaining across remaining slots; reserve at least 1 for last claimers
        let remaining_slots = (max_claims as i64 - total_claims).max(1);
        let max_for_this = (remaining_k - (remaining_slots - 1)).max(1);
        let min_for_this = 1i64;
        let range = (max_for_this - min_for_this + 1).max(1);
        (rand::random::<u64>() as i64 % range) + min_for_this
    };
    let amount = amount.min(remaining_k).max(1);

    // Deduct from envelope
    let _ = sqlx::query(
        "UPDATE red_envelopes SET remaining_k = remaining_k - $2, total_claims = total_claims + 1 WHERE id = $1"
    )
    .bind(env_id).bind(amount)
    .execute(&mut *tx).await;

    // Add to user K balance
    let _ = sqlx::query(
        "UPDATE users SET k_balance = k_balance + $2, updated_at = NOW() WHERE id = $1"
    )
    .bind(user.id).bind(amount)
    .execute(&mut *tx).await;

    // Log claim
    let _ = sqlx::query(
        "INSERT INTO red_envelope_claims (envelope_id, user_id, amount_k) VALUES ($1, $2, $3)"
    )
    .bind(env_id).bind(user.id).bind(amount)
    .execute(&mut *tx).await;

    let _ = sqlx::query(
        "INSERT INTO balance_transactions (user_id, currency, amount, direction, reason)
         VALUES ($1, 'k', $2, 'in', 'bao_li_xi_nhan')"
    )
    .bind(user.id).bind(amount)
    .execute(&mut *tx).await;

    if let Err(e) = tx.commit().await {
        return Json(serde_json::json!({"ok": false, "error": format!("commit: {e}")})).into_response();
    }

    Json(serde_json::json!({
        "ok": true,
        "amount_k": amount,
        "message": format!("Bạn nhận được {} K 🧧", amount)
    })).into_response()
}

// ════════════════════════════════════════════════════════════════════════
// STAGE 64 — Tinh Khí Thần + Kho Đạo Cụ
// ════════════════════════════════════════════════════════════════════════

#[derive(FromRow, Serialize)]
struct InventoryItem {
    id: i64,
    code: String,
    name: String,
    emoji: String,
    description: Option<String>,
    price_k: i64,
    category: String,
    quantity: i64,
}

/// GET /kho-dao-cu — Trang Kho Đạo Cụ.
pub async fn kho_dao_cu_page(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap?next=/kho-dao-cu").into_response();
    };

    let tinh_khi_than: i16 = sqlx::query_scalar("SELECT tinh_khi_than FROM users WHERE id = $1")
        .bind(user.id).fetch_one(&state.pool).await.unwrap_or(0);
    let max_tkth: i16 = sqlx::query_scalar("SELECT max_tinh_khi_than FROM users WHERE id = $1")
        .bind(user.id).fetch_one(&state.pool).await.unwrap_or(100);

    let items: Vec<InventoryItem> = sqlx::query_as(
        "SELECT s.id, s.code, s.name, s.emoji, s.description, s.price_k, s.category,
                COALESCE(i.quantity, 0)::BIGINT AS quantity
         FROM system_items s
         LEFT JOIN user_inventories i ON i.item_id = s.id AND i.user_id = $1
         WHERE s.is_active = true
         ORDER BY s.category, s.price_k"
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let items_html = items.iter().map(|it| {
        let can_use = it.quantity > 0;
        let btn = if can_use {
            format!("<button onclick=\"useItem('{}')\" class=\"px-3 py-1.5 rounded-lg bg-amber-600 hover:bg-amber-700 text-white text-xs font-semibold transition\">Dùng</button>", it.code)
        } else {
            format!("<button onclick=\"buy({})\" class=\"px-3 py-1.5 rounded-lg bg-gray-100 hover:bg-gray-200 text-gray-700 text-xs font-semibold transition\">Mua {}K</button>", it.id, it.price_k)
        };
        format!(r#"<div class="bg-white rounded-xl p-3 border border-gray-200 flex items-center gap-3">
          <span class="text-3xl">{emoji}</span>
          <div class="flex-1 min-w-0"><div class="font-semibold text-gray-800 text-sm">{name}</div>
          <div class="text-xs text-gray-500 line-clamp-2">{desc}</div>
          <div class="text-xs text-amber-700 font-bold mt-0.5">Số lượng: {qty}</div></div>
          {btn}
        </div>"#,
            emoji = it.emoji, name = it.name,
            desc = it.description.as_deref().unwrap_or(""),
            qty = it.quantity, btn = btn)
    }).collect::<Vec<_>>().join("");

    let html = format!(r##"<!DOCTYPE html>
<html lang="vi"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0, viewport-fit=cover">
<title>🔮 Kho Đạo Cụ — Ứng Dụng Từ Bi</title>
<link rel="stylesheet" href="/static/css/app.css">
<script src="https://cdn.tailwindcss.com"></script>
</head><body class="bg-violet-50 min-h-screen">
<section class="max-w-3xl mx-auto px-4 py-8">
  <nav class="text-xs text-gray-500 mb-4"><a href="/" class="hover:text-violet-700">🏠 Trang chủ</a> / 🔮 Kho Đạo Cụ</nav>
  <div class="bg-gradient-to-br from-violet-600 to-purple-700 rounded-2xl p-6 text-white shadow-lg mb-6">
    <h1 class="text-2xl font-bold mb-1">🔮 Kho Đạo Cụ & Tinh Khí Thần</h1>
    <p class="text-sm opacity-90">Quản lý Tinh Thể, Tinh Thạch, Linh Thạch, Tiên Thạch. Nuốt Tinh Thể để tăng Tinh Khí Thần.</p>
    <div class="mt-4 bg-white/15 rounded-xl p-3 backdrop-blur">
      <div class="flex justify-between text-sm mb-1"><span>Tinh Khí Thần</span><span>{cur}/{max}</span></div>
      <div class="w-full bg-white/30 rounded-full h-3"><div class="bg-amber-400 h-3 rounded-full" style="width:{pct}%"></div></div>
    </div>
  </div>
  <h2 class="font-bold text-violet-800 mb-3">Đạo cụ của bạn</h2>
  <div class="grid grid-cols-1 sm:grid-cols-2 gap-2">{items_html}</div>
</section>
<script>
async function useItem(code) {{
  if (!confirm('Dùng 1 ' + code + '?')) return;
  const r = await fetch('/api/kho-dao-cu/' + code + '/dung', {{ method: 'POST' }});
  const d = await r.json();
  alert(d.ok ? '✅ ' + (d.message||'Đã sử dụng') : '⚠️ ' + (d.error||'Lỗi'));
  if (d.ok) location.reload();
}}
async function buy(id) {{
  if (!confirm('Mua đạo cụ này?')) return;
  // Buy uses Thương Thành flow — redirect for now
  alert('Vui lòng mua tại Thương Thành → Cửa Hàng Ứng Dụng');
  location.href = '/thuong-thanh/cua-hang-app';
}}
</script>
</body></html>"##,
        cur = tinh_khi_than,
        max = max_tkth,
        pct = if max_tkth > 0 { tinh_khi_than as u64 * 100 / max_tkth as u64 } else { 0 },
        items_html = items_html
    );

    Html(html).into_response()
}

/// POST /api/kho-dao-cu/{code}/dung — Sử dụng đạo cụ.
pub async fn api_kho_dao_cu_dung(State(state): State<AppState>, jar: CookieJar, Path(code): Path<String>) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({"ok": false, "error": "unauthorized"})).into_response();
    };

    let mut tx = match state.pool.begin().await {
        Ok(tx) => tx,
        Err(e) => return Json(serde_json::json!({"ok": false, "error": format!("DB: {e}")})).into_response(),
    };

    // Lock inventory row
    let inv: Option<(i64, i64, String, String, String)> = sqlx::query_as(
        "SELECT i.id, i.quantity, s.code, s.name, s.category
         FROM user_inventories i
         JOIN system_items s ON s.id = i.item_id
         WHERE i.user_id = $1 AND s.code = $2 AND i.quantity > 0
         FOR UPDATE"
    )
    .bind(user.id)
    .bind(&code)
    .fetch_optional(&mut *tx)
    .await
    .ok()
    .flatten();

    let Some((inv_id, qty, _item_code, item_name, category)) = inv else {
        let _ = tx.rollback().await;
        return Json(serde_json::json!({"ok": false, "error": "Không có đạo cụ này trong kho"})).into_response();
    };

    let message = match code.as_str() {
        "tinh_the" => {
            // Nuốt Tinh Thể → +1 Tinh Khí Thần (check max)
            let cur: i16 = sqlx::query_scalar("SELECT tinh_khi_than FROM users WHERE id = $1")
                .bind(user.id).fetch_one(&mut *tx).await.unwrap_or(0);
            let max_tkth: i16 = sqlx::query_scalar("SELECT max_tinh_khi_than FROM users WHERE id = $1")
                .bind(user.id).fetch_one(&mut *tx).await.unwrap_or(100);
            if cur >= max_tkth {
                let _ = tx.rollback().await;
                return Json(serde_json::json!({"ok": false, "error": "Tinh Khí Thần đã đạt tối đa"})).into_response();
            }
            let _ = sqlx::query("UPDATE users SET tinh_khi_than = tinh_khi_than + 1, updated_at = NOW() WHERE id = $1")
                .bind(user.id).execute(&mut *tx).await;
            let _ = sqlx::query("UPDATE user_inventories SET quantity = quantity - 1, updated_at = NOW() WHERE id = $1")
                .bind(inv_id).execute(&mut *tx).await;
            let _ = sqlx::query("INSERT INTO item_use_log (user_id, item_code, quantity_used, effect) VALUES ($1, $2, 1, 'tinh_khi_than +1')")
                .bind(user.id).bind(&code).execute(&mut *tx).await;
            format!("Nuốt {} → Tinh Khí Thần +1 🌟", item_name)
        }
        "the_ung_ho" => {
            // Thẻ Ủng Hộ → +1 lượt quay
            let _ = sqlx::query("INSERT INTO lucky_spin_grants (user_id, source) VALUES ($1, 'ad_watch')")
                .bind(user.id).execute(&mut *tx).await;
            let _ = sqlx::query("UPDATE user_inventories SET quantity = quantity - 1, updated_at = NOW() WHERE id = $1")
                .bind(inv_id).execute(&mut *tx).await;
            let _ = sqlx::query("INSERT INTO item_use_log (user_id, item_code, quantity_used, effect) VALUES ($1, $2, 1, '+1 lượt quay vòng')")
                .bind(user.id).bind(&code).execute(&mut *tx).await;
            format!("Sử dụng {} → +1 lượt quay Vòng May Mắn", item_name)
        }
        "bao_li_xi" => {
            // Bao Lì Xì → +1 bao 10K để chia sẻ (redirect to create)
            let _ = sqlx::query(
                "INSERT INTO red_envelopes (creator_id, envelope_type, total_k, remaining_k, max_claims)
                 VALUES ($1, 'tubi_10k', 10, 10, 10)"
            )
            .bind(user.id).execute(&mut *tx).await;
            let _ = sqlx::query("UPDATE user_inventories SET quantity = quantity - 1, updated_at = NOW() WHERE id = $1")
                .bind(inv_id).execute(&mut *tx).await;
            format!("Sử dụng {} → Đã tạo bao lì xì 10K trên cộng đồng!", item_name)
        }
        _ => {
            let _ = tx.rollback().await;
            return Json(serde_json::json!({"ok": false, "error": "Đạo cụ này chưa hỗ trợ sử dụng"})).into_response();
        }
    };
    let _ = (qty, category);

    if let Err(e) = tx.commit().await {
        return Json(serde_json::json!({"ok": false, "error": format!("commit: {e}")})).into_response();
    }

    Json(serde_json::json!({"ok": true, "message": message})).into_response()
}

// ════════════════════════════════════════════════════════════════════════
// STAGE 65 — Nhà Vườn (Lotus Garden)
// ════════════════════════════════════════════════════════════════════════

#[derive(FromRow, Serialize)]
struct GardenSlot {
    slot_index: i16,
    plant_code: Option<String>,
    plant_name: Option<String>,
    plant_emoji: Option<String>,
    planted_at: Option<DateTime<Utc>>,
    is_ready: bool,
    ready_at: Option<DateTime<Utc>>,
    growth_seconds: Option<i64>,
    reward_a: Option<i64>,
}

/// GET /nha-vuon — Trang Nhà Vườn.
pub async fn nha_vuon_page(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Redirect::to("/dang-nhap?next=/nha-vuon").into_response();
    };

    // Ensure user has a garden
    let _ = sqlx::query(
        "INSERT INTO user_gardens (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING"
    )
    .bind(user.id)
    .execute(&state.pool)
    .await;

    let garden_id: i64 = sqlx::query_scalar("SELECT id FROM user_gardens WHERE user_id = $1")
        .bind(user.id).fetch_one(&state.pool).await.unwrap_or(0);

    let max_slots: i16 = sqlx::query_scalar("SELECT max_slots FROM user_gardens WHERE id = $1")
        .bind(garden_id).fetch_one(&state.pool).await.unwrap_or(9);

    // Ensure slots exist
    for i in 1..=(max_slots as i32) {
        let _ = sqlx::query(
            "INSERT INTO garden_slots (garden_id, slot_index) VALUES ($1, $2) ON CONFLICT (garden_id, slot_index) DO NOTHING"
        )
        .bind(garden_id).bind(i).execute(&state.pool).await;
    }

    // Auto-mark ready if ready_at passed
    let _ = sqlx::query(
        "UPDATE garden_slots SET is_ready = true
         WHERE garden_id = $1 AND plant_type_id IS NOT NULL AND is_ready = false
         AND ready_at IS NOT NULL AND ready_at <= NOW()"
    )
    .bind(garden_id)
    .execute(&state.pool)
    .await;

    let slots: Vec<GardenSlot> = sqlx::query_as(
        "SELECT s.slot_index, p.code AS plant_code, p.name AS plant_name, p.emoji AS plant_emoji,
                s.planted_at, s.is_ready, s.ready_at,
                p.growth_seconds::BIGINT AS growth_seconds, p.reward_a::BIGINT AS reward_a
         FROM garden_slots s
         LEFT JOIN garden_plant_types p ON p.id = s.plant_type_id
         WHERE s.garden_id = $1
         ORDER BY s.slot_index"
    )
    .bind(garden_id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let slots_json = serde_json::to_string(&slots).unwrap_or_else(|_| "[]".to_string());

    // Plant types available
    let plant_types: Vec<(String, String, String, i64, i64)> = sqlx::query_as(
        "SELECT code, name, emoji, cost_k::BIGINT, reward_a::BIGINT FROM garden_plant_types WHERE is_active = true ORDER BY growth_seconds"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let plant_options = plant_types.iter().map(|(c, n, e, _cost, rew)| {
        format!("<option value=\"{}\">{} {} (+{} A)</option>", c, e, n, rew)
    }).collect::<Vec<_>>().join("");

    let slots_html = (1..=max_slots).map(|idx| {
        let slot = slots.iter().find(|s| s.slot_index == idx);
        if let Some(s) = slot {
            if s.plant_code.is_none() {
                format!(r##"<div class="bg-gray-50 rounded-xl p-3 border-2 border-dashed border-gray-200 text-center min-h-[120px] flex flex-col items-center justify-center">
                  <span class="text-3xl text-gray-300 mb-1">🪴</span>
                  <span class="text-xs text-gray-400">Ô trống #{}</span>
                </div>"##, idx)
            } else if s.is_ready {
                format!(r##"<div class="bg-emerald-50 rounded-xl p-3 border-2 border-emerald-300 text-center min-h-[120px] flex flex-col items-center justify-center">
                  <span class="text-4xl mb-1">{}</span>
                  <span class="text-xs font-semibold text-emerald-700 mb-2">Sẵn sàng!</span>
                  <button onclick="harvest({})" class="px-3 py-1 rounded-lg bg-emerald-600 hover:bg-emerald-700 text-white text-xs font-semibold">Thu hoạch</button>
                </div>"##, s.plant_emoji.as_deref().unwrap_or("🪷"), idx)
            } else {
                format!(r##"<div class="bg-amber-50 rounded-xl p-3 border-2 border-amber-200 text-center min-h-[120px] flex flex-col items-center justify-center">
                  <span class="text-3xl mb-1 opacity-70">{}</span>
                  <span class="text-xs text-amber-700 mb-1">{}</span>
                  <span class="text-xs text-gray-500" data-ready-at="{}"></span>
                </div>"##, s.plant_emoji.as_deref().unwrap_or("🌱"), s.plant_name.as_deref().unwrap_or(""), s.ready_at.map(|d| d.to_rfc3339()).unwrap_or_default())
            }
        } else {
            format!(r##"<div class="bg-gray-50 rounded-xl p-3 border-2 border-dashed border-gray-200 text-center min-h-[120px] flex flex-col items-center justify-center">
              <span class="text-3xl text-gray-300 mb-1">🪴</span>
              <span class="text-xs text-gray-400">Ô trống #{}</span>
            </div>"##, idx)
        }
    }).collect::<Vec<_>>().join("");

    let html = format!(r##"<!DOCTYPE html>
<html lang="vi"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0, viewport-fit=cover">
<title>🪷 Nhà Vườn — Ứng Dụng Từ Bi</title>
<link rel="stylesheet" href="/static/css/app.css">
<script src="https://cdn.tailwindcss.com"></script>
</head><body class="bg-green-50 min-h-screen">
<section class="max-w-3xl mx-auto px-4 py-8">
  <nav class="text-xs text-gray-500 mb-4"><a href="/khong-gian" class="hover:text-green-700">🌍 Không Gian</a> / 🪷 Nhà Vườn</nav>
  <div class="bg-gradient-to-br from-green-600 to-emerald-700 rounded-2xl p-5 sm:p-6 text-white shadow-lg mb-4">
    <h1 class="text-xl sm:text-2xl font-bold mb-1">🪷 Nhà Vườn</h1>
    <p class="text-sm opacity-90">Trồng hoa sen, tưới nước, thu hoạch Niệm Lực A. Tu cũng niệm Phật, chăm hoa cũng niệm Phật.</p>
  </div>
  <div class="bg-white rounded-2xl p-4 border border-green-200 shadow-sm mb-4">
    <h2 class="text-sm font-bold text-gray-700 mb-2">🌱 Trồng cây mới</h2>
    <div class="flex gap-2">
      <select id="plant-type" class="flex-1 px-3 py-2 border border-gray-200 rounded-lg text-sm">{plant_options}</select>
      <select id="plant-slot" class="px-3 py-2 border border-gray-200 rounded-lg text-sm">
        {slot_opts}
      </select>
      <button onclick="plant()" class="px-4 py-2 rounded-lg bg-green-600 hover:bg-green-700 text-white text-sm font-semibold transition">🌱 Trồng</button>
    </div>
    <div id="plant-result" class="text-xs mt-2"></div>
  </div>
  <div class="grid grid-cols-3 gap-2 sm:gap-3">{slots_html}</div>
</section>
<script>
const slots = {slots_json};
function updateCountdowns() {{
  document.querySelectorAll('[data-ready-at]').forEach(el => {{
    const ready = el.getAttribute('data-ready-at');
    if (!ready) return;
    const target = new Date(ready);
    const now = new Date();
    const diff = target - now;
    if (diff <= 0) {{ el.textContent = '✓ Đã sẵn sàng'; el.className = 'text-xs text-emerald-600 font-semibold'; }}
    else {{
      const m = Math.floor(diff / 60000); const s = Math.floor((diff % 60000)/1000);
      el.textContent = '⏳ ' + m + 'm ' + s + 's';
    }}
  }});
}}
updateCountdowns(); setInterval(updateCountdowns, 1000);
async function plant() {{
  const code = document.getElementById('plant-type').value;
  const slot = document.getElementById('plant-slot').value;
  const res = document.getElementById('plant-result');
  const r = await fetch('/api/nha-vuon/trong/' + slot + '/' + code, {{ method: 'POST' }});
  const d = await r.json();
  res.textContent = d.ok ? '✅ ' + (d.message||'Đã trồng') : '⚠️ ' + (d.error||'Lỗi');
  res.className = 'text-xs mt-2 ' + (d.ok ? 'text-emerald-700' : 'text-red-700');
  if (d.ok) setTimeout(()=>location.reload(), 800);
}}
async function harvest(slot) {{
  const r = await fetch('/api/nha-vuon/thuhoach/' + slot, {{ method: 'POST' }});
  const d = await r.json();
  alert(d.ok ? '🎉 ' + d.message : '⚠️ ' + (d.error||'Lỗi'));
  if (d.ok) location.reload();
}}
</script>
</body></html>"##,
        plant_options = plant_options,
        slot_opts = (1..=max_slots).map(|i| format!("<option value=\"{}\">Ô {}</option>", i, i)).collect::<Vec<_>>().join(""),
        slots_html = slots_html,
        slots_json = slots_json
    );

    Html(html).into_response()
}

/// POST /api/nha-vuon/trong/{slot}/{code} — Trồng cây.
pub async fn api_nha_vuon_trong(
    State(state): State<AppState>,
    jar: CookieJar,
    Path((slot, code)): Path<(i16, String)>,
) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({"ok": false, "error": "unauthorized"})).into_response();
    };

    let mut tx = match state.pool.begin().await {
        Ok(tx) => tx,
        Err(e) => return Json(serde_json::json!({"ok": false, "error": format!("DB: {e}")})).into_response(),
    };

    // Check slot is empty + get garden_id
    let garden_id: i64 = sqlx::query_scalar("SELECT id FROM user_gardens WHERE user_id = $1")
        .bind(user.id).fetch_one(&mut *tx).await.unwrap_or(0);

    let empty: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM garden_slots WHERE garden_id = $1 AND slot_index = $2 AND plant_type_id IS NULL FOR UPDATE"
    )
    .bind(garden_id).bind(slot)
    .fetch_optional(&mut *tx).await.ok().flatten();

    if empty.is_none() {
        let _ = tx.rollback().await;
        return Json(serde_json::json!({"ok": false, "error": "Ô đã có cây hoặc không tồn tại"})).into_response();
    }

    // Get plant type
    let plant: Option<(i64, i64, i64, i64)> = sqlx::query_as(
        "SELECT id, growth_seconds::BIGINT, cost_k::BIGINT, reward_a::BIGINT
         FROM garden_plant_types WHERE code = $1 AND is_active = true"
    )
    .bind(&code).fetch_optional(&mut *tx).await.ok().flatten();

    let Some((plant_id, growth_secs, cost_k, _reward_a)) = plant else {
        let _ = tx.rollback().await;
        return Json(serde_json::json!({"ok": false, "error": "Loại cây không hợp lệ"})).into_response();
    };

    // If cost_k > 0, deduct from user
    if cost_k > 0 {
        let new_bal: Option<i64> = sqlx::query_scalar(
            "UPDATE users SET k_balance = k_balance - $2 WHERE id = $1 AND k_balance >= $2 RETURNING k_balance"
        )
        .bind(user.id).bind(cost_k).fetch_optional(&mut *tx).await.ok().flatten();
        if new_bal.is_none() {
            let _ = tx.rollback().await;
            return Json(serde_json::json!({"ok": false, "error": format!("Không đủ K (cần {} K)", cost_k)})).into_response();
        }
    }

    // Plant
    let _ = sqlx::query(
        "UPDATE garden_slots SET plant_type_id = $3, planted_at = NOW(), is_ready = false,
                                   ready_at = NOW() + ($4 || ' seconds')::INTERVAL
         WHERE garden_id = $1 AND slot_index = $2"
    )
    .bind(garden_id).bind(slot).bind(plant_id).bind(growth_secs.to_string())
    .execute(&mut *tx).await;

    if let Err(e) = tx.commit().await {
        return Json(serde_json::json!({"ok": false, "error": format!("commit: {e}")})).into_response();
    }

    Json(serde_json::json!({"ok": true, "message": format!("Đã trồng cây ở ô {} 🌱", slot)})).into_response()
}

/// POST /api/nha-vuon/thuhoach/{slot} — Thu hoạch.
pub async fn api_nha_vuon_thuhoach(State(state): State<AppState>, jar: CookieJar, Path(slot): Path<i16>) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({"ok": false, "error": "unauthorized"})).into_response();
    };

    let mut tx = match state.pool.begin().await {
        Ok(tx) => tx,
        Err(e) => return Json(serde_json::json!({"ok": false, "error": format!("DB: {e}")})).into_response(),
    };

    let garden_id: i64 = sqlx::query_scalar("SELECT id FROM user_gardens WHERE user_id = $1")
        .bind(user.id).fetch_one(&mut *tx).await.unwrap_or(0);

    let slot_info: Option<(i64, i64, bool)> = sqlx::query_as(
        "SELECT s.id, p.reward_a::BIGINT, s.is_ready FROM garden_slots s
         JOIN garden_plant_types p ON p.id = s.plant_type_id
         WHERE s.garden_id = $1 AND s.slot_index = $2 AND s.plant_type_id IS NOT NULL AND s.is_ready = true
         FOR UPDATE"
    )
    .bind(garden_id).bind(slot)
    .fetch_optional(&mut *tx).await.ok().flatten();

    let Some((slot_id, reward_a, _ready)) = slot_info else {
        let _ = tx.rollback().await;
        return Json(serde_json::json!({"ok": false, "error": "Ô chưa có cây sẵn sàng để thu hoạch"})).into_response();
    };

    // Clear slot + add A to user
    let _ = sqlx::query("UPDATE garden_slots SET plant_type_id = NULL, planted_at = NULL, is_ready = false, ready_at = NULL WHERE id = $1")
        .bind(slot_id).execute(&mut *tx).await;
    let _ = sqlx::query("UPDATE users SET a_balance = a_balance + $2, updated_at = NOW() WHERE id = $1")
        .bind(user.id).bind(reward_a).execute(&mut *tx).await;
    let _ = sqlx::query("UPDATE user_gardens SET total_harvest = total_harvest + 1, total_a_earned = total_a_earned + $2 WHERE id = $1")
        .bind(garden_id).bind(reward_a).execute(&mut *tx).await;
    let _ = sqlx::query("INSERT INTO balance_transactions (user_id, currency, amount, direction, reason) VALUES ($1, 'a', $2, 'in', 'nha_vuon_thu_hoach')")
        .bind(user.id).bind(reward_a).execute(&mut *tx).await;

    if let Err(e) = tx.commit().await {
        return Json(serde_json::json!({"ok": false, "error": format!("commit: {e}")})).into_response();
    }

    Json(serde_json::json!({"ok": true, "message": format!("Thu hoạch thành công! +{} A 🪷", reward_a)})).into_response()
}

// ════════════════════════════════════════════════════════════════════════
// STAGE 66 — Đại Sảnh + Cộng Tu
// ════════════════════════════════════════════════════════════════════════

/// GET /dai-sanh — Trang Đại Sảnh + danh sách phiên cộng tu.
pub async fn dai_sanh_page(State(state): State<AppState>, jar: CookieJar) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;

    #[derive(FromRow)]
    struct SessionRow {
        id: i64,
        title: String,
        host_name: String,
        start_at: DateTime<Utc>,
        duration_minutes: i16,
        max_seats: i16,
        participant_count: i64,
        is_full: bool,
    }

    let sessions: Vec<SessionRow> = sqlx::query_as(
        "SELECT s.id, s.title, COALESCE(u.display_name, 'Ẩn danh') AS host_name,
                s.start_at, s.duration_minutes, s.max_seats,
                (SELECT COUNT(*)::BIGINT FROM meditation_participants p WHERE p.session_id = s.id) AS participant_count,
                (SELECT COUNT(*)::BIGINT FROM meditation_participants p WHERE p.session_id = s.id) >= s.max_seats AS is_full
         FROM meditation_sessions s
         JOIN users u ON u.id = s.host_id
         WHERE s.is_active = true AND s.is_cancelled = false AND s.start_at > NOW() - INTERVAL '1 hour'
         ORDER BY s.start_at ASC LIMIT 20"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let sessions_html = sessions.iter().map(|s| {
        let seats_remaining = s.max_seats as i64 - s.participant_count;
        let join_btn = if seats_remaining > 0 {
            format!("<button onclick=\"join({})\" class=\"px-3 py-1.5 rounded-lg bg-amber-600 hover:bg-amber-700 text-white text-xs font-semibold\">Tham gia (còn {} ghế)</button>", s.id, seats_remaining)
        } else {
            "<span class='text-xs text-gray-500'>Đã đủ ghế</span>".to_string()
        };
        format!(r#"<div class="bg-white rounded-xl p-3 border border-amber-200">
          <div class="font-semibold text-gray-800 text-sm mb-1">{title}</div>
          <div class="text-xs text-gray-500 mb-2">👤 {host} · ⏰ {time} · ⏱ {dur} phút · 👥 {p}/{max}</div>
          {btn}
        </div>"#,
            title = html_escape(&s.title), host = html_escape(&s.host_name),
            time = s.start_at.format("%H:%M %d/%m"), dur = s.duration_minutes,
            p = s.participant_count, max = s.max_seats, btn = join_btn)
    }).collect::<Vec<_>>().join("");

    let create_form = if user.is_some() {
        r##"<div class="bg-white rounded-2xl p-4 border border-amber-200 mb-4">
          <h2 class="text-sm font-bold text-gray-700 mb-2">⚡ Tạo phiên cộng tu mới</h2>
          <form onsubmit="createSession(event)" class="space-y-2">
            <input type="text" id="session-title" required maxlength="200" placeholder="Tiêu đề (vd: Cộng tu niệm Phật tối)"
                   class="w-full px-3 py-2 border border-gray-200 rounded-lg text-sm">
            <textarea id="session-desc" maxlength="500" placeholder="Mô tả (tùy chọn)"
                      class="w-full px-3 py-2 border border-gray-200 rounded-lg text-sm" rows="2"></textarea>
            <div class="grid grid-cols-2 gap-2">
              <input type="datetime-local" id="session-start" required class="px-3 py-2 border border-gray-200 rounded-lg text-sm">
              <select id="session-duration" class="px-3 py-2 border border-gray-200 rounded-lg text-sm">
                <option value="15">15 phút</option>
                <option value="30" selected>30 phút</option>
                <option value="60">60 phút</option>
                <option value="120">120 phút</option>
              </select>
            </div>
            <button type="submit" class="w-full px-3 py-2 rounded-lg bg-amber-600 hover:bg-amber-700 text-white text-sm font-semibold">⚡ Tạo phiên</button>
          </form>
          <div id="create-result" class="text-xs mt-2"></div>
        </div>"##.to_string()
    } else {
        "<div class='bg-amber-50 rounded-2xl p-4 border border-amber-200 text-amber-800 text-sm mb-4'>Vui lòng đăng nhập để tạo phiên cộng tu.</div>".to_string()
    };

    let html = format!(r##"<!DOCTYPE html>
<html lang="vi"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0, viewport-fit=cover">
<title>🪷 Đại Sảnh — Ứng Dụng Từ Bi</title>
<link rel="stylesheet" href="/static/css/app.css">
<script src="https://cdn.tailwindcss.com"></script>
</head><body class="bg-amber-50 min-h-screen">
<section class="max-w-3xl mx-auto px-4 py-8">
  <nav class="text-xs text-gray-500 mb-4"><a href="/khong-gian" class="hover:text-amber-700">🌍 Không Gian</a> / 🪷 Đại Sảnh</nav>
  <div class="bg-gradient-to-br from-amber-500 to-yellow-600 rounded-2xl p-5 sm:p-6 text-white shadow-lg mb-6 text-center">
    <div class="text-6xl mb-2">🪷</div>
    <h1 class="text-xl sm:text-2xl font-bold">Đại Sảnh — Cộng Tu</h1>
    <p class="text-sm opacity-90 mt-1">Bông sen lớn + Tượng Phật + 10 bồ đoàn. Ngồi cùng nhau niệm Phật, hiệu quả tăng 10 lần.</p>
  </div>
  {create_form}
  <h2 class="font-bold text-amber-800 mb-3">⚡ Phiên cộng tu sắp diễn ra</h2>
  <div class="space-y-3">{sessions_html}</div>
  {empty}
</section>
<script>
async function createSession(e) {{
  e.preventDefault();
  const title = document.getElementById('session-title').value;
  const desc = document.getElementById('session-desc').value;
  const start = document.getElementById('session-start').value;
  const dur = document.getElementById('session-duration').value;
  const res = document.getElementById('create-result');
  const r = await fetch('/dai-sanh/tao-phien', {{
    method: 'POST',
    headers: {{'Content-Type': 'application/x-www-form-urlencoded'}},
    body: new URLSearchParams({{title, description: desc, start_at: start, duration_minutes: dur}})
  }});
  const text = await r.text();
  if (r.ok) {{ res.textContent = '✅ Đã tạo phiên!'; res.className = 'text-xs mt-2 text-emerald-700'; setTimeout(()=>location.reload(), 800); }}
  else {{ res.textContent = '⚠️ ' + text; res.className = 'text-xs mt-2 text-red-700'; }}
}}
async function join(id) {{
  const seat = prompt('Chọn ghế (1-10, 1 = chủ tọa gần tượng Phật):', '2');
  if (!seat) return;
  const r = await fetch('/dai-sanh/' + id + '/tham-gia/' + seat, {{ method: 'POST' }});
  const text = await r.text();
  if (r.ok) {{ alert('✅ Đã tham gia!'); location.reload(); }}
  else {{ alert('⚠️ ' + text); }}
}}
</script>
</body></html>"##,
        create_form = create_form,
        sessions_html = sessions_html,
        empty = if sessions.is_empty() { r#"<div class="text-center py-8 text-gray-400"><span class="text-4xl block mb-2">🪷</span>Chưa có phiên cộng tu nào. Hãy tạo phiên đầu tiên!</div>"# } else { "" }
    );

    Html(html).into_response()
}

#[derive(Deserialize)]
pub struct CreateSessionForm {
    pub title: String,
    pub description: Option<String>,
    pub start_at: String,
    pub duration_minutes: i64,
}

/// POST /dai-sanh/tao-phien — Tạo phiên cộng tu.
pub async fn dai_sanh_tao_phien(State(state): State<AppState>, jar: CookieJar, Form(form): Form<CreateSessionForm>) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return (axum::http::StatusCode::UNAUTHORIZED, "Vui lòng đăng nhập").into_response();
    };

    let start_at = match chrono::DateTime::parse_from_rfc3339(&format!("{}:00", form.start_at.replace(' ', "T"))) {
        Ok(d) => d.with_timezone(&Utc),
        Err(_) => return (axum::http::StatusCode::BAD_REQUEST, "Định dạng thời gian không hợp lệ").into_response(),
    };

    if let Err(e) = sqlx::query(
        "INSERT INTO meditation_sessions (host_id, title, description, start_at, duration_minutes, max_seats)
         VALUES ($1, $2, $3, $4, $5, 10)"
    )
    .bind(user.id).bind(&form.title).bind(form.description.as_deref().unwrap_or(""))
    .bind(start_at).bind(form.duration_minutes as i16)
    .execute(&state.pool).await
    {
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("Lỗi DB: {e}")).into_response();
    }

    Redirect::to("/dai-sanh").into_response()
}

/// POST /dai-sanh/{id}/tham-gia/{seat} — Tham gia phiên cộng tu.
pub async fn dai_sanh_tham_gia(State(state): State<AppState>, jar: CookieJar, Path((session_id, seat)): Path<(i64, i16)>) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return (axum::http::StatusCode::UNAUTHORIZED, "Vui lòng đăng nhập").into_response();
    };

    if seat < 1 || seat > 10 {
        return (axum::http::StatusCode::BAD_REQUEST, "Ghế phải từ 1-10").into_response();
    }

    if let Err(e) = sqlx::query(
        "INSERT INTO meditation_participants (session_id, user_id, seat_number)
         VALUES ($1, $2, $3)
         ON CONFLICT (session_id, seat_number) DO NOTHING"
    )
    .bind(session_id).bind(user.id).bind(seat)
    .execute(&state.pool).await
    {
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("Lỗi DB: {e}")).into_response();
    }

    Redirect::to("/dai-sanh").into_response()
}

// ════════════════════════════════════════════════════════════════════════
// STAGE 67 — Nhà Truyền Tống
// ════════════════════════════════════════════════════════════════════════

/// GET /nha-truyen-tong — Trang Nhà Truyền Tống.
pub async fn nha_truyen_tong_page(State(state): State<AppState>, jar: CookieJar) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;

    let bookmarks_html = if let Some(ref u) = user {
        #[derive(FromRow)]
        struct BookmarkRow {
            id: i64,
            target_type: String,
            target_id: String,
            label: String,
        }
        let rows: Vec<BookmarkRow> = sqlx::query_as(
            "SELECT id, target_type, target_id, label FROM teleport_bookmarks WHERE user_id = $1 ORDER BY created_at DESC"
        )
        .bind(u.id)
        .fetch_all(&state.pool)
        .await
        .unwrap_or_default();

        rows.iter().map(|b| {
            let (icon, url) = match b.target_type.as_str() {
                "user_space" => ("👤", format!("/ca-nhan?id={}", b.target_id)),
                "group_space" => ("👥", format!("/cong-dong/nhom/{}", b.target_id)),
                _ => ("🪷", "/".to_string()),
            };
            format!(r#"<a href="{}" class="flex items-center gap-2 p-2 rounded-lg bg-white border border-gray-200 hover:border-violet-300 transition">
              <span class="text-xl">{}</span><span class="text-sm text-gray-700">{}</span>
            </a>"#, url, icon, html_escape(&b.label))
        }).collect::<Vec<_>>().join("")
    } else {
        String::new()
    };

    let html = format!(r##"<!DOCTYPE html>
<html lang="vi"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0, viewport-fit=cover">
<title>🌀 Nhà Truyền Tống — Ứng Dụng Từ Bi</title>
<link rel="stylesheet" href="/static/css/app.css">
<script src="https://cdn.tailwindcss.com"></script>
</head><body class="bg-violet-50 min-h-screen">
<section class="max-w-2xl mx-auto px-4 py-8">
  <nav class="text-xs text-gray-500 mb-4"><a href="/khong-gian" class="hover:text-violet-700">🌍 Không Gian</a> / 🌀 Truyền Tống</nav>
  <div class="bg-gradient-to-br from-violet-600 to-purple-700 rounded-2xl p-6 text-white shadow-lg mb-6 text-center">
    <div class="text-6xl mb-2">🌀</div>
    <h1 class="text-xl sm:text-2xl font-bold">Nhà Truyền Tống</h1>
    <p class="text-sm opacity-90 mt-1">Dịch chuyển đến Không Gian người chơi khác · Nhóm · Bản đồ Du Hí</p>
  </div>
  <div class="bg-white rounded-2xl p-4 border border-violet-200 shadow-sm mb-4">
    <h2 class="text-sm font-bold text-gray-700 mb-3">🌀 Truyền tống</h2>
    <form onsubmit="teleport(event)" class="space-y-3">
      <select id="target-type" class="w-full px-3 py-2 border border-gray-200 rounded-lg text-sm">
        <option value="user_space">👤 Không gian thành viên (nhập ID user)</option>
        <option value="group_space">👥 Không gian nhóm (nhập slug nhóm)</option>
      </select>
      <input type="text" id="target-id" required placeholder="ID hoặc slug"
             class="w-full px-3 py-2 border border-gray-200 rounded-lg text-sm">
      <button type="submit" class="w-full px-3 py-2 rounded-lg bg-violet-600 hover:bg-violet-700 text-white text-sm font-semibold">🌀 Truyền Tống</button>
    </form>
    <div id="teleport-result" class="text-xs mt-2"></div>
  </div>
  <h2 class="font-bold text-violet-800 mb-3">📌 Địa điểm đã ghé</h2>
  <div class="grid grid-cols-1 sm:grid-cols-2 gap-2">{bookmarks}</div>
</section>
<script>
async function teleport(e) {{
  e.preventDefault();
  const type = document.getElementById('target-type').value;
  const id = document.getElementById('target-id').value;
  const res = document.getElementById('teleport-result');
  const r = await fetch('/api/nha-truyen-tong/di', {{
    method: 'POST',
    headers: {{'Content-Type': 'application/x-www-form-urlencoded'}},
    body: new URLSearchParams({{target_type: type, target_id: id}})
  }});
  const d = await r.json();
  if (d.ok) {{ res.textContent = '✅ ' + d.message; res.className = 'text-xs mt-2 text-emerald-700';
              setTimeout(()=> location.href = d.redirect, 500); }}
  else {{ res.textContent = '⚠️ ' + (d.error||'Lỗi'); res.className = 'text-xs mt-2 text-red-700'; }}
}}
</script>
</body></html>"##,
        bookmarks = bookmarks_html
    );

    Html(html).into_response()
}

#[derive(Deserialize)]
pub struct TeleportForm {
    pub target_type: String,
    pub target_id: String,
}

/// POST /api/nha-truyen-tong/di — Truyền tống.
pub async fn api_nha_truyen_tong_di(State(state): State<AppState>, jar: CookieJar, Form(form): Form<TeleportForm>) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({"ok": false, "error": "unauthorized"})).into_response();
    };

    let (redirect, label) = match form.target_type.as_str() {
        "user_space" => {
            let target_id = match Uuid::parse_str(&form.target_id) {
                Ok(u) => u,
                Err(_) => return Json(serde_json::json!({"ok": false, "error": "ID user không hợp lệ (cần UUID)"})).into_response(),
            };
            // Get display name
            let name: Option<String> = sqlx::query_scalar("SELECT display_name FROM users WHERE id = $1 AND is_active = true")
                .bind(target_id).fetch_optional(&state.pool).await.ok().flatten();
            let Some(name) = name else {
                return Json(serde_json::json!({"ok": false, "error": "Không tìm thấy thành viên"})).into_response();
            };
            (format!("/ca-nhan?id={}", target_id), format!("Không gian của {}", name))
        }
        "group_space" => {
            let slug = form.target_id.clone();
            let name: Option<String> = sqlx::query_scalar("SELECT name FROM groups WHERE slug = $1 AND is_active = true")
                .bind(&slug).fetch_optional(&state.pool).await.ok().flatten();
            let Some(name) = name else {
                return Json(serde_json::json!({"ok": false, "error": "Không tìm thấy nhóm"})).into_response();
            };
            (format!("/cong-dong/nhom/{}", slug), format!("Nhóm {}", name))
        }
        _ => return Json(serde_json::json!({"ok": false, "error": "Loại truyền tống không hợp lệ"})).into_response(),
    };

    // Log visit
    let _ = sqlx::query(
        "INSERT INTO teleport_visits (user_id, target_type) VALUES ($1, $2)"
    )
    .bind(user.id).bind(&form.target_type)
    .execute(&state.pool).await;

    // Bookmark if not exists
    let _ = sqlx::query(
        "INSERT INTO teleport_bookmarks (user_id, target_type, target_id, label)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (user_id, target_type, target_id) DO NOTHING"
    )
    .bind(user.id).bind(&form.target_type).bind(&form.target_id).bind(&label)
    .execute(&state.pool).await;

    Json(serde_json::json!({
        "ok": true,
        "message": format!("Đang truyền tống đến {}...", label),
        "redirect": redirect
    })).into_response()
}

// ════════════════════════════════════════════════════════════════════════
// STAGE 68 — Sự Kiện Phật Lịch
// ════════════════════════════════════════════════════════════════════════

#[derive(FromRow, Serialize)]
struct EventRow {
    id: i64,
    code: String,
    name: String,
    emoji: String,
    description: Option<String>,
    event_date: chrono::naive::NaiveDate,
    bonus_a: i64,
    bonus_k: i64,
    is_active: bool,
    claimed: bool,
}

/// GET /su-kien — Trang Sự Kiện Phật Lịch.
pub async fn su_kien_page(State(state): State<AppState>, jar: CookieJar) -> Response {
    let user = get_user_from_session(&state.pool, &jar).await;

    let events: Vec<EventRow> = if let Some(ref u) = user {
        sqlx::query_as(
            "SELECT e.id, e.code, e.name, e.emoji, e.description, e.event_date,
                    e.bonus_a::BIGINT, e.bonus_k::BIGINT, e.is_active,
                    EXISTS(SELECT 1 FROM event_reward_claims c WHERE c.event_id = e.id AND c.user_id = $1 AND c.event_date = CURRENT_DATE) AS claimed
             FROM buddhist_events e
             WHERE e.is_active = true
             ORDER BY e.event_date ASC"
        )
        .bind(u.id)
        .fetch_all(&state.pool)
        .await
        .unwrap_or_default()
    } else {
        sqlx::query_as(
            "SELECT e.id, e.code, e.name, e.emoji, e.description, e.event_date,
                    e.bonus_a::BIGINT, e.bonus_k::BIGINT, e.is_active, false AS claimed
             FROM buddhist_events e
             WHERE e.is_active = true
             ORDER BY e.event_date ASC"
        )
        .fetch_all(&state.pool)
        .await
        .unwrap_or_default()
    };

    let events_html = events.iter().map(|e| {
        let bonus_text = format!("{}{}", 
            if e.bonus_a > 0 { format!("+{} A ", e.bonus_a) } else { String::new() },
            if e.bonus_k > 0 { format!("+{} K ", e.bonus_k) } else { String::new() });
        let today = chrono::Local::now().date_naive();
        let is_today = e.event_date.month() == today.month() && e.event_date.day() == today.day();
        let badge = if is_today { "<span class='ml-2 px-2 py-0.5 bg-red-500 text-white text-[10px] rounded-full font-bold'>HÔM NAY!</span>" } else { "" };
        let claim_btn = if user.is_some() {
            if e.claimed {
                "<span class='text-xs text-emerald-700 font-semibold'>✓ Đã nhận</span>".to_string()
            } else if is_today || e.bonus_a == 0 && e.bonus_k == 0 {
                format!("<button onclick=\"claim({})\" class='px-3 py-1.5 rounded-lg bg-amber-600 hover:bg-amber-700 text-white text-xs font-semibold'>Nhận thưởng</button>", e.id)
            } else {
                "<span class='text-xs text-gray-400'>Chưa đến ngày</span>".to_string()
            }
        } else {
            "<span class='text-xs text-gray-400'>Đăng nhập để nhận</span>".to_string()
        };
        let highlight = if is_today { "border-2 border-red-400 bg-red-50" } else { "" };
        format!(r#"<div class="bg-white rounded-xl p-3 border border-amber-200 {highlight}">
          <div class="flex items-start justify-between mb-1">
            <div class="font-semibold text-amber-900 text-sm">{emoji} {name}{badge}</div>
            <span class="text-xs text-amber-700 font-bold">{bonus}</span>
          </div>
          <div class="text-xs text-gray-500">{date}{desc}</div>
          <div class="mt-2">{btn}</div>
        </div>"#,
            highlight = highlight,
            emoji = e.emoji, name = html_escape(&e.name), badge = badge, bonus = bonus_text,
            date = e.event_date.format("%d/%m").to_string(),
            desc = e.description.as_ref().map(|d| format!(" — {}", html_escape(d))).unwrap_or_default(),
            btn = claim_btn
        )
    }).collect::<Vec<_>>().join("");

    let html = format!(r##"<!DOCTYPE html>
<html lang="vi"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0, viewport-fit=cover">
<title>🪷 Sự Kiện Phật Lịch — Ứng Dụng Từ Bi</title>
<link rel="stylesheet" href="/static/css/app.css">
<script src="https://cdn.tailwindcss.com"></script>
</head><body class="bg-amber-50 min-h-screen">
<section class="max-w-3xl mx-auto px-4 py-8">
  <nav class="text-xs text-gray-500 mb-4"><a href="/" class="hover:text-amber-700">🏠 Trang chủ</a> / 🪷 Sự Kiện</nav>
  <div class="bg-gradient-to-br from-amber-600 to-red-700 rounded-2xl p-5 sm:p-6 text-white shadow-lg mb-6 text-center">
    <div class="text-5xl mb-2">🪷</div>
    <h1 class="text-xl sm:text-2xl font-bold">Sự Kiện Phật Lịch</h1>
    <p class="text-sm opacity-90 mt-1">Lễ Phật Đản · Vu Lan · Thanh Đinh · và các ngày lễ quan trọng. Nhận thưởng đặc biệt vào ngày lễ.</p>
  </div>
  <div class="space-y-3">{events_html}</div>
</section>
<script>
async function claim(id) {{
  const r = await fetch('/api/su-kien/' + id + '/nhan', {{ method: 'POST' }});
  const d = await r.json();
  alert(d.ok ? '🎉 ' + d.message : '⚠️ ' + (d.error||'Lỗi'));
  if (d.ok) location.reload();
}}
</script>
</body></html>"##,
        events_html = events_html
    );

    Html(html).into_response()
}

/// POST /api/su-kien/{event_id}/nhan — Nhận thưởng sự kiện (atomic).
pub async fn api_su_kien_nhan(State(state): State<AppState>, jar: CookieJar, Path(event_id): Path<i64>) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({"ok": false, "error": "unauthorized"})).into_response();
    };

    let mut tx = match state.pool.begin().await {
        Ok(tx) => tx,
        Err(e) => return Json(serde_json::json!({"ok": false, "error": format!("DB: {e}")})).into_response(),
    };

    let event: Option<(i64, i64, bool)> = sqlx::query_as(
        "SELECT bonus_a::BIGINT, bonus_k::BIGINT, is_active FROM buddhist_events WHERE id = $1"
    )
    .bind(event_id).fetch_optional(&mut *tx).await.ok().flatten();

    let Some((bonus_a, bonus_k, is_active)) = event else {
        let _ = tx.rollback().await;
        return Json(serde_json::json!({"ok": false, "error": "Sự kiện không tồn tại"})).into_response();
    };
    if !is_active {
        let _ = tx.rollback().await;
        return Json(serde_json::json!({"ok": false, "error": "Sự kiện đã tắt"})).into_response();
    }

    // Check if already claimed today
    let already: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM event_reward_claims WHERE user_id = $1 AND event_id = $2 AND event_date = CURRENT_DATE"
    )
    .bind(user.id).bind(event_id).fetch_one(&mut *tx).await.unwrap_or(0);
    if already > 0 {
        let _ = tx.rollback().await;
        return Json(serde_json::json!({"ok": false, "error": "Bạn đã nhận thưởng sự kiện này hôm nay"})).into_response();
    }

    // Apply rewards
    if bonus_a > 0 {
        let _ = sqlx::query("UPDATE users SET a_balance = a_balance + $2, updated_at = NOW() WHERE id = $1")
            .bind(user.id).bind(bonus_a).execute(&mut *tx).await;
        let _ = sqlx::query("INSERT INTO balance_transactions (user_id, currency, amount, direction, reason) VALUES ($1, 'a', $2, 'in', 'su_kien_phat_lich')")
            .bind(user.id).bind(bonus_a).execute(&mut *tx).await;
    }
    if bonus_k > 0 {
        let _ = sqlx::query("UPDATE users SET k_balance = k_balance + $2, updated_at = NOW() WHERE id = $1")
            .bind(user.id).bind(bonus_k).execute(&mut *tx).await;
        let _ = sqlx::query("INSERT INTO balance_transactions (user_id, currency, amount, direction, reason) VALUES ($1, 'k', $2, 'in', 'su_kien_phat_lich')")
            .bind(user.id).bind(bonus_k).execute(&mut *tx).await;
    }

    // Log claim — v0.9.47 fix: dùng ON CONFLICT DO NOTHING + check rows_affected
    // để tránh race condition (2 request concurrent cùng pass check `already`, cùng apply reward → double count).
    let claim_result = sqlx::query(
        "INSERT INTO event_reward_claims (user_id, event_id, event_date, reward_a, reward_k)
         VALUES ($1, $2, CURRENT_DATE, $3, $4)
         ON CONFLICT (user_id, event_id, event_date) DO NOTHING"
    )
    .bind(user.id).bind(event_id).bind(bonus_a).bind(bonus_k)
    .execute(&mut *tx).await;

    let claim_rows = claim_result.map(|r| r.rows_affected()).unwrap_or(0);
    if claim_rows == 0 {
        // Race condition: đã có claim từ request khác → rollback toàn bộ reward updates.
        let _ = tx.rollback().await;
        return Json(serde_json::json!({
            "ok": false,
            "error": "Bạn đã nhận thưởng sự kiện này hôm nay (race condition detected, rollback)"
        })).into_response();
    }

    if let Err(e) = tx.commit().await {
        return Json(serde_json::json!({"ok": false, "error": format!("commit: {e}")})).into_response();
    }

    let mut msg = String::new();
    if bonus_a > 0 { msg.push_str(&format!("+{} A ", bonus_a)); }
    if bonus_k > 0 { msg.push_str(&format!("+{} K ", bonus_k)); }
    if msg.is_empty() { msg = "Sự kiện không có thưởng hôm nay".to_string(); }

    Json(serde_json::json!({"ok": true, "message": format!("Đã nhận: {}", msg)})).into_response()
}

// ════════════════════════════════════════════════════════════════════════
// STAGE 69 — Huy Hiệu Thành Tích
// ════════════════════════════════════════════════════════════════════════

#[derive(FromRow, Serialize)]
struct BadgeInfo {
    id: i64,
    code: String,
    name: String,
    emoji: String,
    description: Option<String>,
    category: String,
    requirement_type: String,
    requirement_value: i64,
    awarded_at: Option<DateTime<Utc>>,
}

/// GET /api/huy-hieu/{user_id} — JSON huy hiệu của user.
pub async fn api_huy_hieu_user(State(state): State<AppState>, Path(user_id): Path<Uuid>) -> Response {
    let badges: Vec<BadgeInfo> = sqlx::query_as(
        "SELECT b.id, b.code, b.name, b.emoji, b.description, b.category,
                b.requirement_type, b.requirement_value::BIGINT,
                ub.awarded_at
         FROM achievement_badges b
         LEFT JOIN user_badges ub ON ub.badge_id = b.id AND ub.user_id = $1
         WHERE b.is_active = true
         ORDER BY b.category, b.requirement_value"
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    Json(badges).into_response()
}

/// POST /api/huy-hieu/check — Auto-check + award huy hiệu cho current user.
pub async fn api_huy_hieu_check(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(user) = get_user_from_session(&state.pool, &jar).await else {
        return Json(serde_json::json!({"ok": false, "error": "unauthorized"})).into_response();
    };

    let badges: Vec<(i64, String, String, i64)> = sqlx::query_as(
        "SELECT id, requirement_type, code, requirement_value::BIGINT
         FROM achievement_badges WHERE is_active = true"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let mut awarded: Vec<String> = vec![];
    for (badge_id, req_type, code, req_val) in badges {
        let met = match req_type.as_str() {
            "niem_count" => {
                let v: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(niem_count), 0)::BIGINT FROM practice_logs WHERE user_id = $1")
                    .bind(user.id).fetch_one(&state.pool).await.unwrap_or(0);
                v >= req_val
            }
            "a_balance" => {
                let v: i64 = sqlx::query_scalar("SELECT a_balance::BIGINT FROM users WHERE id = $1")
                    .bind(user.id).fetch_one(&state.pool).await.unwrap_or(0);
                v >= req_val
            }
            "k_balance" => {
                let v: i64 = sqlx::query_scalar("SELECT k_balance::BIGINT FROM users WHERE id = $1")
                    .bind(user.id).fetch_one(&state.pool).await.unwrap_or(0);
                v >= req_val
            }
            "friend_count" => {
                let v: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*)::BIGINT FROM friendships
                     WHERE (user_id_1 = $1 OR user_id_2 = $1) AND status = 'accepted'"
                )
                .bind(user.id).fetch_one(&state.pool).await.unwrap_or(0);
                v >= req_val
            }
            "group_count" => {
                let v: i64 = sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM group_members WHERE user_id = $1")
                    .bind(user.id).fetch_one(&state.pool).await.unwrap_or(0);
                v >= req_val
            }
            "topic_count" => {
                let v: i64 = sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM topics WHERE user_id = $1")
                    .bind(user.id).fetch_one(&state.pool).await.unwrap_or(0);
                v >= req_val
            }
            "days_active" => {
                // distinct log_date count
                let v: i64 = sqlx::query_scalar("SELECT COUNT(DISTINCT log_date)::BIGINT FROM practice_logs WHERE user_id = $1")
                    .bind(user.id).fetch_one(&state.pool).await.unwrap_or(0);
                v >= req_val
            }
            "tu_si_rank" => {
                let v: i16 = sqlx::query_scalar("SELECT COALESCE(tu_si_rank, 0) FROM users WHERE id = $1")
                    .bind(user.id).fetch_one(&state.pool).await.unwrap_or(0);
                v as i64 >= req_val
            }
            _ => false,
        };

        if met {
            let inserted = sqlx::query(
                "INSERT INTO user_badges (user_id, badge_id) VALUES ($1, $2)
                 ON CONFLICT (user_id, badge_id) DO NOTHING"
            )
            .bind(user.id).bind(badge_id)
            .execute(&state.pool).await
            .map(|r| r.rows_affected() > 0)
            .unwrap_or(false);
            if inserted {
                awarded.push(code);
            }
        }
    }

    Json(serde_json::json!({
        "ok": true,
        "awarded": awarded,
        "message": if awarded.is_empty() { "Chưa có huy hiệu mới".to_string() } else { format!("Đã nhận {} huy hiệu mới!", awarded.len()) }
    })).into_response()
}

// ════════════════════════════════════════════════════════════════════════
// STAGE 70 — Bảng Vinh Danh
// ════════════════════════════════════════════════════════════════════════

#[derive(FromRow, Serialize)]
struct HofRow {
    rank_position: i16,
    user_id: Uuid,
    display_name: String,
    avatar_url: Option<String>,
    score: i64,
    tu_si_rank: Option<i16>,
}

/// GET /bang-vinh-danh — Trang Bảng Vinh Danh.
pub async fn bang_vinh_danh_page(State(state): State<AppState>, jar: CookieJar) -> Response {
    let _user = get_user_from_session(&state.pool, &jar).await;

    let categories: &[(&str, &str, &str)] = &[
        ("niem_total",  "🏆 BXH Niệm Phật Tổng",  "Tổng số lần niệm Phật từ khi tham gia"),
        ("a",           "⚡ BXH Niệm Lực A",      "Số A (Niệm Lực) hiện có"),
        ("k",           "💰 BXH Tài Phú K",      "Số K (Tiền app) hiện có"),
        ("bi",          "🪷 BXH Từ Bi Bi",        "Số Bi (Tiền Từ Bi) hiện có"),
        ("friend",      "👥 BXH Kết Duyên",       "Số bạn bè đã kết"),
        ("topic",       "📝 BXH Tác Giả",         "Số chủ đề đã tạo"),
        ("tu_si",       "⭐ BXH Tu Sĩ",            "Cấp Tu Sĩ cao nhất"),
    ];

    let mut sections_vec: Vec<String> = Vec::with_capacity(categories.len());
    for (cat, title, desc) in categories {
        let rows: Vec<HofRow> = match *cat {
            "niem_total" => sqlx::query_as(
                "SELECT 1 AS rank_position, u.id AS user_id, COALESCE(u.display_name, 'Ẩn danh') AS display_name,
                        u.avatar_url, COALESCE(SUM(p.niem_count), 0)::BIGINT AS score, u.tu_si_rank
                 FROM users u LEFT JOIN practice_logs p ON p.user_id = u.id
                 WHERE u.is_active = true
                 GROUP BY u.id, u.display_name, u.avatar_url, u.tu_si_rank
                 ORDER BY score DESC LIMIT 10"
            ).fetch_all(&state.pool).await.unwrap_or_default(),
            "a" => sqlx::query_as(
                "SELECT ROW_NUMBER() OVER (ORDER BY a_balance DESC)::SMALLINT AS rank_position,
                        id AS user_id, COALESCE(display_name, 'Ẩn danh') AS display_name,
                        avatar_url, a_balance::BIGINT AS score, tu_si_rank
                 FROM users WHERE is_active = true ORDER BY a_balance DESC LIMIT 10"
            ).fetch_all(&state.pool).await.unwrap_or_default(),
            "k" => sqlx::query_as(
                "SELECT ROW_NUMBER() OVER (ORDER BY k_balance DESC)::SMALLINT AS rank_position,
                        id AS user_id, COALESCE(display_name, 'Ẩn danh') AS display_name,
                        avatar_url, k_balance::BIGINT AS score, tu_si_rank
                 FROM users WHERE is_active = true ORDER BY k_balance DESC LIMIT 10"
            ).fetch_all(&state.pool).await.unwrap_or_default(),
            "bi" => sqlx::query_as(
                "SELECT ROW_NUMBER() OVER (ORDER BY COALESCE(bi_balance, 0) DESC)::SMALLINT AS rank_position,
                        id AS user_id, COALESCE(display_name, 'Ẩn danh') AS display_name,
                        avatar_url, COALESCE(bi_balance, 0)::BIGINT AS score, tu_si_rank
                 FROM users WHERE is_active = true ORDER BY COALESCE(bi_balance, 0) DESC LIMIT 10"
            ).fetch_all(&state.pool).await.unwrap_or_default(),
            "friend" => sqlx::query_as(
                "SELECT ROW_NUMBER() OVER (ORDER BY cnt DESC)::SMALLINT AS rank_position,
                        u.id AS user_id, COALESCE(u.display_name, 'Ẩn danh') AS display_name,
                        u.avatar_url, cnt::BIGINT AS score, u.tu_si_rank
                 FROM users u
                 LEFT JOIN LATERAL (
                   SELECT COUNT(*)::BIGINT AS cnt FROM friendships
                   WHERE (user_id_1 = u.id OR user_id_2 = u.id) AND status = 'accepted'
                 ) f ON true
                 WHERE u.is_active = true
                 ORDER BY cnt DESC NULLS LAST LIMIT 10"
            ).fetch_all(&state.pool).await.unwrap_or_default(),
            "topic" => sqlx::query_as(
                "SELECT ROW_NUMBER() OVER (ORDER BY cnt DESC)::SMALLINT AS rank_position,
                        u.id AS user_id, COALESCE(u.display_name, 'Ẩn danh') AS display_name,
                        u.avatar_url, cnt::BIGINT AS score, u.tu_si_rank
                 FROM users u
                 LEFT JOIN LATERAL (
                   SELECT COUNT(*)::BIGINT AS cnt FROM topics t WHERE t.user_id = u.id
                 ) t ON true
                 WHERE u.is_active = true
                 ORDER BY cnt DESC NULLS LAST LIMIT 10"
            ).fetch_all(&state.pool).await.unwrap_or_default(),
            "tu_si" => sqlx::query_as(
                "SELECT ROW_NUMBER() OVER (ORDER BY tu_si_rank DESC NULLS LAST)::SMALLINT AS rank_position,
                        id AS user_id, COALESCE(display_name, 'Ẩn danh') AS display_name,
                        avatar_url, COALESCE(tu_si_rank, 0)::BIGINT AS score, tu_si_rank
                 FROM users WHERE is_active = true AND tu_si_rank IS NOT NULL
                 ORDER BY tu_si_rank DESC LIMIT 10"
            ).fetch_all(&state.pool).await.unwrap_or_default(),
            _ => Vec::new(),
        };

        let rows_html = rows.iter().map(|r| {
            let medal = match r.rank_position {
                1 => "🥇",
                2 => "🥈",
                3 => "🥉",
                _ => "",
            };
            let stars = r.tu_si_rank.map(|v| "⭐".repeat(v as usize)).unwrap_or_default();
            let avatar = r.avatar_url.as_ref().map(|u| format!("<img src='{}' class='w-8 h-8 rounded-full' alt='avatar'>", u)).unwrap_or_else(|| format!("<div class='w-8 h-8 rounded-full bg-amber-200 flex items-center justify-center text-xs font-bold text-amber-800'>{}</div>", r.display_name.chars().next().unwrap_or('?').to_uppercase()));
            let bg = if r.rank_position <= 3 { "bg-amber-50 border border-amber-200" } else { "bg-gray-50" };
            format!(r#"<div class="flex items-center gap-3 p-2 rounded-lg {bg}">
              <span class="text-lg font-bold w-8 text-center">{medal}{rank}</span>
              {avatar}
              <div class="flex-1 min-w-0"><div class="font-semibold text-sm text-gray-800 truncate">{name}</div>
              <div class="text-xs text-amber-700">{stars}</div></div>
              <div class="text-sm font-bold text-amber-700">{score}</div>
            </div>"#,
                bg = bg, medal = medal, rank = r.rank_position,
                avatar = avatar, name = html_escape(&r.display_name), stars = stars,
                score = r.score
            )
        }).collect::<Vec<_>>().join("");

        let section_html = format!(r#"<div class="bg-white rounded-2xl p-4 border border-amber-200 shadow-sm">
          <h2 class="font-bold text-amber-900 mb-2">{title}</h2>
          <p class="text-xs text-gray-500 mb-3">{desc}</p>
          <div class="space-y-1">{rows_html}</div>
        </div>"#, title = title, desc = desc, rows_html = rows_html);
        sections_vec.push(section_html);
    }
    let sections = sections_vec.join("");

    let html = format!(r##"<!DOCTYPE html>
<html lang="vi"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0, viewport-fit=cover">
<title>🏆 Bảng Vinh Danh — Ứng Dụng Từ Bi</title>
<link rel="stylesheet" href="/static/css/app.css">
<script src="https://cdn.tailwindcss.com"></script>
</head><body class="bg-amber-50 min-h-screen">
<section class="max-w-3xl mx-auto px-4 py-8">
  <nav class="text-xs text-gray-500 mb-4"><a href="/" class="hover:text-amber-700">🏠 Trang chủ</a> / 🏆 Vinh Danh</nav>
  <div class="bg-gradient-to-br from-yellow-500 to-amber-700 rounded-2xl p-6 text-white shadow-lg mb-6 text-center">
    <div class="text-5xl mb-2">🏆</div>
    <h1 class="text-xl sm:text-2xl font-bold">Bảng Vinh Danh</h1>
    <p class="text-sm opacity-90 mt-1">Top thành viên theo nhiều tiêu chí: Niệm Phật · A · K · Bi · Bạn bè · Chủ đề · Tu Sĩ</p>
  </div>
  <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">{sections}</div>
</section>
</body></html>"##,
        sections = sections
    );

    Html(html).into_response()
}

// ════════════════════════════════════════════════════════════════════════
// HELPERS
// ════════════════════════════════════════════════════════════════════════

fn format_ago(ts: DateTime<Utc>) -> String {
    let now = Utc::now();
    let diff = now.signed_duration_since(ts);
    if diff.num_seconds() < 60 {
        return format!("{} giây trước", diff.num_seconds().max(0));
    }
    if diff.num_minutes() < 60 {
        return format!("{} phút trước", diff.num_minutes());
    }
    if diff.num_hours() < 24 {
        return format!("{} giờ trước", diff.num_hours());
    }
    if diff.num_days() < 7 {
        return format!("{} ngày trước", diff.num_days());
    }
    ts.format("%d/%m/%Y").to_string()
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

// Suppress unused warning
#[allow(dead_code)]
fn _unused(_: Duration) {}
