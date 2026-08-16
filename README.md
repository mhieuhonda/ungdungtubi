# 🪷 Ứng Dụng Từ Bi

> *Siêu thoát không siêu thích. Giải thoát không giải thích. Buông bỏ mới có thể trở về.*

**Domain:** [tubi.louis.vangioitutien.com](https://tubi.louis.vangioitutien.com)

## 📦 Phiên bản hiện tại: v0.9.33 — Giai đoạn 38

**Giai đoạn 38: Nhà Nhạc (Music House — KG-03) + Logo Emoji Sharpened 🪷**

Triển khai **Nhà Nhạc** — phòng KG-03 trong Không Gian (theo tài liệu "Hệ Thống Và Chức Năng Chi Tiết"). 5 thư mục nhạc (Niem · Thien · Dao · KhongLoi · CaNhan), 4 chế độ phát (SingleRepeat · Shuffle · RepeatAll · Loop), hẹn giờ tắt, playlist Cá Nhân. Làm nét logo emoji 🪷 (giữ nguyên emoji, tối ưu render `geometricPrecision` + emoji font fallback + 256 viewBox).

### 🎵 Nhà Nhạc — `/khong-gian/nha-nhac`

- **[MUSIC-1]** `migrations/023_nha_nhac.sql` — Tạo 3 bảng: `music_tracks` (kho nhạc hệ thống), `user_music_prefs` (preferences per-user), `user_personal_tracks` (playlist Cá Nhân). Seed 12 track mẫu cho 4 category.
- **[MUSIC-2]** `src/models/nha_nhac.rs` — Models: `MusicCategory` (5 enum), `PlaybackMode` (4 enum), `MusicTrack`, `UserMusicPrefs`, `MusicPrefsForm`, `PersonalPlaylistItem`, `NhaNhacStats`.
- **[MUSIC-3]** `src/handlers/nha_nhac.rs` — 9 handlers: index, category filter, tracks API (all + by category), preferences GET/POST, add/remove Cá Nhân, play count, stats.
- **[MUSIC-4]** `templates/khong-gian/nha-nhac.html` — Player UI: HTML5 audio + Alpine.js component `musicPlayer()` với play/pause/next/prev, playback mode selector, volume slider, sleep timer (15/30/60 phút), category tabs, track list với ⭐ thêm vào Cá Nhân.
- **[MUSIC-5]** `src/main.rs` — 10 routes mới:
  - `GET /khong-gian/nha-nhac` — Trang Nhà Nhạc (default category: niem)
  - `GET /khong-gian/nha-nhac/{category}` — Lọc theo category
  - `GET /api/nha-nhac/tracks` — JSON tất cả track
  - `GET /api/nha-nhac/tracks/{category}` — JSON track theo category
  - `GET|POST /api/nha-nhac/preferences` — Read/update preferences
  - `POST /api/nha-nhac/ca-nhan/them` — Add track → Cá Nhân
  - `POST /api/nha-nhac/ca-nhan/xoa/{track_id}` — Remove track khỏi Cá Nhân
  - `POST /api/nha-nhac/track/{track_id}/play` — Increment play count
  - `GET /api/nha-nhac/stats` — Stats JSON
- **[MUSIC-6]** `templates/khong-gian/index.html` — Thêm card "Nhà Nhạc" (gradient indigo→violet) với CTA "🎵 Mở Nhà Nhạc →" ngay dưới hero Không Gian.
- **[MUSIC-7]** `src/static/css/app.css` — CSS cho player: `.nha-nhac-player`, `.nha-nhac-btn`, `.nha-nhac-track`, `.nha-nhac-playing-indicator`, `.nha-nhac-equalizer-bar` (animated equalizer bars).

### 🪷 Logo Emoji Sharpened (giữ nguyên emoji 🪷)

- **[LOGO-1]** `src/static/favicon.svg` — Bump viewBox 100→256, font-size 90→240, thêm `shape-rendering="geometricPrecision"`, `text-rendering="geometricPrecision"`, `text-anchor="middle"`, `dominant-baseline="central"`, font-family fallback.
- **[LOGO-2]** `src/static/logo.svg` — Tương tự favicon (256×256, geometricPrecision).
- **[LOGO-3]** `src/static/logo-inline.svg` — Tương tự (256×256, geometricPrecision).
- **[LOGO-4]** `templates/layout.html` — Favicon data URI: bump 100→256 viewBox, thêm font-family fallback (Apple Color Emoji → Segoe UI Emoji → Noto Color Emoji → Twemoji Mozilla → system-ui), thêm `shape-rendering` + `text-rendering="geometricPrecision"`. URL-encode `%3C` `%3E` cho data URI SVG hợp lệ.
- **[LOGO-5]** `src/static/css/app.css` — Thêm class `.lotus-emoji`, `.lotus-logo-header`, override `.niem-btn` + `.buddha-statue` với `font-family` emoji fallback, `text-rendering: geometricPrecision`, `-webkit-font-smoothing: antialiased`, `-moz-osx-font-smoothing: grayscale`, `font-feature-settings: "liga" 1`, `font-variant-emoji: emoji`, `transform: translateZ(0)`.
- **[LOGO-6]** Layout.html + home.html — Thêm class `lotus-logo-header` (header + bottom nav center button) + `lotus-emoji` (hero home + chat bubble).

### 📦 Version Sync v0.9.33

- Bump version `0.9.32` → `0.9.33` ở: `Cargo.toml`, `src/main.rs` (startup log + health check public + health check inner + phase 37 → 38), `templates/layout.html` (footer), `templates/khong-gian/index.html` (footer), `Dockerfile.coolify` (comment), `templates/admin/phat-trien/index.html` (phase badge + roadmap), và footer version ở 6 admin templates.
- Update phase 37 → 38 trong health check + main log.
- Thêm 9 feature flags v0.9.33 vào `HEALTH_FEATURES` array.
- Cập nhật `khong_gian.features` trong health check: thêm `nha-nhac-music-house`.
- Thêm `khong_gian.nha_nhac` object trong health check (status, route, categories, playback_modes, sleep_timer, personal_playlist).
- Cập nhật roadmap `/admin/phat-trien`: Giai đoạn 37 → "Hoàn thành" (green), Giai đoạn 38 → "Đang triển khai" (indigo).

---

## 📦 Phiên bản trước: v0.9.32 — Giai đoạn 37

### 🧭 Admin Phát Triển Dashboard — /admin/phat-trien

- **[DASH-1]** `templates/admin/phat-trien/index.html` — Dashboard riêng (indigo, vision, roadmap, CI/CD)
- **[DASH-2]** `src/handlers/admin.rs` — Handler `admin_phat_trien_dashboard` + template struct
- **[DASH-3]** `src/main.rs` — Route `GET /admin/phat-trien`
- **[DASH-4]** `src/models/user.rs` — `admin_dashboard_path()` → `/admin/phat-trien` (không còn dùng `/admin/ky-thuat` tạm)

### 🪷 Logo Emoji Hoa Sen

- **[LOGO-1]** Favicon — Đổi từ SVG sang inline emoji SVG data URI 🪷
- **[LOGO-2]** Header logo — Đổi từ `<img>` SVG sang `<span>🪷</span>`
- **[LOGO-3]** Home hero — Đổi từ `<img>` SVG sang `<span>🪷</span>`
- **[LOGO-4]** SVG files — `favicon.svg`, `logo.svg`, `logo-inline.svg` → emoji-based SVG

### 📦 Version Sync v0.9.32

- Fix version drift: v0.9.19/v0.9.29/v0.9.30 → v0.9.32 (31 replacements across 9 files)
- Health check: version 0.9.32, phase 37
- 6 new features in HEALTH_FEATURES

---

## 📦 Phiên bản trước: v0.9.31 — Giai đoạn 36

Chat UX overhauled — xóa role badge trong chat theo yêu cầu user, redesign logo thành hoa sen cách điệu, fix admin_phat_trien 403 Forbidden, thêm REST fallback cho global chat, tối ưu WebSocket reconnect timing.

### 🪷 Logo Redesign — Hoa Sen trên bàn phím

- **[LOGO-1]** `logo.svg` — Redesign hoàn toàn thành hoa sen cách điệu (5 cánh ngoài + 3 cánh trong + nhụy vàng), gradient xanh lá + vàng
- **[LOGO-2]** `logo-inline.svg` — Inline variant (48x48) cùng phong cách
- **[LOGO-3]** `favicon.svg` — Favicon (64x64) cùng phong cách hoa sen

### 🗑️ Xóa Role Badge trong Chat

- **[BADGE-1]** `chat.js` — `roleBadgeHtml()` trả về rỗng (xóa ⚙️ SYS, 👑 ADMIN, 🛡️ ADMIN, 🧭 DEV, 📜 MOD)
- **[BADGE-2]** `app.css` — `.chat-role-badge` ẩn hoàn toàn (`display: none !important`)
- Theo yêu cầu user: "Xóa biểu tượng bên cạnh tên như SYS hay ADMIN khi admin/mod nhắn tin vì như thế sẽ bị khó chịu"

### 🐛 Bug Fix Sweep

- **[FIX-1]** `admin.rs` — Fix admin_phat_trien 403 Forbidden trên /admin/ky-thuat dashboard. Thay `is_admin_ky_thuat()` bằng `is_admin_ky_thuat() || is_admin_phat_trien()` cho 3 handler: dashboard, users redirect, audit log.
- **[FIX-2]** `app.css` — Thêm CSS cho `.chat-msg-admin-phat-trien`, `.chat-msg-admin-phat-trien-name`, `.chat-avatar-admin-phat-trien`, `.chat-role-badge-admin-phat-trien` (trước đây thiếu hoàn toàn).
- **[FIX-3]** `conversation.html` — DM template dùng `authorLabel()`, `msgBubbleClass()`, `msgNameClass()`, `roleBadgeHtml()` — đồng nhất với global chat.
- **[FIX-4]** `chat.rs` + `main.rs` — Thêm REST fallback endpoint `POST /api/chat-chung/gui` cho global chat. Fix mất tin nhắn khi WS không kết nối được.
- **[FIX-5]** `chat.js` — REST fallback `_sendViaRest()` cho globalChat (giống dmChat v0.9.30).

### ⚡ Tối ưu WebSocket Chat

- **[PERF-1]** `chat.js` — Đồng bộ client ping interval từ 30s → 25s (match server).
- **[PERF-2]** `chat.js` — Giảm reconnect delay: max 3s (từ 8s), attempt 1 = 200ms (từ 500ms).
- **[PERF-3]** `chat.js` — Giảm health check timeout từ 60s → 40s.

---

## 📦 Phiên bản trước: v0.9.30 — Giai đoạn 35

**Giai đoạn 35: Admin Phát Triển Role + DM REST Fallback + Bug Fix Sweep**

Xem chi tiết trong CHANGELOG.md.

---

## 📦 Phiên bản trước: v0.9.29 — Giai đoạn 34

**Giai đoạn 34: Admin Equal Rebalance + Live Chat Optimize + DM Fix + Performance**

Xem chi tiết trong CHANGELOG.md.

---

## 📦 Phiên bản trước: v0.9.27 — Giai đoạn 32

**Giai đoạn 32: Critical UI Fix (FOUC + Chat + Menu) + Chat History Robustness + Security**

### 🚨 Fix CRITICAL — FOUC (Flash of Unstyled Content) trên mobile
- **[FOUC-1] Chat Chung popup flash visible trước khi Alpine.js init** — Khi vừa vào web, chat popup bị hiện rồi mới ẩn (FOUC), tạo cảm giác "chat tự mở che hết màn hình". Nguyên nhân: `x-cloak` CSS selector `[x-cloak]` có specificity thấp hơn `.chat-chung-popup` (display:flex) → trên một số trình duyệt mobile, chat popup flash visible trong khoảnh khắc trước khi Alpine xử lý `x-show`.
  - **Fix v0.9.27**:
    - Thêm `style="display:none"` trực tiếp vào chat popup, backdrop, chat bubble, và mobile menu drawer — fallback HTML-level không phụ thuộc Alpine
    - Thêm class-specific x-cloak selectors: `[x-cloak].chat-chung-popup`, `[x-cloak].mobile-menu-drawer` — specificity (0,2,0) thắng (0,1,0)
    - Alpine `x-show` sẽ override `display:none` khi khởi tạo xong
  - **File**: `templates/layout.html`

- **[FOUC-2] Mobile hamburger menu (3 gạch) flash visible trước khi Alpine init** — Cùng nguyên nhân FOUC-1, mobile menu drawer bị flash visible rồi mới ẩn.
  - **Fix v0.9.27**: Thêm `style="display:none"` + class `mobile-menu-drawer` + x-cloak class-specific selector
  - **File**: `templates/layout.html`

### 🐛 Fix HIGH — Chat popup tự mở + không đóng được
- **[CHAT-1] Chat popup có thể tự mở nếu Alpine component re-init** — Khi HTMX partial replace hoặc Alpine component bị re-initialize, `isOpen` có thể bị `undefined` → `!undefined === true` → popup tự mở.
  - **Fix v0.9.27**:
    - Thêm guard trong `toggleChat()`: nếu `typeof isOpen !== 'boolean'` → reset về `false`
    - Thêm `this.isOpen = false` đầu tiên trong `init()` — double-safe
    - Chat popup **KHÔNG BAO GIỜ** tự mở — chỉ mở khi user click bubble
  - **File**: `src/static/js/chat.js`

- **[CHAT-2] Nút đóng chat (×) quá nhỏ trên mobile** — w-8 h-8 (32px) dễ bị miss tap.
  - **Fix v0.9.27**: Tăng lên w-10 h-10 (40px), thêm border + active:bg-white/30 cho feedback
  - **File**: `templates/layout.html`

- **[CHAT-3] Chat popup vẫn che nhiều trên điện thoại nhỏ** — 50dvh + min 280px vẫn che >50% trên màn hình 568px.
  - **Fix v0.9.27**: Giảm từ 50dvh → 45dvh, min 280px → 240px — chỉ chiếm ~45% viewport
  - **File**: `src/static/css/app.css`, `src/static/css/chat.css`

### 🐛 Fix HIGH — Chat history bị mất (retry + robustness)
- **[CHAT-4] loadHistory() fail silently → user thấy "Chưa có tin nhắn" dù DB có data** — Nếu API `/api/chat-chung/history` fail (network glitch, DB timeout), `loadHistory()` catch error rồi im lặng → messages = [] → user tưởng "mất lịch sử".
  - **Fix v0.9.27**: Thêm retry (tối đa 2 lần) với exponential backoff, log error rõ ràng bằng `console.warn`, validate response là Array trước khi assign
  - **File**: `src/static/js/chat.js`

### 🔒 Fix MEDIUM — ILIKE wildcard injection trong search
- **[SEARCH-1] User search "%" → match tất cả rows** — `format!("%{q}%")` không escape `%` và `_` → unintended broad matches.
  - **Fix v0.9.27**: Escape `\` → `\\`, `%` → `\%`, `_` → `\_` trước khi wrap; thêm `ESCAPE '\\'` clause trong SQL
  - **File**: `src/handlers/tim_kiem.rs`, `src/handlers/kinh_sach.rs`, `src/handlers/friends.rs`

### 🎨 Fix LOW — Missing x-cloak trên các element Alpine.js
- **[FOUC-3] Missing x-cloak trên DM chat connection status + empty message** — 6 element trong `conversation.html` và `layout.html` thiếu `x-cloak` → flash visible
  - **Fix v0.9.27**: Thêm `x-cloak` + `style="display:none"` cho tất cả element `x-show` cần ẩn ban đầu
  - **File**: `templates/ban-be/conversation.html`, `templates/layout.html`

### 🐛 Fix UI — Hamburger Menu stuck open (HIGH)
- **[UI-2] Mobile menu (3 gạch) bị bật vĩnh viễn, không tự đóng** — Trước v0.9.26, `<div x-show="mobileMenu">` không có `@click.outside` directive. Khi user tap nút 3 gạch, `mobileMenu = true` → menu mở. Nhưng menu chỉ đóng khi:
  - Tap lại nút 3 gạch (toggle)
  - Tap vào 1 link trong menu (chuyển trang)
  - Resize sang desktop
  - Không có cách nào đóng menu khi tap outside hoặc nhấn ESC.
  - **Tác động**: User báo "cái ba gạch bị bật vĩnh viễn" — menu stuck open che nội dung.
  - **Fix v0.9.26**:
    - Thêm `@click.outside="mobileMenu = false"` → đóng menu khi tap ra ngoài
    - Thêm `@keydown.escape.window="mobileMenu = false"` → đóng menu khi nhấn ESC
    - Icon toggle: 3 gạch ⇄ X (đổi icon khi mở/đóng)
    - Tất cả link trong menu thêm `@click="mobileMenu = false"` → đóng menu khi click link (trước đây click link chỉ chuyển trang, không đóng menu ngay)
    - `aria-expanded` + `aria-label="Menu"` cho accessibility
  - **File**: `templates/layout.html`.

### 🐛 Fix UI — Chat bubble đè lên bottom nav (MEDIUM)
- **[UI-3] Chat bubble trên mobile đè 32px lên bottom nav** — Trước v0.9.26, chat bubble có `top: y = innerHeight - 88` trên mobile. Bubble height = 56px → bottom edge ở `innerHeight - 32`. Bottom nav top ở `innerHeight - 64`. → Bubble bottom (innerHeight - 32) nằm BELOW nav top (innerHeight - 64) → đè lên nav 32px.
  - **Tác động**: User tap "🙏 Niệm Phật" (rightmost nav item) → vô tình tap chat bubble → mở popup chat.
  - **Fix v0.9.26**:
    - Đổi bubble `y = innerHeight - 128` trên mobile → bubble bottom ở `innerHeight - 72` = ngay trên bottom nav top.
    - Ẩn chat bubble khi chat popup đang mở (`x-show="!isOpen"`) → tránh bubble che popup input area.
  - **File**: `src/static/js/chat.js`, `templates/layout.html`.

### 📦 Version Sync
- Bump version `0.9.25` → `0.9.26` ở: `Cargo.toml`, `src/main.rs` (log + health check response + phase 30 → 31), `templates/layout.html` (footer), `src/handlers/mod.rs` (placeholder footer), `Dockerfile.coolify` (comment).
- Update phase 30 → 31 trong health check.
- Update `HEALTH_FEATURES` (+12 features v0.9.26).

### 📋 Ghi chú vận hành
- **Database**: Vẫn dùng `tubi-postgres` (PostgreSQL 17-alpine) trên Coolify, có persistent volume. Database KHÔNG bị reset khi deploy — migration 021 (TRUNCATE role_permissions) chỉ xoá bảng permissions (re-seed), không chạm vào user data, chat messages, topics, comments.
- **Mất dữ liệu lịch sử**: Nếu user thấy "mất hết dữ liệu kể từ v0.9.25", nguyên nhân là DB container bị recreate ngày 2026-08-13 (không phải do code v0.9.25). Database hiện tại có persistent volume, sẽ không bị mất khi deploy lại.
- **Env vars duplicate**: Coolify app hiện có 36 env vars (18 keys × 2 entries). Đây là artifact từ lần migrate hạ tầng trước. Không ảnh hưởng functionality (Coolify dùng giá trị mới nhất), nhưng nên clean up để tránh nhầm lẫn.

---

## 📦 Phiên bản trước: v0.9.25 — Giai đoạn 30

**Giai đoạn 30: Stability Fix + Critical Bug Fixes (Login + Migration + Schema)**

### 🚨 Fix Critical Login Bug (CRITICAL — Production-down fix)
- **[B1] Mọi login mới fail sau v0.9.24** — Migration 021 set `csrf_token NOT NULL` trên bảng `sessions`, nhưng `auth.rs::google_callback` INSERT session mới không set `csrf_token` → fail với `null value in column "csrf_token" violates not-null constraint`.
  - **Fix**: Sinh `csrf_token` random (64 hex chars = 32 bytes) + INSERT cùng session.
  - **Tác động**: 100% new login bị fail trước v0.9.25 — đây là nguyên nhân chính khiến user báo "hỏng hết rồi".

### 🚨 Fix Migration Failure (CRITICAL)
- **[B2] Migration 021 fail vì thiếu pgcrypto** — `gen_random_bytes()` thuộc extension `pgcrypto`, nhưng không migration nào `CREATE EXTENSION pgcrypto`. Migration 021 fail tại `UPDATE sessions SET csrf_token = encode(gen_random_bytes(32), 'hex')` → cascade failure: các phần sau (rate_limit_log, login_attempts tables) không được tạo.
  - **Fix**: Thêm `CREATE EXTENSION IF NOT EXISTS pgcrypto;` ở đầu migration 021 (idempotent).

### 🔧 Fix Schema Safety Column Names (HIGH)
- **[B3] `ensure_schema_safety` tạo bảng `permissions`/`role_permissions` với SAI column names** — Dùng `name` thay vì `name_vi`, `role_code` thay vì `role`, `permission_id` thay vì `permission_code`, `assigned_at` thay vì `granted_at`. Trên fresh deploy, safety_schema tạo bảng sai trước → migration 014 `CREATE TABLE IF NOT EXISTS` bị skip → INSERT fail vì column không tồn tại → cascading migration failure.
  - **Fix**: Đồng bộ column names với migration 014.

### 🔧 Fix Rate Limit Memory Leak (HIGH)
- **[B4] Rate limit cleanup task chạy trên instance throwaway** — `main.rs` gọi `spawn_cleanup_task(RateLimitState::new())` (instance MỚI với empty map), trong khi middleware thực tế dùng `RateLimitState::get_global()` (OnceLock singleton — instance KHÁC). Cleanup task làm trống map rỗng, không bao giờ dọn global map → memory leak theo thời gian.
  - **Fix**: `spawn_cleanup_task(RateLimitState::get_global().clone())`.

### 🔧 Fix Search Books/Groups (HIGH)
- **[B5] `tim_kiem.rs` query dùng cột không tồn tại `cover_image_url`** — Bảng `books` có cột `cover_url` (không phải `cover_image_url`); bảng `groups` có `cover_upload_id` (không có `cover_image_url`). Cả 2 query SELECT fail → search books + groups luôn trả empty.
  - **Fix**: Đổi `cover_image_url` → `cover_url` cho books; dùng subquery join `images` cho groups.

### 🔧 Fix Permission Inconsistency (HIGH)
- **[B6] admin_ky_thuat không đổi role được** — Comment trong `admin.rs` nói "admin_ky_thuat và admin_quan_li có quyền [users_change_role]", nhưng `user.rs::has_permission_code("users_change_role")` cho admin_ky_thuat trả về false. → admin_ky_thuat gọi `/admin/thanh-vien/{id}/role` sẽ bị 403.
  - **Fix**: Thêm `users_change_role` vào match arm của admin_ky_thuat trong `user.rs::has_permission_code`, và thêm `'users_change_role'` vào migration 021 cho admin_ky_thuat. Update `permission_count()` 40 → 41.

### 🐛 Fix UI/UX Bugs (MEDIUM)
- **[C1] Version drift trong footer** — `handlers/mod.rs:594` (placeholder_page) hiển thị "v0.9.21", `layout.html:422` hiển thị "v0.9.23". Cả 2 nên là "v0.9.25".
- **[C2] `BuddhaVowForm::validate` dùng byte length thay vì char count** — `content.len() < 10` đếm byte, không phải ký tự. Tiếng Việt có dấu là multi-byte UTF-8 (2-3 bytes/char) → validation sai.
  - **Fix**: Dùng `chars().count()`.
- **[C3] `notifications_list` TOCTOU — đánh dấu all-read sau fetch** — Handler SELECT 50 notifications, rồi UPDATE mark all unread → read. Nếu notification mới đến giữa SELECT và UPDATE, nó bị mark read mà chưa hiển thị cho user.
  - **Fix**: UPDATE chỉ mark những id đã fetch.

### 📦 Version Sync
- Bump version `0.9.24` → `0.9.25` ở: `Cargo.toml`, `src/main.rs` (log + health check response), `templates/layout.html` (footer), `src/handlers/mod.rs` (placeholder footer), `Dockerfile.coolify` (tag).
- Update phase 29 → 30 trong health check.
- Update `HEALTH_FEATURES` (+9 features v0.9.25).

---

## 📦 Phiên bản trước: v0.9.24 — Giai đoạn 29

**Giai đoạn 29: Permission Redesign + SVG Redesign + Security Hardening + Deploy Fix**

### 🔐 Redesign Phân Quyền — Admin ngang hàng (MAJOR)
- **Bỏ hierarchy cũ**: admin_ky_thuat(5) > admin_quan_li(4) > admin_cong_dong(3) → tất cả admin giờ **NGANG HÀNH** (level 3)
- **Mỗi admin có scope quyền riêng** theo phần phụ trách:
  - `admin_ky_thuat` (40 quyền) — system, security, technical infrastructure, media storage, analytics
  - `admin_quan_li` (40 quyền) — users (incl. change_role), content, community, fund, mail/notif
  - `admin_cong_dong` (45 quyền) — content, community, friends, mail, events, achievements, media mod
  - `mod` (15 quyền) — content moderation, chat moderation, basic community
- **Migration 021** — Re-seed `role_permissions`, thêm `csrf_token` vào sessions, `last_login_ip` vào users, `ip_address` vào audit_log, tạo bảng `rate_limit_log` + `login_attempts`
- **`can_manage_*()` dùng permission check** thay vì role_level — `can_manage_admin()` check `users_change_role`, `can_ban_user()` check `users_ban`, etc.

### 🪷 SVG Redesign
- **Redraw `favicon.svg`** — Hoa sen 3 lớp cánh (8+8+6) + 2 lá sen + tim sen vàng-xanh, gradient hồng-đỏ-vàng-xanh
- **Tạo `logo.svg`** — Logo đầy đủ 128x128 cho home hero, có background + glow filter
- **Tạo `logo-inline.svg`** — Logo inline 48x48 cho header navbar (thay emoji 🪷)
- **Layout.html + home.html** — Dùng SVG image thay emoji

### 🔒 Security Hardening
- **Security Headers middleware** — CSP, X-Frame-Options: DENY, X-Content-Type-Options: nosniff, Referrer-Policy, Permissions-Policy, HSTS (2 năm), Cross-Origin-Isolation
- **Rate Limiting middleware** — In-memory token bucket per IP + endpoint:
  - Auth: 10 req/phút | Upload: 10 req/phút | API: 60 req/phút | POST: 30 req/phút | General: 120 req/phút
  - 429 Too Many Requests + Retry-After khi exceed
- **CSRF Protection middleware** — Log-only mode (v0.9.24), block mode ở v0.9.25
- **Audit log IP tracking** — Track IP mọi admin action + login
- **Login attempts table** — Detect brute-force (sẽ integrate auth handler v0.9.25)

### 🐛 Fix Deploy
- **v0.9.23 không deploy thực sự** — Production vẫn chạy v0.9.22. Fix v0.9.24: bump tag image, verify deploy, restart Coolify app thủ công nếu cần.

---

## 📦 Phiên bản trước: v0.9.22 — Giai đoạn 27

### 👥 Trang Đội Ngũ Quản Lí (new feature)
- **Route**: `GET /doi-ngu-quan-li` — công khai, không yêu cầu đăng nhập
- **Nội dung**: Hiển thị 4 thành viên đội ngũ quản trị với thông tin chi tiết
  - Đỗ Minh Đức — 👑 Admin Quản Lí (quản lí chuyên mục hỏi đáp)
  - Võ Đăng Trọng Nghĩa (Thích Giác Ti) — 🧭 Admin Phát Triển
  - Đỗ Văn Cường — ⚙️ Admin Kỹ Thuật (hiện tại đã lui về hỗ trợ)
  - Nguyễn Đình Minh Hiếu — 💻 Admin Kỹ Thuật (hiện tại đang làm chính)
- **UI**: Card grid responsive (1 cột mobile, 2 cột desktop), gradient accent theo role, Facebook link, hệ thống phân cấp quản lí
- **Navigation**: Thêm vào mega menu (Khám Phá → Hệ Thống), footer (Hệ Thống), tong_quan hub

### 🐛 Fix SQL Injection (security fix)
- **Bug**: `bang_xep_hang.rs` và `quy_tu_bi.rs` dùng `format!()` để interpolate `limit` vào SQL — tiềm năng SQL injection nếu giá trị không phải hardcoded
- **Fix**: Bind `limit` as `$1` parameter thay vì string interpolation
  - `fetch_leaderboard()` — 5 tab queries (a, i, k, today, streak)
  - `fetch_streak_leaderboard()` — streak CTE
  - `fetch_recent_donations()`, `fetch_top_donors()`, `fetch_recent_expenses()` — quỹ từ bi

### 🎨 UI Fix
- Thêm link "👥 Đội Ngũ" vào mega menu, footer, navigation
- Cập nhật footer version v0.9.22

---

## 📦 Phiên bản trước: v0.9.20 — Giai đoạn 25

**Giai đoạn 25: Live Chat Total Fix + Sound Effects + Animations + Performance**

### 🐛 Fix Live Chat Cộng Đồng (bug user report)
- **Bug**: Admin/Mod không gửi được tin nhắn trong live chat của nhóm cộng đồng khi chưa tham gia nhóm
- **Root cause**: WebSocket handler `chat_ws_upgrade` yêu cầu user phải là `active member` của nhóm — không có bypass cho admin/mod
- **Fix**: Thêm `can_chat_any_group()` method cho User — admin + mod được chat trong BẤT KỲ nhóm nào
- **Frontend**: Template `community/group.html` hiển thị form chat cho admin/mod ngay cả khi chưa tham gia nhóm
- **Alpine.js**: `isMember` flag trong `liveChat()` component = `membership.status == "active" || user.is_staff()`

### 🎨 Hiệu ứng tin nhắn Admin/Mod (new feature)
- **Admin Kỹ Thuật (admin_ky_thuat) — Coder Effect**: Phong cách Matrix Terminal cực ngầu
  - Nền đen `#0a0e0a`, chữ xanh lá `#00ff41` phát sáng
  - Font monospace (Courier New / Monaco / Menlo)
  - Scan-line animation chạy liên tục
  - Border glow + box-shadow pulse
  - Prefix `[SYS]` trước tên
  - Avatar viền xanh lá pulse glow
- **Admin Quản Lý (admin_quan_li) — Premium Gold Frame**: Khung vàng luxury
  - Background gradient `#fffbeb → #fef3c7`
  - Border 2px gold + border-left 4px amber
  - Badge 👑 ở góc trên-phải
  - Box-shadow vàng ấm áp
- **Admin Cộng Đồng (admin_cong_dong) — Shield Blue Frame**: Khung xanh dương khiên
  - Background gradient `#eff6ff → #dbeafe`
  - Border 2px blue + border-left 4px navy
  - Badge 🛡️ ở góc trên-phải
  - Box-shadow xanh dương
- **Mod — Moderator Teal Frame**: Khung teal nổi bật
  - Background gradient `#f0fdfa → #ccfbf1`
  - Border 2px teal
  - Badge 📜 ở góc trên-phải
  - Box-shadow teal
- **Role badge mini** cạnh tên author trong chat (⚙️ SYS / 👑 ADMIN / 🛡️ ADMIN / 📜 MOD)
- **Dark mode overrides** cho từng role — giữ đặc trưng khi user chuyển sang dark mode
- Áp dụng cho cả 3 loại chat: Live Chat nhóm, Chat Chung toàn platform, DM 1-1

### 📜 Chức vụ Mod mới (new role)
- **Hierarchy mới**: admin_ky_thuat (5) > admin_quan_li (4) > admin_cong_dong (3) > **mod (2)** > member (1)
- **Mod có quyền**:
  - Xem `/admin` (redirect về `/admin/thanh-vien`)
  - Xem `/admin/thanh-vien` (danh sách thành viên)
  - Xem `/admin/cong-dong/cam-ngo` (duyệt cảm ngộ)
  - Xem các trang placeholder (`/admin/cong-dong/nhom`, `/admin/kinh-sach`, `/admin/binh-luan`, `/admin/quy-tu-bi`)
  - Chat trong BẤT KỲ nhóm cộng đồng nào (không cần membership)
  - Hiển thị badge 📜 Mod trong chat, profile, header
- **Mod KHÔNG có quyền**:
  - Đổi role user khác (chỉ admin_ky_thuat + admin_quan_li)
  - Ban/activate user (chỉ admin_ky_thuat)
  - Truy cập 3 dashboard admin riêng (`/admin/ky-thuat`, `/admin/cong-dong`, `/admin/quan-li`)
- **Migration 020**: drop old `users_role_check` constraint + add new constraint cho phép 'mod'
- **DB safety check** trong `db/mod.rs` cũng được cập nhật để đảm bảo 'mod' được chấp nhận
- **Admin user list**: thêm option "📜 Mod" trong dropdown đổi role, sắp xếp mod sau admin_cong_dong
- **Form đổi role** trong `/admin/thanh-vien` có nút cho Mod (teal background)

### 🔧 Code Quality & Cleanups
- `is_admin()` method giờ return true chỉ cho 3 role admin (KHÔNG bao gồm mod)
- Thêm `is_mod()` method — true chỉ cho role 'mod'
- Thêm `is_staff()` method — true cho admin HOẶC mod (dùng cho các quyền cơ bản)
- Thêm `can_chat_any_group()` method — admin + mod được chat mọi nhóm
- Cập nhật `role_level()`: mod=2, admin_cong_dong=3, admin_quan_li=4, admin_ky_thuat=5
- Cập nhật `can_manage_technical()`: chỉ admin (level >=3), mod không có
- Cập nhật `can_manage_community()`: mod trở lên (level >=2) — mod có quyền community
- Cập nhật `can_manage_admin()`: chỉ admin_quan_li trở lên (level >=4)
- `admin_dashboard_path()` cho mod = `/admin/thanh-vien` (không có dashboard riêng)
- Thêm `author_role` field vào `ChatMessageWithAuthor`, `GlobalChatMessageWithAuthor`, `DirectMessageWithAuthor`
- SQL queries thêm `u.role AS author_role` cho chat history + DM history
- WebSocket handlers (`handle_chat_socket`, `handle_global_chat_socket`, `handle_dm_socket`) lưu `author_role` khi persist message
- Cập nhật 403 Forbidden page hiển thị Mod trong hierarchy
- Version strings đồng bộ 0.9.19 ở mọi nơi: Cargo.toml, main.rs (3 nơi), layout.html, admin templates (4 files), khong-gian, handlers/, README, CHANGELOG

### 🎯 Mục tiêu Giai đoạn 24
- Fix bug user report: "không thể gửi tin nhắn trong live chat của cộng đồng"
- Thêm hiệu ứng đặc biệt cho tin nhắn admin (tech admin = coder, admin khác = khung riêng)
- Thêm chức vụ "mod" — dưới admin, trên thành viên, có quyền quản trị cơ bản
- Quét và fix toàn bộ lỗi logic + UI liên quan

## 📦 Phiên bản trước: v0.9.18 — Giai đoạn 23

**Giai đoạn 23: Mobile UI Overhaul + Admin Nav Logic Fix + Logout/Profile State Bug Fix**

### 🎨 Dark Mode (chế độ sáng/tối)
- **Toggle button trong header** — 🌙 (chuyển sang tối) / ☀️ (chuyển sang sáng)
- **Toggle trong mobile drawer** — nút riêng dễ chạm, full-width
- **Anti-FOUC script** — apply theme class BEFORE paint, không bị flash sáng/tối khi load
- **Cookie persistence** — `theme=lotus|dark` set 1 năm, server đọc được
- **localStorage fallback** — khách chưa login vẫn nhớ theme
- **API endpoint** `POST /api/theme` — upsert `user_settings.theme` trong DB (sync giữa các thiết bị)
- **Tailwind `darkMode: 'class'`** — config chính thức trong tailwind.config
- **CSS overrides** cho chat bubble, scrollbar, prayer ripple, chat popup

### 🐛 Admin Nav Fix (bug user report)
- **Bug**: các nav tile trong admin dashboard trỏ tới USER pages (`/cong-dong`, `/kinh-sach`, `/quy-tu-bi`) — admin click vào rồi bị redirect ra khỏi admin context
- **Fix**: tạo 4 route admin placeholder mới
  - `GET /admin/cong-dong/nhom` — Quản lý Nhóm Cộng Đồng (read-only list 20 nhóm mới nhất)
  - `GET /admin/kinh-sach` — Quản lý Kinh Sách (read-only list 20 sách mới nhất)
  - `GET /admin/binh-luan` — Quản lý Bình luận (read-only list 20 comment mới nhất)
  - `GET /admin/quy-tu-bi` — Quản lý Quỹ Từ Bi (read-only list 20 đóng góp mới nhất)
- Mỗi trang có: header, stats tổng quan, banner "Module đang phát triển", danh sách items, nút "Trở về dashboard"
- Tất cả 3 dashboard (ky-thuat / cong-dong / quan-li) đã cập nhật nav links trỏ tới admin pages thay vì user pages

### 📱 Mobile-first Polish
- **Bottom nav touch targets** — mỗi nút có `min-h-[44px]` (Apple HIG)
- **Border dark:border-slate-800** cho nút giữa 🪷 — đúng contrast trong dark mode
- **Mobile drawer dark mode** — tất cả 7 mục + theme toggle + logout có `dark:` variants
- **Smooth transitions** — `transition-colors` 150ms cho body, header, footer, nav
- **Animations preserved** — pulse/float/glow không bị transition chậm

### 🔧 Cleanup & Version Sync
- **Version strings đồng bộ 0.9.17 ở mọi nơi**: Cargo.toml, main.rs (3 nơi), layout.html, admin templates (4 files), khong-gian, handlers/mod.rs, README, CHANGELOG
- **Permission counts chính xác**: admin_ky_thuat 150/150, admin_quan_li 100/150, admin_cong_dong 75/150 (trước đây hiển thị sai "4/20", "4/30")
- **Permission summary trong template** dùng `{{ u.permission_count() }}` thay vì hardcode
- **`cargo check` sạch** — 0 warnings
- **`cargo clippy --release` sạch** — 0 warnings

## 📦 Phiên bản trước: v0.9.16 — Giai đoạn 21

**Giai đoạn 21: UI Redesign + Route Hub + Polish**

### UI/UX Redesign tổng thể
- **Layout redesign gọn nhẹ** — Header h-14 (thay vì h-16), logo 🪷 + tên rút gọn "TỪ BI", background paper nhẹ nhàng. Cards nhỏ gọn, ít chữ, nhiều icon.
- **Mega menu desktop 4 cột** — Hệ Thống / Cá Nhân / Kinh Sách / Cộng Đồng với 24+ link, fix lỗi route mồ côi.
- **Footer 6 cột** — Logo + 5 nhóm link với 30+ route đều có link truy cập.
- **Home page compact** — Hero ngắn, bỏ Prayer Counter Demo dài dòng, thêm section Khám Phá 12 card link.
- **Trang /tong-quan redesign thành Hub đẹp** — 8 nhóm: Chuyên Mục / Hệ Thống / Kinh Sách / BXH / Cá Nhân / Cộng Đồng / Quản Trị / Liên Kết Nhanh.

### Route Hub mở rộng
- **Health Check link** từ /tong-quan (icon 💓)
- **5 Thư Viện Kinh Sách** có link từ /tong-quan, mega menu, footer
- **5 BXH tabs** (a/i/k/today/streak) có link từ /tong-quan, mega menu, footer
- **Admin Dashboard quick links** từ /tong-quan (chỉ admin thấy)
- **Cộng Đồng quick links** — Lướt Nhóm, Lướt Chủ Đề, Tạo Nhóm, Tạo Chủ Đề

### Giữ nguyên theo yêu cầu user
- **Bottom nav mobile** giữ nguyên icon/label (Trang Chủ / Cộng Đồng / 🪷 Tổng Quan / Bạn Bè / Niệm Phật)
- **Mobile menu 3 gạch** giữ nguyên 7 mục — không thêm mục nào

### Code Quality
- `cargo check` sạch — 0 warnings
- `cargo clippy --all-targets` sạch — 0 warnings
- Version strings đồng bộ 0.9.16 ở mọi nơi

### Bug fixes critical
- **Niệm Phật counter không bị lệch trái** sau click (HTMX response giữ nguyên class `text-center mb-4`)
- **Streak (số ngày tu liên tiếp) tính đúng** — fix timezone mismatch giữa `chrono::Local::now()` và `CURRENT_DATE` của PostgreSQL trong Docker (TZ=UTC)
- **Tổng niệm / niệm hôm nay cập nhật ngay lập tức** sau khi niệm — dùng HTMX `hx-swap-oob` để swap nhiều element cùng lúc (counter + 4 stats card + footer)
- **Form Cầu Nguyện / Sám Hối / Hồi Hướng gửi được** — fix bug `hx-post=""` (rỗng) chặn submit. Tách thành 3 form riêng biệt với `hx-post` URL cố định
- **`practice_logs` upsert không còn nuốt error** — log + rollback nếu fail (trước đây `let _ =` làmtoday_niem không cập nhật dù a_balance vẫn tăng)

### UI/UX redesign
- **Bảng quản trị Admin Kỹ Thuật redesign theo ảnh tham chiếu** — dark theme, mobile-first, 2-col stats grid (số đỏ coral) + 2-col nav tiles grid
- **Hiển thị đúng 150 quyền** — admin_ky_thuat có 150 quyền (trước đây hiển thị hardcoded "6/50")
- **15 nav tiles điều hành** — Hướng dẫn, Phê duyệt, Thành viên, Nhóm, Kinh sách, Báo cáo, Bình luận, Từ vựng cấm, Nội dung đánh dấu, Quản lý tag, VIP, Quỹ Từ Bi, Bảng xếp hạng, Nhật ký, Health check
- **Permission matrix 10 nhóm × 10 quyền** — hiển thị đầy đủ 150 quyền chia 10 nhóm (system, users, content, community, kinh_sach, fund, achievements, security, media, analytics)

### Navigation overhaul (theo yêu cầu user)
- **Menu 3 gạch rút gọn chỉ còn 7 mục**: Không Gian, Cộng Đồng, Bạn Bè, Kinh Sách, Hồ Sơ, Quản Trị (nếu admin), Thoát
- **Bottom nav đổi icon**: Trang Chủ → 🏠 ngôi nhà, nút giữa → 🪷 hoa sen (Tổng Quan)
- **Các mục khác phân bổ hợp lý**: Tổng Quan/Quỹ/Bảng Xếp Hạng/Thành Tích/Thương Thành/Tìm Kiếm → truy cập qua nút giữa 🪷; Cài Đặt/Tin Nhắn/Hộp Thư/Thông Báo/Tìm Bạn → trong trang /ban-be và /ca-nhan; Kinh Phật/Kinh Đạo/Tìm Sách/Tạo Nhóm → trong trang /kinh-sach và /cong-dong

## Tầm Nhìn

Xây dựng một hệ sinh thái giúp mọi người có thể ứng dụng Từ Bi vào cuộc sống, tu học và giải trí, từ đó hiểu rõ hơn về bản chất của khổ đau, giác ngộ và giải thoát.

**Triết lý cốt lõi:** Tu cũng niệm Phật. Chơi cũng niệm Phật.

## Công Nghệ

| Thành phần | Công nghệ |
|-----------|-----------|
| Backend | Rust 1.97.1 + Axum 0.8 |
| Template | Askama 0.14 (type-safe HTML templates) |
| Database | PostgreSQL 17 + SQLx (async, compile-time checked) |
| Frontend | HTMX (server-driven UI) + Alpine.js (reactive) |
| Styling | Tailwind CSS |
| Auth | Google OAuth 2.0 (OpenID Connect — userinfo) — đăng nhập duy nhất |
| Container | Docker (multi-stage build với Rust 1.97.1-slim-bookworm, image final ~30 MB) |
| CI/CD | GitHub Actions (build → push GHCR) + Coolify API (auto pull image → deploy) |
| Registry | GHCR (ghcr.io/mhieuhonda/tubi-app) — image public |

## 4 Chuyên Mục Chính

1. 🌍 **Không Gian** – Không gian cá nhân, cộng tu, niệm Phật
2. 👥 **Cộng Đồng** – Diễn đàn, nhóm, chủ đề, live chat
3. 👤 **Bạn Bè** – Kết nối, nhắn tin, gửi thư
4. 📚 **Kinh Sách** – Thư viện kinh sách Phật giáo & Đạo giáo

---

## Lộ Trình 25 Giai Đoạn Phát Triển

### Giai đoạn 1: Kiến tạo nền móng — Thiết lập dự án & hạ tầng cốt lõi ✅ (v0.1)
- Khởi tạo project Rust (Axum + Askama + SQLx + PostgreSQL)
- Cấu hình HTMX + Alpine.js + Tailwind CSS
- Thiết kế database schema nền tảng (users, sessions)
- Trang landing page / trang chủ
- Hệ thống template layout (header, footer, navigation)
- Cấu hình domain `tubi.louis.vangioitutien.com`
- **Mục tiêu:** Server chạy được, hiển thị trang chủ với giao diện cơ bản

### Giai đoạn 2: Hệ thống xác thực — Đăng ký & Đăng nhập ✅ (v0.2)
- Form đăng ký thành viên (email, mật khẩu, tên hiển thị)
- Đăng nhập (email + password)
- Session management (cookie-based, SQLx session store)
- Logout & bảo vệ route (xoá session khỏi database)
- Xác thực người dùng từ session cookie trên mọi trang
- Kiểm tra `is_active` khi đăng nhập (chặn tài khoản bị vô hiệu)
- Migrate database: bảng `users`, `sessions`
- **Mục tiêu:** Thành viên có thể đăng ký, đăng nhập, đăng xuất

### Giai đoạn 3: Chuyển sang Google OAuth — Đăng nhập duy nhất bằng Google ✅ (v0.3)
- Tích hợp Google OAuth 2.0 (Authorization Code Flow)
- **Bỏ hoàn toàn form đăng ký email/password** — web chỉ còn đăng nhập/đăng ký bằng Google
- Endpoint `/auth/google` → chuyển hướng sang Google consent
- Endpoint `/auth/google/callback` → đổi code lấy access_token, gọi userinfo, upsert user
- Tự động link tài khoản Google với tài khoản cũ (nếu email trùng nhau)
- State chống CSRF (cookie HttpOnly, SameSite=Lax, TTL 10 phút)
- Cookie session_id bảo mật (HttpOnly, SameSite=Lax, Secure khi production)
- Migration 002: `password_hash` NULL được, thêm `google_sub`, `avatar_url`, `email_verified`
- Trang `/dang-nhap` chỉ còn nút "Đăng nhập bằng Google"
- **Mục tiêu:** Người dùng chỉ đăng nhập qua Google; tài khoản cũ vẫn dùng được

### Giai đoạn 4: Hồ sơ thành viên & Hệ thống cấp bậc ✅ (v0.4)
- Trang hồ sơ cá nhân `/ca-nhan` với avatar, cấp bậc, thống kê A/K
- Form chỉnh sửa hồ sơ: tên hiển thị, pháp danh, pháp hiệu, bút danh, giới tính, tiểu sử
- Hệ thống cấp bậc 9 mức: 🌱 Người Mới → 👑 Đại Gia
  | # | Cấp bậc | Màu | Icon | Min K |
  |---|---------|-----|------|-------|
  | 1 | Người Mới | #9E9E9E | 🌱 | 0 |
  | 2 | Người Thường | #795548 | 🍃 | 1 |
  | 3 | Người Bình Thường | #558B2F | 🌿 | 10 |
  | 4 | Người Tốt | #388E3C | 🌳 | 100 |
  | 5 | Người Khá Tốt | #2E7D32 | 🌲 | 500 |
  | 6 | Người Rất Tốt | #1B5E20 | 🎋 | 1.000 |
  | 7 | Người Cực Kỳ Tốt | #00695C | 🏆 | 5.000 |
  | 8 | Thiện Nhân | #FFB300 | 🪷 | 10.000 |
  | 9 | Đại Gia | #FF6F00 | 👑 | 100.000 |
- Hiển thị icon + tên cấp bậc trên header (gần tên user)
- Hiển thị cấp bậc hiện tại + tiến độ各级 bậc trên profile
- Migration 003: thêm cột `phap_danh`, `phap_hieu`, `but_danh`, `gender`, `bio` vào `users`; tạo bảng `member_ranks` + seed 9 cấp bậc mặc định
- Endpoint POST `/ca-nhan/cap-nhat` để cập nhật hồ sơ (validate input, không cho sửa email/rank/số dư)
- **Mục tiêu:** Hồ sơ hoạt động, cấp bậc hiển thị đúng

### Giai đoạn 5: Hạ tầng deploy — Docker + Coolify + storage ảnh ✅ (v0.5)
- **Dockerfile multi-stage** với Rust 1.97.1, image final ~30 MB (glibc + stripped binary)
- **Coolify** deploy trên sub VPS, domain `tubi.louis.vangioitutien.com`
- **PostgreSQL 17** trên sub VPS (10.187.247.3) làm database + storage
- **API upload ảnh** `/api/upload-image` (max 5 MB/ảnh, JPEG/PNG/WebP/GIF)
- Migration 004: bảng `images` + `audit_log` + trigger `updated_at` tự động
- Auto-run migrations khi khởi động (set `RUN_MIGRATIONS=true`)
- Health check endpoint `/api/health` giờ check cả DB
- **[SECURITY]** Bỏ GET `/dang-xuat` để chống CSRF — chỉ còn POST
- **[SECURITY]** Logout form dùng JavaScript submit thay vì link GET
- Graceful shutdown (30s timeout), 4 workers
- DB pool size tunable qua env `DB_MAX_CONNECTIONS`
- Release profile tối ưu (LTO thin, strip symbols, panic=abort)
- **Mục tiêu:** Web chạy production ổn định, sẵn sàng cho giai đoạn 6+

### Giai đoạn 6: Cộng Đồng Foundation — Nhóm + Chủ Đề + Bình luận ✅ (v0.6)
- **Chuyên mục Cộng Đồng chính thức ra mắt** — không còn placeholder
- **Hệ thống Nhóm (Groups)**:
  - Tạo nhóm với tên, mô tả, phân loại (9 categories), visibility (public/private/hidden), require_approval
  - Trang nhóm `/cong-dong/nhom/{slug}` hiển thị thông tin + danh sách chủ đề
  - Tham gia / rời nhóm (POST-only, chống CSRF)
  - Slug tự sinh từ tên, đảm bảo duy nhất (thêm hậu tố UUID nếu trùng)
- **Hệ thống Chủ Đề (Topics)** — bài viết trong nhóm (diễn đàn):
  - Tạo chủ đề với title (max 200) + body (multiline)
  - Chỉ thành viên active mới được tạo chủ đề
  - Hỗ trợ ghim (`is_pinned`) + khoá (`is_locked`) ở schema, UI giai đoạn sau
  - Tự tăng `view_count` khi xem chủ đề
- **Hệ thống Bình luận (Comments)** — bình luận trên chủ đề:
  - Form bình luận nhanh ngay trên trang chủ đề
  - Hỗ trợ reply (parent_id) ở schema, UI nested giai đoạn sau
  - Validate body (không rỗng, tối đa 5000 ký tự)
  - Không bình luận được nếu chủ đề bị khoá
- **Migration 005**: 5 bảng mới (group_categories, groups, group_members, topics, comments) + 4 triggers + 9 seed categories
- **Giao diện Lướt**: tabs "Lướt Nhóm" / "Lướt Chủ Đề" trên trang chính Cộng Đồng
- **10 endpoint** mới cho Cộng Đồng (xem routes bên dưới)
- **Mục tiêu:** Cộng Đồng hoạt động đầy đủ — tạo nhóm, tham gia, tạo chủ đề, bình luận

### Giai đoạn 7: Live Chat WebSocket trong Nhóm ✅ (v0.9.2)
- **Live Chat real-time (WebSocket) trong Nhóm** — điểm khác biệt cốt lõi của Cộng Đồng Ứng Dụng Từ Bi so với Telegram/Zalo/Facebook Group
- Theo thiết kế `HieuLouis/Giao Diện Cộng Đồng Trong Ứng Dụng.docx`: mỗi nhóm có Danh sách Chủ Đề (diễn đàn, ~65% chiều cao) + Live Chat (real-time, ~35% chiều cao)
- **Triết lý**: Live Chat chỉ để giao lưu / kết bạn / tán gẫu / hỏi nhanh. Mọi nội dung có giá trị nên được chuyển thành Chủ Đề (lưu trữ tri thức lâu dài)
- **WebSocket endpoint** `GET /ws/cong-dong/nhom/{slug}` (Axum 0.8 `WebSocketUpgrade`)
  - Auth bằng session_id cookie, chỉ member active mới chat được
  - ChatHub quản lý `broadcast::Sender` per-group, capacity 256
  - Spawn 2 task song song: send (broadcast→client) + recv (client→DB→broadcast)
- **REST endpoint** `GET /api/cong-dong/nhom/{slug}/chat-history?limit=50&before={iso8601}` — paginated history
- **Migration 006**: bảng `group_chat_messages` (body VARCHAR(500), 2 index)
- **Frontend**: Alpine.js `liveChat()` component — auto-reconnect exponential backoff, auto-scroll, formatTime vi-VN
- **Mục tiêu:** Thành viên trong nhóm có thể chat real-time, kết nối cộng đồng

### Giai đoạn 8: CI/CD tự động — GitHub Actions + Docker Image + Coolify ✅ (v0.9.4)
- **Quay lại GitHub Actions** nhưng với mô hình mới: GitHub Actions build & push Docker Image lên GHCR, Coolify auto pull image và deploy (không còn build từ source trên VPS như v0.9.1)
- **Workflow `.github/workflows/docker.yml`**:
  - Trigger: push lên `main` hoặc tag `v*`
  - Build multi-stage Docker image với Rust 1.97.1-slim-bookworm (image final ~30 MB)
  - Push lên GHCR (`ghcr.io/mhieuhonda/tubi-app`) với multi-tag: `latest`, `sha-<short>`, `vX.Y.Z`, `vX.Y`
  - Buildx cache (type=gha) để tăng tốc build sau
  - Gọi Coolify API `/api/v1/applications/{uuid}/start` để trigger deploy
  - Coolify nhận yêu cầu → pull image `:latest` → redeploy container
- **Coolify app** chuyển từ `build_pack: dockerfile` (build từ source) sang `build_pack: dockerimage` (pull image từ registry)
- **GitHub Secrets**: `COOLIFY_API_TOKEN`, `COOLIFY_APP_UUID`
- **Lợi ích so với v0.9.1**:
  - Build trên GitHub-hosted runner (không tốn CPU/RAM VPS)
  - Image đã build sẵn, deploy chỉ mất vài giây (pull + restart)
  - Rollback dễ dàng: đổi tag trong Coolify về `sha-<old>` hoặc `v0.9.3`
  - Multi-arch support sẵn sàng (chỉ cần thêm `platforms: linux/amd64,linux/arm64`)
  - Image có thể ký (provenance/SBOM optional)
- **Mục tiêu:** Push code → tự động build & deploy trong < 5 phút, không cần thao tác thủ công

### Giai đoạn 9: Module Bạn Bè — Friends + DM + Mail + Notifications ✅ (v0.9.5)
- **BB-01 Kết bạn (Friend System)**: gửi/nhận/từ chối/hủy lời mời kết bạn; tìm user theo tên/email/pháp danh
- **BB-02 Nhắn tin 1-1 (Direct Messaging)**: WebSocket realtime `/ws/ban-be/tin-nhan/{conversation_id}` (reuse ChatHub pattern, `DmChatHub` per-conversation capacity 128), REST history paginated, `dmChat()` Alpine.js component với auto-reconnect backoff
- **BB-03 Gửi thư (Mail/Inbox)**: thư dài (subject max 200 + body TEXT), hộp thư đến/đi, auto mark as read, notifications cho recipient
- **Notification Center**: bảng `notifications` (JSONB payload), bell icon ở header với red badge (poll mỗi 30s), types: friend_request/friend_accept/mail/dm/system/group_invite
- **4 migration mới**: `008_friendships.sql`, `009_conversations_direct_messages.sql`, `010_mails.sql`, `011_notifications.sql`
- **Bug fixes**: Fix live chat tổng luôn báo "đang kết nối..." (bỏ check `document.cookie.includes('session_id')` vì cookie HttpOnly không đọc được qua `document.cookie`); thêm server-side logging cho WS errors; fix version strings v0.9.3 → v0.9.5
- **Mục tiêu:** Thành viên có thể kết bạn, nhắn tin realtime 1-1, gửi thư dài, nhận thông báo — bù lỗ hổng social của v0.9.4

### Giai đoạn 13: Không Gian Cá Nhân & Niệm Phật ✅ (v0.9.9)
- **Chuyên mục Không Gian chính thức ra mắt** — không còn placeholder, là 1 trong 4 trụ cột chính của app
- **Niệm Phật Counter** (HTMX realtime):
  - Nút 🪷 lớn ở giữa trang, mỗi lần nhấn = +1 Niệm Lực A
  - `POST /api/niem-phat` — upsert `practice_logs` (1 row/user/day) + increment `a_balance` trong transaction
  - Hiệu ứng pulse lotus + scale animation khi nhấn
- **Tượng Phật** (4 chức năng theo `HieuLouis/Hệ Thống Và Chức Năng Chi Tiết.docx` mục I.6):
  - 🙏 **Cầu Nguyện** — `POST /tuong-phat/cau-nguyen` → +1 Nguyên lực I
  - 🙇 **Sám Hối** — `POST /tuong-phat/sam-hoi` → +2 Nguyên lực I
  - 🌸 **Hồi Hướng** — `POST /tuong-phat/hoi-huong` → +3 Nguyên lực I
  - (Ủng Hộ chưa làm — cần tích hợp quảng cáo, sẽ thêm ở giai đoạn sau)
  - Form vow: nội dung 10–2000 ký tự, có checkbox "hiển thị công khai trên bảng Kính Nguyện"
- **Nhật Ký Tu Học** (7 ngày gần nhất):
  - Biểu đồ cột (bar chart) hiển thị số lần niệm mỗi ngày
  - Tính `streak` (số ngày liên tiếp có niệm)
- **Bảng Kính Nguyện** — danh sách vow công khai gần nhất (20 cái), hiển thị tác giả + loại vow + nội dung
- **Hệ thống điểm mở rộng**:
  - `a_balance` (Niệm Lực A) — có từ Giai đoạn 1
  - `k_balance` (Tiền K) — có từ Giai đoạn 1
  - **`i_balance` (Nguyên lực I) — MỚI v0.9.9** — phần thưởng từ Tượng Phật
- **Migration 015**: thêm cột `i_balance` vào `users` + bảng `practice_logs` + bảng `buddha_vows` + triggers + index
- **5 endpoint** mới cho Không Gian + 1 API JSON stats
- **Bug fix CRITICAL (login "lỗi ghi nhận người dùng")**:
  - `auth.rs` `upsert_google_user` thêm `SELECT` fallback sau khi `INSERT ... RETURNING` fail (tránh block login khi struct/column mismatch)
  - Truncate `display_name` về 100 ký tự (Google profile name có thể dài hơn `VARCHAR(100)`)
  - Log chi tiết lỗi theo loại (`ColumnNotFound` / `Database` / `Decode`) để debug triệt để
- **Admin user list redesign (theo ảnh tham chiếu)**:
  - Bỏ table layout, chuyển sang card-based compact grid (2 cột trên desktop)
  - Mỗi card: tên (bold) + role badge (top-right), @handle, email, footer với A/K metrics + online status ("Đang hoạt động" / "X phút trước" / "Bị khóa")
  - Actions dropdown (⋮) thay vì select inline — UI gọn hơn
  - `last_session_at` từ subquery `MAX(sessions.created_at)` để hiển thị hoạt động gần nhất
- **Mục tiêu:** Không Gian cá nhân hoạt động — niệm Phật tích lũy A, phát nguyện trước Tượng Phật tích lũy I, theo dõi nhật ký tu học hàng ngày

### Giai đoạn 12: 50 quyền chi tiết + 3 giao diện admin riêng biệt ✅ (v0.9.8)
- **Hệ thống 50 quyền chi tiết (Granular Permissions)** — 5 nhóm x 10 quyền:
  - `system` (10) — hệ thống, server, DB, config, logs, metrics, backup, debug
  - `users` (10) — xem, sửa, đổi role, ban, xóa, sessions, OAuth, export
  - `content` (10) — duyệt, sửa, xóa, ghim, khoá, danh mục, tags, mod comments/reviews
  - `community` (10) — nhóm, events, chat, members, broadcast, invites, archive, merge
  - `kinh_sach` (10) — sách, chương, upload, danh mục, reviews, donations, mail, notif, analytics, API
- **Nâng Admin Kỹ Thuật lên CHỨC VỤ CAO NHẤT** — toàn bộ 50 quyền
  - Hierarchy mới: Admin Kỹ Thuật (50 quyền) > Admin Quản Lý (30 quyền) > Admin Cộng Đồng (20 quyền) > Thành Viên (0)
- **3 giao diện bảng quản trị hoàn toàn khác nhau**:
  - ⚙️ `/admin/ky-thuat` — Phong cách Coder/Terminal — tối, Matrix-like, cực ngầu
  - 🛡️ `/admin/cong-dong` — Phong cách Community Mod — xanh dương, social, ấm áp
  - 👑 `/admin/quan-li` — Phong cách Executive/Premium — vàng, luxury dashboard
- **Migration 014**: bảng `permissions` + `role_permissions` + view `v_user_permissions` + function `user_has_permission()`
- **Bug fix CRITICAL**: `USER_COLUMNS` trong auth.rs thiếu cột `role` — Google OAuth login bị hỏng
- **Mục tiêu:** Admin Kỹ Thuật có toàn quyền, 3 kiểu admin có giao diện riêng, hệ thống phân quyền chi tiết

### Giai đoạn 11: Hệ thống vai trò Admin & Phân quyền cộng đồng ✅ (v0.9.7)
- **Lần đầu tiên app có hệ thống phân quyền rõ ràng** — 4 vai trò:
  - `member` — Thành Viên (mặc định)
  - `admin_ky_thuat` — Admin Kỹ Thuật (hệ thống, server, DB, mã nguồn)
  - `admin_cong_dong` — Admin Cộng Đồng (duyệt nội dung, mod diễn đàn)
  - `admin_quan_li` — Admin Quản Lý (super admin — quyền cao nhất)
- **Hierarchy:** Admin Quản Lý > Admin Cộng Đồng > Admin Kỹ Thuật > Thành Viên
- **Migration 013**: thêm cột `role` vào `users` + CHECK constraint + index
  + UPSERT `khongdich.admin@gmail.com` → `admin_ky_thuat`
- **Hiển thị chức vụ trên hồ sơ** (`/ca-nhan`) — role badge bên cạnh rank badge,
  trong bảng Thông Tin Tài Khoản, và nút "Vào trang Quản Trị" (chỉ admin)
- **Hiển thị chức vụ trong header** — role badge nhỏ + link "⚙️ Quản Trị"
  (chỉ admin nhìn thấy)
- **Trang Quản Trị** `/admin`:
  - Dashboard: tổng users, active users, admin count, groups/topics/comments/books/mails,
    cảm ngộ chờ duyệt
  - `/admin/thanh-vien` — Danh sách thành viên + UI đổi role (chỉ Admin Quản Lý)
  - `POST /admin/thanh-vien/{user_id}/role` — Đổi role user
  - Permission gate: user không phải admin → 403 Forbidden
- **User model** mở rộng: 8 helper methods (`role_display`, `role_icon`, `role_color`,
  `role_level`, `is_admin`, `is_admin_ky_thuat`, `is_admin_cong_dong`, `is_admin_quan_li`)
- **Health check** `/api/health` thêm `admin` stats + `roles` object
- **Mục tiêu:** Hệ thống có admin rõ ràng, có trang quản trị cơ bản, nền tảng
  cho các giai đoạn tiếp theo (duyệt cảm ngộ, mod community, ban user, ...)

### Giai đoạn 10: Kinh Sách — Thư viện kinh sách Phật giáo & Đạo giáo ✅ (v0.9.6)
- **Chuyên mục Kinh Sách chính thức ra mắt** — không còn placeholder
- **5 Thư Viện Chính** (theo `HieuLouis/Hệ Thống Và Chức Năng Chi Tiết.docx` mục IV):
  - 🪷 Phật Gia — Kinh điển, luận thư, pháp thoại Phật giáo
  - ☯️ Đạo Gia — Đạo Đức Kinh, Nam Hoa Kinh, thư tịch Đạo giáo
  - 📜 Kinh Văn — Kinh văn tụng đọc, chú giải, nghi thức
  - 💎 Sách Quý — Khoa học, Triết học, Tâm học, Văn học
  - ⭐ Quan Trọng — Bài viết quan trọng do Quản Lý chọn lựa
- **Hệ thống Sách (Books)**:
  - Sách điện tử hoàn chỉnh (`book_type=single`) hoặc theo chương (`book_type=multi`)
  - Trang sách `/kinh-sach/{slug}` hiển thị thông tin + danh sách chương + cảm ngộ
  - Tự tăng `view_count` khi xem sách/chương
  - Phân loại theo 5 thư viện + 3 ngôn ngữ (vi/en/zh, ưu tiên Tiếng Việt)
- **Hệ thống Chương (Chapters)**:
  - Trang đọc chương `/kinh-sach/{slug}/chuong/{chapter_slug}` với sidebar mục lục sticky
  - Điều hướng trước/sau giữa các chương
  - Tự tăng `view_count` chương khi đọc
- **Hệ thống Cảm Ngộ (Reviews)**:
  - Form cảm ngộ ngay trên trang sách (tối thiểu 100 chữ, tối đa 10.000 chữ)
  - **Phải qua xét duyệt** mới hiển thị công khai (status: `pending` → `approved`)
  - 1 user chỉ được viết 1 cảm ngộ/sách (unique index), có thể edit
  - Hiển thị trạng thái cảm ngộ của user (chờ duyệt / đã duyệt / bị từ chối)
- **Tặng Hoa (Flowers)**:
  - 1 user chỉ tặng 1 hoa/sách (unique index `book_flowers`)
  - Tự tăng counter `flower_count` qua trigger
- **Tìm kiếm sách** `/kinh-sach/tim-kiem?q=` — dùng ILIKE + pg_trgm (fuzzy search)
- **Migration 012**: 5 bảng mới (`book_categories`, `books`, `book_chapters`, `book_reviews`, `book_donations`, `book_flowers`) + 5 triggers + seed 4 cuốn sách mẫu (Kinh A Di Đà, Đạo Đức Kinh, Kinh Tam Đại Hải, Kinh Pháp Cú)
- **12 endpoint** mới cho Kinh Sách
- **Health check** bổ sung `kinh_sach` stats (số sách, chương, cảm ngộ, tổng view)
- **Mục tiêu:** Thành viên có thể đọc kinh sách online, viết cảm ngộ, tặng hoa kính dâng — bù lỗ hổng kiến thức của v0.9.5

### Giai đoạn 15: Quỹ Từ Bi — Hệ thống quỹ cộng đồng ✅ (v0.9.11)
- **Chuyên mục Quỹ Từ Bi chính thức ra mắt** (`/quy-tu-bi`) — theo `HieuLouis/Hệ Thống Và Chức Năng Chi Tiết.docx` mục VI
- **Hệ thống đóng góp K vào quỹ**:
  - `POST /quy-tu-bi/dong-gop` — form đóng góp K (trừ từ `k_balance` của user, transaction-safe)
  - 5 loại quỹ: 🪷 Quỹ Chung · 📚 Quỹ Sách · 🕉️ Quỹ Tu · 🎁 Quỹ Quà · 🤝 Quỹ Thiện Nguyện
  - Hỗ trợ đóng góp ẩn danh, lời nhắn tùy chọn (max 500 ký tự)
  - Notification tự động cho admins khi có donation mới
- **Dashboard tổng quan**:
  - Hero số dư Quỹ (gradient xanh tubi, chữ vàng amber)
  - Stats grid: tổng K/A/I hệ thống · tổng lượt đóng góp
  - Quỹ theo chuyên mục: 5 card màu
  - Top 10 nhà hảo tâm (medal 🥇🥈🥉)
  - 20 đóng góp gần nhất (table với avatar + loại quỹ badge + lời nhắn)
  - 10 khoản chi tiêu gần đây (công khai, minh bạch)
- **Migration 016**: 3 bảng mới (`fund_donations`, `fund_campaigns`, `fund_expenses`) + view `v_fund_summary` + 6 index + 2 trigger
- **API endpoint** `GET /api/quy-tu-bi/stats` — JSON tổng quan
- **Bug fix CRITICAL (lỗi đăng nhập)**:
  - Fix production vẫn chạy v0.9.9 dù v0.9.10 đã build & push lên GHCR
  - Nguyên nhân: `Dockerfile.coolify` dùng `FROM :latest` → Docker daemon cache stale digest
  - Giải pháp: đổi sang `FROM :0.9.11` (tag semver unique → Docker chắc chắn pull image mới)
  - Đảm bảo v0.9.10's safety schema fix được deploy (fix lỗi "column i_balance does not exist")
- **Mục tiêu:** Thành viên có thể đóng góp K vào quỹ chung, công khai minh bạch, fix triệt để lỗi đăng nhập

### Giai đoạn 12–25: *(xem kế hoạch chi tiết trong HieuLouis/)*

### Giai đoạn 18: Navigation Overhaul + User Hub + Settings ✅ (v0.9.14)
- **Trang Tổng Quan (User Hub) `/tong-quan`** — liệt kê TẤT CẢ 24 tính năng của app trong 4 nhóm (Chuyên Mục / Hệ Thống / Cá Nhân / Liên Kết Nhanh)
- **Mega Menu Desktop** — dropdown "🧭 Khám Phá" 3 cột với 18 link, fix lỗi route mồ côi trên desktop
- **Mobile Drawer đầy đủ** — 6 nhóm với 22 link (vs 4 chuyên mục + Hồ sơ + Thoát ở v0.9.13)
- **Bottom Nav redesign** — icon giữa đổi từ 🪷 sang 🧭 (Tổng Quan); tab cuối đổi từ "Không Gian" sang "🙏 Niệm Phật"
- **Footer 5 cột** — Giới Thiệu + Chuyên Mục + Hệ Thống + Cá Nhân + Khám Phá (vs 4 cột cũ)
- **Trang Cài Đặt `/cai-dat`** — 4 nhóm cài đặt: Riêng Tư, Thông Báo, Giao Diện, Chat & Niệm Phật
- **Migration 017**: bảng `user_settings` (16 cột) + trigger `updated_at` + seed default
- **Mục tiêu**: User có thể truy cập MỌI route chỉ từ UI — không còn "route mồ côi" nào

### Giai đoạn 19: 150 Quyền + Thành Tích + Tìm Kiếm toàn cục ✅ (v0.9.14)
- **Hệ thống 150 quyền chi tiết** — mở rộng từ 50 → 150, thêm 100 quyền mới chia 10 nhóm × 10 quyền:
  - `fund` (10) — Quản lý Quỹ Từ Bi
  - `achievements` (10) — Quản lý Thành Tích
  - `security` (10) — Bảo mật & chống spam
  - `navigation` (10) — Quản lý UI/Navigation/Settings
  - `analytics` (10) — Phân tích & báo cáo
  - `media` (10) — Quản lý media/uploads
  - `friends` (10) — Quản lý Bạn Bè/DM
  - `mail` (10) — Quản lý Thư/Thông báo
  - `events` (10) — Quản lý Sự kiện/Cộng tu
  - `shop` (10) — Quản lý Thương Thành
- **Phân bổ mới**: admin_ky_thuat (150) > admin_quan_li (100) > admin_cong_dong (75) > member (0)
- **Hệ thống Thành Tích** — 30 achievements mẫu chia 6 nhóm (Niệm Phật, Tượng Phật, Cộng Đồng, Kinh Sách, Bạn Bè, Quỹ Từ Bi) với 5 độ hiếm (common → mythic)
- **Trang `/thanh-tich`** — cards thành tích đã đạt + progress bar cho đang tiến hành + 5 stats tổng quan
- **Trang `/tim-kiem`** — search đồng thời users + books + topics + groups, tối đa 10 kết quả mỗi loại
- **Migration 018**: 100 quyền mới + role_permissions (325 row, idempotent)
- **Migration 019**: 3 bảng (`achievements`, `user_achievements`, `achievement_progress`) + 2 view + function `check_and_grant_achievement()`
- **Mục tiêu**: Hệ thống phân quyền chi tiết 150 quyền, thành tích động viên user tu tập, tìm kiếm toàn cục

---

## Cấu Trúc Dự Án (Giai đoạn 12 / v0.9.8)

```
ungdungtubi/
├── src/
│   ├── main.rs              # Entry point + routes + auto-migrate + health check DB
│   ├── config.rs            # Config (DB, host, Google OAuth, static_dir, upload_dir)
│   ├── db/
│   │   └── mod.rs           # Database helpers (session cleanup)
│   ├── errors/
│   │   └── mod.rs           # AppError enum with HTTP response mapping
│   ├── handlers/
│   │   ├── mod.rs           # Page handlers + session auth + profile update + placeholder pages
│   │   ├── auth.rs          # google_login, google_callback, logout (POST-only)
│   │   ├── chat.rs          # Live Chat WebSocket + chat-history REST + ChatHub + GlobalChatHub
│   │   ├── community.rs     # Groups + Topics + Comments handlers + group cover upload
│   │   ├── kinh_sach.rs     # [v0.9.6] Kinh Sách (Books + Chapters + Reviews + Flowers)
│   │   ├── friends.rs       # [v0.9.5] Friends + DM + Mail + Notifications
│   │   ├── uploads.rs       # Upload ảnh API + change avatar (5MB max, SHA-256)
│   │   └── admin.rs         # [v0.9.7] Admin panel: dashboard + user list + role management
│   ├── models/
│   │   ├── mod.rs
│   │   ├── user.rs          # User, GoogleUserInfo, MemberRank, ProfileUpdate + role helpers (v0.9.7)
│   │   ├── community.rs     # Group, Topic, Comment, GroupMember, GroupCategory, ChatMessage
│   │   ├── friends.rs       # [v0.9.5] Friendship, Conversation, DirectMessage, Mail, Notification
│   │   └── kinh_sach.rs     # [v0.9.6] Book, BookChapter, BookReview, BookCategory
│   └── static/
│       ├── css/app.css
│       ├── js/app.js        # + liveChat() + globalChat() Alpine.js components
│       └── uploads/         # Nơi lưu ảnh user upload
├── templates/                # Askama templates (Vietnamese)
│   ├── layout.html
│   ├── home.html
│   ├── profile.html
│   ├── auth/
│   │   └── login.html
│   ├── admin/                # [v0.9.7] Admin panel (dashboard + user list + role UI)
│   │   ├── index.html
│   │   └── users.html
│   ├── community/            # Cộng Đồng (groups + topics + comments + live chat)
│   │   ├── index.html
│   │   ├── group.html
│   │   ├── topic.html
│   │   ├── create_group.html
│   │   └── create_topic.html
│   └── kinh-sach/            # [v0.9.6] Kinh Sách (books + chapters + reviews)
│       ├── index.html
│       ├── category.html
│       ├── search.html
│       ├── book.html
│       └── chapter.html
├── migrations/                # 13 migration files
│   ├── 001_create_users_sessions.sql
│   ├── 002_google_oauth.sql
│   ├── 003_member_profile_ranks.sql
│   ├── 004_storage_images_audit.sql
│   ├── 005_community_groups_topics_comments.sql
│   ├── 006_group_chat_messages.sql
│   ├── 007_global_chat_messages.sql
│   ├── 008_friendships.sql
│   ├── 009_conversations_direct_messages.sql
│   ├── 010_mails.sql
│   ├── 011_notifications.sql
│   ├── 012_kinh_sach.sql
│   └── 013_admin_roles.sql      # [v0.9.7] Hệ thống vai trò Admin (member/admin_ky_thuat/admin_cong_dong/admin_quan_li)
├── .github/workflows/
│   └── docker.yml            # [v0.9.4] Build & push GHCR (ghcr.io/mhieuhonda/tubi-app) + trigger Coolify API
├── HieuLouis/                # Tài liệu dự án
├── Cargo.toml                # v0.9.7, Rust 1.97, axum ws feature, release profile tối ưu
├── Cargo.lock                # Lock file (commit cho reproducible build)
├── Dockerfile                # Multi-stage Rust 1.97.1, ~30 MB
├── docker-compose.yml        # Dev environment (Postgres 17 + app)
├── .dockerignore             # Tối ưu build context
├── .env.example              # Template cấu hình môi trường v0.9
└── README.md
```

## Cài Đặt & Chạy

### Local dev với Docker Compose (Khuyên dùng)

```bash
# 1. Clone
git clone https://github.com/mhieuhonda/ungdungtubi.git
cd ungdungtubi

# 2. Tạo .env từ .env.example
cp .env.example .env
# Điền GOOGLE_CLIENT_ID, GOOGLE_CLIENT_SECRET (lấy từ Google Cloud Console)
# Đặt GOOGLE_REDIRECT_URI=http://localhost:8080/auth/google/callback

# 3. Chạy với Docker Compose (Postgres 17 + app cùng lúc)
docker compose up -d

# Server: http://localhost:8080
# DB: postgres://tubi:tubi_password@localhost:5432/ungdungtubi
```

### Local dev thủ công (không Docker)

```bash
# 1. Clone
git clone https://github.com/mhieuhonda/ungdungtubi.git
cd ungdungtubi

# 2. Tạo .env
cp .env.example .env
# Điền DATABASE_URL + GOOGLE_CLIENT_ID + GOOGLE_CLIENT_SECRET

# 3. Cấu hình Google OAuth
#    - Vào Google Cloud Console → APIs & Services → Credentials
#    - Tạo OAuth 2.0 Client ID (Web application)
#    - Thêm Authorized redirect URIs khớp với GOOGLE_REDIRECT_URI

# 4. Tạo database + chạy migrations
createdb ungdungtubi
psql -d ungdungtubi -f migrations/001_create_users_sessions.sql
psql -d ungdungtubi -f migrations/002_google_oauth.sql
psql -d ungdungtubi -f migrations/003_member_profile_ranks.sql
psql -d ungdungtubi -f migrations/004_storage_images_audit.sql

# 5. Chạy
cargo run
# Server: http://localhost:8080
```

### Production (CI/CD tự động qua GitHub Actions + Coolify)

Từ v0.9.4, dự án áp dụng mô hình CI/CD hoàn toàn tự động:

1. **Push code** lên branch `main` (hoặc tạo tag `v*`)
2. **GitHub Actions** tự động:
   - Build Docker image với Rust 1.97.1 (multi-stage, image final ~30 MB)
   - Push lên GHCR: `ghcr.io/mhieuhonda/tubi-app:{latest,sha-<short>,vX.Y.Z}`
   - Gọi Coolify API `/api/v1/applications/{uuid}/start` để trigger deploy
3. **Coolify** tự động:
   - Pull image `:latest` từ GHCR
   - Stop container cũ, start container mới
   - Run health check trên `https://tubi.louis.vangioitutien.com/api/health`
   - Cấp phát SSL tự động qua Traefik + Let's Encrypt (đã có sẵn)
4. Kiểm tra trạng thái: `https://tubi.louis.vangioitutien.com/api/health`

**Cấu hình đã có trên Coolify:**
- Sub VPS: `10.187.247.3` (Ubuntu 24.04, 6 CPU, 8 GB RAM) — server `vangioi-vps`
- PostgreSQL 17 chạy riêng trên sub VPS (DATABASE_URL set trong env của app)
- Domain: `https://tubi.louis.vangioitutien.com` (Traefik reverse proxy + Let's Encrypt)
- Healthcheck: `GET /api/health` (port 8080, scheme http, retries 5, start-period 60s)
- Volume bind: `/app/static/uploads` để giữ ảnh user upload giữa các lần deploy
- Auto-migrations: chạy khi khởi động (`RUN_MIGRATIONS=true`, `APP_ENV=production`)
- Sentinel: bật (giám sát VPS, push metrics mỗi 60s, giữ history 7 ngày)

**GitHub Secrets cần thiết:**
- `COOLIFY_API_TOKEN` — API token của Coolify (tạo ở User Settings → API Tokens)
- `COOLIFY_APP_UUID` — UUID của application trên Coolify

**Rollback:** Đổi tag image trong Coolify từ `:latest` sang `:sha-<old>` hoặc `:v0.9.3` → deploy lại.

**Lịch sử thay đổi CI/CD:**
- v0.5 — GitHub Actions đầu tiên (build + push GHCR + webhook)
- v0.9.1 — Bỏ GitHub Actions, chuyển sang deploy thủ công qua Coolify (build từ source trên VPS)
- v0.9.4 — Quay lại GitHub Actions nhưng với mô hình Docker Image (Coolify pull image, không build từ source)

## Routes (v0.9.7)

| Method | Path | Mô tả | Auth |
|--------|------|-------|------|
| GET | `/` | Trang chủ — Không Gian | Public |
| GET | `/dang-nhap` | Trang đăng nhập (nút Google) | Public |
| POST | `/dang-nhap` | Alias chuyển hướng tới `/auth/google` | Public |
| GET | `/auth/google` | Redirect tới Google consent | Public |
| GET | `/auth/google/callback` | OAuth callback → upsert user + tạo session | Public |
| POST | `/dang-xuat` | Xoá session, redirect về `/` | Auth (POST-only để chống CSRF) |
| GET | `/ca-nhan` | Hồ sơ cá nhân + form chỉnh sửa + danh sách cấp bậc | Auth |
| POST | `/ca-nhan/cap-nhat` | Cập nhật hồ sơ | Auth |
| GET | `/cong-dong` | **[v0.6]** Trang chính Cộng Đồng (Lướt Nhóm / Lướt Chủ Đề) | Public |
| GET | `/cong-dong/tao-nhom` | **[v0.6]** Form tạo nhóm | Auth |
| POST | `/cong-dong/tao-nhom` | **[v0.6]** Tạo nhóm mới | Auth |
| GET | `/cong-dong/nhom/{slug}` | **[v0.6]** Trang nhóm + danh sách chủ đề + Live Chat panel | Public |
| POST | `/cong-dong/nhom/{slug}/tham-gia` | **[v0.6]** Tham gia nhóm | Auth |
| POST | `/cong-dong/nhom/{slug}/roi-khoi` | **[v0.6]** Rời nhóm (owner không được rời) | Auth |
| GET | `/cong-dong/nhom/{slug}/tao-chu-de` | **[v0.6]** Form tạo chủ đề | Auth + member |
| POST | `/cong-dong/nhom/{slug}/tao-chu-de` | **[v0.6]** Tạo chủ đề mới | Auth + member |
| GET | `/cong-dong/chu-de/{id}` | **[v0.6]** Trang chủ đề + bình luận | Public |
| POST | `/cong-dong/chu-de/{id}/binh-luan` | **[v0.6]** Đăng bình luận | Auth |
| **WS** | `/ws/cong-dong/nhom/{slug}` | **[v0.9.2]** Live Chat WebSocket (upgrade) | Auth + member |
| GET | `/api/cong-dong/nhom/{slug}/chat-history` | **[v0.9.2]** Lấy 50 tin nhắn gần nhất (`?limit=&before=`) | Public |
| GET | `/khong-gian` | Không Gian (placeholder) | Public |
| GET | `/cong-dong` | Cộng Đồng — Lướt Nhóm / Lướt Chủ Đề | Public |
| GET | `/ban-be` | Bạn Bè — Friend list, DM, Mail, Notifications | Public |
| GET | `/kinh-sach` | **[v0.9.6]** Trang chính Kinh Sách (Featured / Popular / Recent) | Public |
| GET | `/kinh-sach/tim-kiem` | **[v0.9.6]** Tìm kiếm sách `?q=` | Public |
| GET | `/kinh-sach/thu-vien/{category_slug}` | **[v0.9.6]** Lọc theo thư viện (phat-gia/dao-gia/kinh-van/sach-quy/quan-trong) | Public |
| GET | `/kinh-sach/{slug}` | **[v0.9.6]** Trang sách + danh sách chương + cảm ngộ | Public |
| GET | `/kinh-sach/{slug}/chuong/{chapter_slug}` | **[v0.9.6]** Đọc chương (sidebar mục lục) | Public |
| POST | `/kinh-sach/{slug}/cam-ngo` | **[v0.9.6]** Gửi cảm ngộ (min 100 chữ, chờ duyệt) | Auth |
| POST | `/kinh-sach/{slug}/tang-hoa` | **[v0.9.6]** Tặng hoa (1 user/sách) | Auth |
| GET | `/admin` | **[v0.9.7]** Trang Quản Trị (dashboard + stats) | Auth + admin |
| GET | `/admin/thanh-vien` | **[v0.9.7]** Danh sách thành viên + role | Auth + admin |
| POST | `/admin/thanh-vien/{user_id}/role` | **[v0.9.7]** Đổi role user (chỉ Admin Quản Lý) | Auth + admin_quan_li |
| GET | `/quy-tu-bi` | **[v0.9.11]** Trang Quỹ Từ Bi (dashboard + form + lists) | Public |
| POST | `/quy-tu-bi/dong-gop` | **[v0.9.11]** Đóng góp K vào quỹ | Auth |
| GET | `/api/quy-tu-bi/stats` | **[v0.9.11]** JSON tổng quan Quỹ Từ Bi | Public |
| GET | `/thuong-thanh` | Thương Thành (placeholder) | Public |
| GET | `/bang-xep-hang` | **[v0.9.10]** Trang Bảng Xếp Hạng (5 tabs) | Public |
| GET | `/tong-quan` | **[v0.9.14]** Trang Tổng Quan (User Hub) — 24 tính năng | Public |
| GET | `/cai-dat` | **[v0.9.14]** Trang Cài Đặt cá nhân | Auth |
| POST | `/cai-dat/cap-nhat` | **[v0.9.14]** Cập nhật cài đặt | Auth |
| GET | `/thanh-tich` | **[v0.9.14]** Trang Thành Tích cá nhân | Auth |
| GET | `/api/thanh-tich/stats` | **[v0.9.14]** JSON tổng quan thành tích | Auth |
| GET | `/tim-kiem` | **[v0.9.14]** Tìm kiếm toàn cục (?q=) | Public |
| GET | `/api/health` | Health check JSON + DB status | Public |
| POST | `/api/heartbeat` | Heartbeat giữ session | Auth |
| GET | `/api/upload-info` | Trả về giới hạn upload | Public |
| POST | `/api/upload-image` | Upload ảnh (5MB max, JPEG/PNG/WebP/GIF) | Auth |

## Phiên Bản

- **v0.1** — Giai đoạn 1: Nền móng hạ tầng cốt lõi
- **v0.2** — Giai đoạn 2: Hệ thống xác thực email/password
- **v0.3** — Giai đoạn 3: Chuyển sang Google OAuth (đăng nhập duy nhất bằng Google)
- **v0.4** — Giai đoạn 4: Hồ sơ thành viên & Hệ thống cấp bậc
- **v0.5** — Giai đoạn 5: Hạ tầng deploy (Docker + GitHub Actions + Coolify) + storage ảnh
- **v0.6** — Giai đoạn 6: Cộng Đồng Foundation (Nhóm + Chủ Đề + Bình luận)
- **v0.9** — Giai đoạn 9: Codebase sạch lỗi, clippy pedantic/nursery pass, Axum 0.8 ổn định
- **v0.9.1** — Fix UI mobile (bottom nav + x-cloak), bỏ GitHub Actions, deploy thủ công qua Coolify
- **v0.9.2** — Giai đoạn 7: Live Chat WebSocket trong Nhóm (Axum 0.8 ws + ChatHub broadcast + Alpine.js liveChat component)
- **v0.9.3** — Fix live chat (mpsc channel + tokio::select!), thêm Chat Chung toàn platform, avatar/group image upload, favicon
- **v0.9.4** — Giai đoạn 8: CI/CD tự động (GitHub Actions build & push Docker Image lên GHCR → Coolify auto pull & deploy)
- **v0.9.5** — Giai đoạn 9: Module Bạn Bè (Friends + DM 1-1 WebSocket + Mail + Notifications) + Fix live chat bugs (HttpOnly cookie check)
- **v0.9.6** — Giai đoạn 10: Kinh Sách (Thư viện kinh sách Phật giáo & Đạo giáo — 5 thư viện + Books + Chapters + Reviews + Flowers + Search)
- **v0.9.7** — Giai đoạn 11: Hệ thống vai trò Admin & Phân quyền cộng đồng (4 roles + admin panel + role display trên profile/header)
- **v0.9.8** — Giai đoạn 12: 50 quyền chi tiết + 3 giao diện admin riêng biệt (Admin Kỹ Thuật 50 quyền cao nhất + terminal/mod/executive dashboards)
- **v0.9.9** — Giai đoạn 13: Không Gian Cá Nhân (Niệm Phật Counter + Tượng Phật vows + Nhật Ký Tu Học + i_balance)
- **v0.9.10** — Giai đoạn 14: Bảng Xếp Hạng (5 tabs: A/I/K/Hôm Nay/Streak) + Safety schema fix cho login
- **v0.9.11** — Giai đoạn 15: Quỹ Từ Bi (đóng góp K + dashboard + 5 loại quỹ) + Fix Docker cache stale (CRITICAL deploy fix cho login)
- **v0.9.14** — Giai đoạn 18 + 19: Navigation Overhaul (User Hub + Mega Menu + Mobile Drawer + Settings) + 150 Quyền chi tiết + Hệ thống Thành Tích + Tìm Kiếm toàn cục
- **v0.9.16** — Giai đoạn 21: UI Redesign (header gọn, footer 6 cột, mega menu 4 cột) + Route Hub mở rộng (Health Check, 5 thư viện Kinh Sách, 5 BXH tabs, Admin quick links) + Code Quality (0 warnings, 0 clippy)
- **v0.9.18** — Giai đoạn 23: Mobile UI Overhaul (admin dashboards responsive) + Admin Nav Logic Fix (placeholder back button role-aware) + Logout/Profile state bug fix (mobile drawer shows correct auth state) + Quan-li tabs fix (Tổng quan & Nhóm tabs no longer 403)
- **v0.9.19** — Giai đoạn 24: Live Chat Cộng Đồng Fix (admin/mod bypass membership) + Hiệu ứng tin nhắn Admin/Mod (coder effect cho admin_ky_thuat, gold/blue/teal frame cho các role khác) + Chức vụ Mod mới (dưới admin, trên member, có quyền quản trị cơ bản) + author_role field trong chat messages
- **v0.9.20** — Giai đoạn 25: Live Chat Total Fix (6 root causes: WebSocket Ping/Pong keepalive 25s, idle timeout 180s, app-level ping/pong backup, client health check 60s, session heartbeat HttpOnly cookie fix, WS close code 1008 handling) + Sound Effects (Web Audio API, 4 sounds: send/receive/connect/error) + Animations (msg-slide-in, send-btn-pulse, conn-pulse, GPU-accelerated) + Live Chat panel phóng to (60→70dvh) + Message Queue + Optimistic UI + Performance (debounced scroll rAF, DOM cache, messages capped 200, max_message_size 64KB) + Folder reorganization (JS tách thành sound.js + chat.js + app.js, CSS tách thành app.css + chat.css) + Body data-logged-in attribute

---

*Nguyện công đức vô lượng. Nam Mô A Di Đà Phật.* 🪷
