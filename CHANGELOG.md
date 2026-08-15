# Changelog — Ứng Dụng Từ Bi

Tất cả thay đổi đáng chú ý của dự án sẽ được ghi lại trong file này.
Định dạng dựa trên [Keep a Changelog](https://keepachangelog.com/vi/1.1.0/),
tuân thủ [Semantic Versioning](https://semver.org/lang/vi/).

---

## [0.9.20] — 2026-08-15 — Giai đoạn 25: Live Chat Total Fix + Sound Effects + Animations + Performance

### Sửa lỗi (CRITICAL — Live Chat Total Fix)

Bug user report: "sửa lỗi không thể gửi tin nhắn trong live chat của cộng đồng, BẮT BUỘC PHẢI FIX TRIỆT ĐỂ, vì tôi đã fix mấy lần nhưng nó vẫn lỗi". Sau khi phân tích toàn bộ pipeline (server → proxy → client), tìm ra **6 root causes** chưa từng được fix:

- **[BF-1] WebSocket Ping/Pong keepalive thiếu hoàn toàn** — Server không bao giờ gửi `Message::Ping` cho client. Traefik proxy (cổng vào production) có idle timeout ~30-60s, đóng kết nối WebSocket idle mà client không phát hiện (TCP vẫn báo "alive"). User thấy "● online" nhưng thực sự kết nối đã chết → `socket.send()` thành công (browser buffer) nhưng server không bao giờ nhận → "không gửi được tin nhắn". **Fix**: Server (chat.rs + friends.rs) spawn `ping_interval` 25s trong `send_task` dùng `tokio::select!` — gửi `Message::Ping(Bytes::from_static(b"tubi"))` mỗi 25s. 25s < 30s Traefik timeout → kết nối sống mãi.
- **[BF-2] Idle timeout 180s — phát hiện dead connections** — Trước đây nếu proxy đóng kết nối âm thầm, server vẫn giữ `receiver.next()` block mãi, leak task + memory. **Fix**: `tokio::time::timeout(Duration::from_secs(180), receiver.next())` trong recv loop. 180s = 7 lần Ping không phản hồi → chắc chắn dead → break + cleanup.
- **[BF-3] App-level ping/pong backup (Text message)** — Một số proxy/firewall strip WebSocket control frames (Ping/Pong ở opcode 0x9/0xA). **Fix**: Client gửi Text message `{"type":"ping"}` mỗi 30s, server respond `{"type":"pong"}`. Dual-layer keepalive (protocol + app) đảm bảo works qua mọi proxy.
- **[BF-4] Client health check 60s — phát hiện dead connection từ phía client** — Client chỉ dựa vào `onclose` event, nhưng nếu TCP không báo close, client nghĩ kết nối vẫn sống. **Fix**: `_lastReceivedAt` timestamp + `_healthTimer` 15s check: nếu 60s không nhận gì (kể cả pong) → force `socket.close(4000)` + reconnect.
- **[BF-5] Session heartbeat KHÔNG BAO GIỜ chạy (HttpOnly cookie bug)** — `app.js` line 47 check `document.cookie.includes('session_id')` để start heartbeat, nhưng `session_id` cookie là HttpOnly (v0.9.5 đã ghi comment về điều này cho `globalChat()` nhưng không fix cho `DOMContentLoaded`). → `document.cookie` không đọc được session_id → check luôn return false → heartbeat không chạy → session hết hạn khi user đang active → WS upgrade fail 401 → "không gửi được tin nhắn" sau 1-2 giờ. **Fix**: thêm `<body data-logged-in="true|false">` attribute trong `layout.html`, `app.js` check `document.body.dataset.loggedIn === 'true'` thay vì cookie.
- **[BF-6] WebSocket close code 1008 handling sai** — Server trả HTTP 401/403 TRƯỚC khi WS upgrade (không phải WS close 1008). Frontend `if (event.code === 1008)` branch không bao giờ fire. → onerror + onclose(1006) → reconnect attempts fail lại (vẫn 401) → "Không thể kết nối sau 5 lần thử". **Fix**: Backend vẫn trả HTTP status (đúng), frontend detect via `onclose` code 1006 + `onerror` → handle riêng: nếu 401/403 thì KHÔNG reconnect (show error clear).

### Tính năng mới (Live Chat UX + Sound + Animation)

- **[FT-1] Sound Effects (Web Audio API — không cần file audio)**
  - Tạo module `src/static/js/sound.js` (160 dòng) — dùng Web Audio API tạo tones dynamically
  - `playSend()` — pop ngắn 880Hz → 1320Hz, 70ms, triangle wave (bright "ting" khi gửi)
  - `playReceive()` — chime nhẹ 660Hz, 100ms, sine wave (soft bell khi nhận)
  - `playConnect()` — bell 2 nốt C5+E5 (523Hz → 659Hz) khi WS kết nối thành công
  - `playError()` — buzzer 200Hz, 150ms, square wave khi có lỗi
  - Lazy init: AudioContext chỉ tạo khi user first interaction (browser autoplay policy)
  - Sound toggle: `<button data-sound-toggle>` trong chat header (cả live chat + global chat)
  - Preference persist trong `localStorage` key `tubi_sound`
  - Default: enabled

- **[FT-2] Animations mượt mà (CSS keyframes — GPU-accelerated)**
  - `@keyframes msg-slide-in` — message mới slide up 8px + fade in, 0.25s ease-out
  - `@keyframes msg-pop` — bubble pop scale 0.96 → 1.01 → 1
  - `@keyframes send-btn-pulse` — nút Gửi scale 1 → 0.92 → 1 khi click, 0.2s
  - `@keyframes conn-pulse` — connection indicator opacity 1 → 0.5 → 1, 2s infinite
  - Tất cả dùng `transform` + `opacity` (GPU-accelerated, không trigger layout)
  - `backface-visibility: hidden` để tránh flicker
  - `@media (prefers-reduced-motion: reduce)` — respect user preference, disable animations
  - CSS trong file mới `src/static/css/chat.css`

- **[FT-3] Live Chat panel PHÓNG TO**
  - Group chat: 60dvh → **70dvh**, max-height 480px → **640px**, min 280 → 320
  - Mobile: 50dvh → **58dvh**, max 540px
  - Global chat popup: 340px → **380px** wide, 60dvh → **65dvh**, max 480 → 580
  - Message area padding tăng 12px → 14-16px cho dễ đọc
  - User report: "làm màn hình live chat tổng to thêm một tí vì hiện tại nó quá bé" → fixed

- **[FT-4] Message Queue + Optimistic UI**
  - Khi disconnected, tin nhắn được queue trong `_queue: []`, flush khi reconnect
  - Không mất tin nhắn nữa — user có thể type nhiều tin khi mất mạng, tự gửi khi có mạng lại
  - `maxReconnectAttempts` tăng 5 → 10, backoff dày hơn (1s, 2s, 4s, 8s, 16s, 32s cap)

- **[FT-5] Reconnect logic cải thiện**
  - Reset `reconnectAttempts` khi nhận message thành công
  - Close code 1000 (normal) không reconnect
  - Close code 1008 (policy) không reconnect — show error clear
  - Backoff exponential: 1s, 2s, 4s, 8s, 16s, 32s (cap 30s)

### Cải thiện Performance (không lag, mượt hơn)

- **[PF-1] Debounced scroll bằng requestAnimationFrame**
  - `scrollToBottom()` wrap trong rAF + `_scrollPending` flag
  - Nhiều messages đến cùng lúc → chỉ 1 scroll call → giảm jank
- **[PF-2] Cache DOM refs**
  - `_messagesEl = this.$refs.messages` cache trong `init()`
  - Trước đây query DOM mỗi lần scroll → giờ query 1 lần
- **[PF-3] Messages array capped 200**
  - `_trimMessages()` splice old entries khi > 200
  - Tránh memory bloat khi chat lâu (8+ giờ)
- **[PF-4] WebSocket max_message_size 64KB**
  - `ws.max_message_size(64 * 1024)` — chống abuse (500 char chat + overhead << 64KB)
  - Axum default là 64MB → giảm memory footprint
- **[PF-5] Server handle Message::Pong, Message::Ping, Message::Close đúng cách**
  - Trước đây chỉ handle `Message::Text`, ignore các loại khác (có thể break)
  - Giờ: Pong → keepalive (continue), Ping → respond Pong (echo payload), Close → break, Binary → ignore
- **[PF-6] GPU acceleration cho animations**
  - `transform` + `opacity` only ( không trigger layout/paint)
  - `backface-visibility: hidden` + `-webkit-backface-visibility: hidden`
  - `will-change: opacity, transform` cho message containers

### Cải thiện cấu trúc thư mục (Folder Reorganization)

- **[RE-1] Tách JavaScript thành modules**
  - Trước: `src/static/js/app.js` 862 dòng (monolith)
  - Sau:
    - `src/static/js/sound.js` (160 dòng) — Sound effects module (Web Audio API)
    - `src/static/js/chat.js` (520 dòng) — Chat Alpine.js components (liveChat, globalChat, dmChat, chatBubble, notificationBadge)
    - `src/static/js/app.js` (130 dòng) — Main init, PrayerCounter, session heartbeat, utility functions
  - Load order trong `layout.html`: sound.js → chat.js → app.js
  - Shared helpers (`msgBubbleClass`, `avatarClass`, `roleBadgeHtml`, etc.) tách ra functions chung trong chat.js — không duplicate 3 lần như trước

- **[RE-2] Tách CSS thành modules**
  - Trước: `src/static/css/app.css` 619 dòng (monolith)
  - Sau:
    - `src/static/css/app.css` (619 dòng — giữ nguyên, không break existing styles)
    - `src/static/css/chat.css` (210 dòng — styles MỚI cho v0.9.20: enlarged chat, animations, sound toggle)
  - Load order: app.css → chat.css (chat.css override khi cần)

- **[RE-3] Body data attribute**
  - `<body data-logged-in="true|false">` — thay thế check `document.cookie.includes('session_id')` (không hoạt động với HttpOnly cookie)
  - dùng cho session heartbeat + future client-side auth checks

### Cải thiện Code Quality

- **[CQ-1] chat.rs refactor** — Tách `handle_ws_message` thành function riêng (giảm duplication), thêm `CtrlMessage` enum cho control channel (Error + Pong), xóa `err_tx` String channel thay bằng typed enum
- **[CQ-2] friends.rs DM handler đồng bộ** — Cùng pattern ping/pong/idle-timeout như chat.rs, dùng `DmCtrlMessage` enum riêng
- **[CQ-3] chat.js shared mixin** — `chatSocketMixin(getUrl)` chứa tất cả WebSocket logic chung (connect, send, ping, healthCheck, reconnect) — 3 components (liveChat, globalChat, dmChat) đều dùng chung, chỉ override `handleIncoming` và `init`
- **[CQ-4] HEALTH_FEATURES const** — Tách features array trong `health_check()` ra `const HEALTH_FEATURES: &[&str]` để tránh `serde_json::json!` recursion limit (array quá dài sau v0.9.20)
- **[CQ-5] Build verification** — `cargo check` + `cargo clippy` + `cargo build --release` pass sạch với Rust 1.97.1, 0 warnings

### Files Changed

**Mới:**
- `src/static/js/sound.js` (160 dòng)
- `src/static/js/chat.js` (520 dòng)
- `src/static/css/chat.css` (210 dòng)

**Sửa:**
- `src/handlers/chat.rs` — refactor hoàn toàn, thêm ping/pong/idle-timeout/CtrlMessage
- `src/handlers/friends.rs` — DM handler thêm ping/pong/idle-timeout/DmCtrlMessage
- `src/static/js/app.js` — rewrite, bỏ chat components (chuyển sang chat.js), fix session heartbeat
- `src/main.rs` — version → 0.9.20, tách HEALTH_FEATURES const, thêm v0.9.20 features
- `templates/layout.html` — load 3 JS files + 2 CSS files, `<body data-logged-in>`, sound toggle button, version string
- `templates/community/group.html` — bỏ inline .chat-panel style (chat.css xử lý), thêm send-btn + conn-indicator class, sound toggle button
- `templates/ban-be/conversation.html` — thêm send-btn class
- `Cargo.toml` — version 0.9.19 → 0.9.20
- `Dockerfile.coolify` — FROM tag 0.9.19 → 0.9.20

---

## [0.9.19] — 2026-08-15 — Giai đoạn 24: Live Chat Fix + Admin/Mod Message Effects + Mod Role

### Sửa lỗi (Critical Bug Fixes — user report)

- **[BF-1] Live Chat Cộng Đồng không gửi được tin nhắn (admin/mod)** — Bug user report: "không thể gửi tin nhắn trong live chat của cộng đồng". **Root cause**: WebSocket handler `chat_ws_upgrade` yêu cầu user phải là `active member` của nhóm (kiểm tra `group_members` table) — không có bypass cho admin/mod. Khi admin/mod vào một nhóm chưa tham gia, WebSocket upgrade fail với 403 Forbidden → client receive event.code 1008 → "Không có quyền chat" → không thể gửi tin nhắn. **Fix**: thêm `can_chat_any_group()` method cho User (return true cho admin + mod). Trong `chat_ws_upgrade`, nếu `user.can_chat_any_group()` là true thì bypass membership check. Frontend: template `community/group.html` hiển thị form chat cho admin/mod ngay cả khi chưa tham gia nhóm, và `isMember` flag trong Alpine.js `liveChat()` component = `membership.status == "active" || user.is_staff()`.
- **[BF-2] Logic sai trong template isMember** — Fix cú pháp Askama template: thay vì ghép 2 if-else (sẽ tạo "truefalse"), dùng `membership.as_ref().map_or(false, |m| m.status == "active") || user.as_ref().map_or(false, |u| u.is_staff())` — đúng OR logic.

### Tính năng mới (Features)

- **[FT-1] Chức vụ Mod (mod role)** — Thêm role mới "Mod" — dưới admin, trên thành viên, có quyền quản trị cơ bản:
  - Hierarchy mới: admin_ky_thuat (5) > admin_quan_li (4) > admin_cong_dong (3) > **mod (2)** > member (1)
  - **Mod có quyền**: xem `/admin` (redirect → `/admin/thanh-vien`), xem `/admin/thanh-vien`, xem `/admin/cong-dong/cam-ngo`, xem các trang placeholder, chat trong BẤT KỲ nhóm cộng đồng nào, hiển thị badge 📜 Mod
  - **Mod KHÔNG có quyền**: đổi role user khác, ban/activate user, truy cập 3 dashboard admin riêng
  - Migration 020: drop old `users_role_check` constraint + add new constraint cho phép 'mod'
  - DB safety check trong `db/mod.rs` cũng được cập nhật để đảm bảo 'mod' được chấp nhận idempotent

- **[FT-2] Hiệu ứng tin nhắn Admin/Mod trong chat** — Mỗi vai trò có style riêng cho tin nhắn:
  - **Admin Kỹ Thuật — Coder Effect**: Matrix Terminal — nền đen `#0a0e0a`, chữ xanh lá `#00ff41` phát sáng, font monospace, scan-line animation, border glow + box-shadow pulse, prefix `[SYS]`, avatar viền xanh lá pulse glow
  - **Admin Quản Lý — Premium Gold Frame**: khung vàng luxury — background gradient `#fffbeb → #fef3c7`, border 2px gold + border-left 4px amber, badge 👑, box-shadow vàng
  - **Admin Cộng Đồng — Shield Blue Frame**: khung xanh dương khiên — background gradient `#eff6ff → #dbeafe`, border 2px blue, badge 🛡️, box-shadow xanh dương
  - **Mod — Moderator Teal Frame**: khung teal — background gradient `#f0fdfa → #ccfbf1`, border 2px teal, badge 📜, box-shadow teal
  - Role badge mini cạnh tên author (⚙️ SYS / 👑 ADMIN / 🛡️ ADMIN / 📜 MOD)
  - Dark mode overrides cho từng role
  - Áp dụng cho cả 3 loại chat: Live Chat nhóm, Chat Chung toàn platform, DM 1-1
  - CSS trong `src/static/css/app.css` (thêm ~270 dòng styles)
  - Alpine.js helpers: `msgBubbleClass(role)`, `msgNameClass(role)`, `avatarClass(role)`, `roleBadgeHtml(role)`, `authorLabel(msg)`

### Cải thiện (Code Quality & Cleanups)

- **[CQ-1] User model refactor cho mod role**:
  - `is_admin()` giờ return true chỉ cho 3 role admin (KHÔNG bao gồm mod)
  - Thêm `is_mod()` method — true chỉ cho role 'mod'
  - Thêm `is_staff()` method — true cho admin HOẶC mod (dùng cho các quyền cơ bản)
  - Thêm `can_chat_any_group()` method — admin + mod được chat mọi nhóm
  - Cập nhật `role_level()`: mod=2, admin_cong_dong=3, admin_quan_li=4, admin_ky_thuat=5
  - Cập nhật `can_manage_technical()`: chỉ admin (level >=3), mod không có
  - Cập nhật `can_manage_community()`: mod trở lên (level >=2) — mod có quyền community
  - Cập nhật `can_manage_admin()`: chỉ admin_quan_li trở lên (level >=4)
  - `admin_dashboard_path()` cho mod = `/admin/thanh-vien` (không có dashboard riêng)
  - `role_display()`, `role_icon()`, `role_color()` thêm entry cho "mod"

- **[CQ-2] author_role field trong chat models**:
  - Thêm `author_role: Option<String>` vào `ChatMessageWithAuthor`, `GlobalChatMessageWithAuthor`, `DirectMessageWithAuthor`
  - SQL queries thêm `u.role AS author_role` cho chat history + DM history (cả 3 routes: group chat, global chat, DM)
  - WebSocket handlers (`handle_chat_socket`, `handle_global_chat_socket`, `handle_dm_socket`) lưu `author_role` khi persist message
  - `#[sqlx(default)]` để backward-compatible với DB cũ (nếu migration chưa chạy)

- **[CQ-3] Admin handler cập nhật cho mod**:
  - `admin_index` cho phép mod (is_staff) — redirect về `/admin/thanh-vien`
  - `admin_users_list` cho phép mod xem (is_staff)
  - `admin_change_role` thêm 'mod' vào danh sách role hợp lệ
  - `admin_change_role` thêm check: admin_cong_dong + mod không được đổi role
  - `fetch_users_list` ORDER BY thêm 'mod' ở vị trí 4 (sau admin_cong_dong)
  - `AdminUserRow::role_color_hint()` + `role_badge_html()` thêm entry cho 'mod'
  - `render_forbidden` hiển thị Mod trong hierarchy
  - 7 placeholder/preview handlers đổi `is_admin()` → `is_staff()` (cho mod xem)

- **[CQ-4] Templates cập nhật cho mod**:
  - `admin/users.html`: thêm option "📜 Mod" trong dropdown đổi role (teal background)
  - `admin/users.html`: cập nhật hierarchy text "KT (150/150) > QL (100/100) > CD (75/75) > Mod (15) > TV (0)"
  - `layout.html`: header admin link hiển thị cho mod (`is_staff()` thay vì `is_admin()`)
  - `layout.html`: mobile drawer admin link hiển thị cho mod
  - `layout.html`: footer version → v0.9.19
  - `community/group.html`: form chat hiển thị cho admin/mod ngay cả khi chưa tham gia nhóm
  - `community/group.html`: render hiệu ứng admin/mod (msgBubbleClass, msgNameClass, avatarClass, roleBadgeHtml, authorLabel)
  - `layout.html` global chat: render hiệu ứng admin/mod

- **[CQ-5] Version strings đồng bộ 0.9.19 ở mọi nơi**:
  - `Cargo.toml` — `version = "0.9.19"`
  - `src/main.rs` — 3 nơi (log info, health check JSON, phase_name)
  - `templates/layout.html` — footer
  - `templates/admin/users.html` — hierarchy text
  - `templates/admin/placeholder.html` — version badge
  - `templates/admin/cong-dong/index.html` — version badge
  - `templates/admin/cong-dong/cam-ngo.html` — version badge
  - `templates/admin/quan-li/index.html` — version badge
  - `templates/admin/ky-thuat/index.html` — version badge
  - `templates/khong-gian/index.html` — version badge
  - `src/handlers/admin.rs` — version string trong doc comments
  - `src/handlers/mod.rs` — version string trong doc comments
  - `src/handlers/chat.rs` — version string trong doc comments
  - `src/handlers/friends.rs` — version string trong doc comments
  - `Dockerfile.coolify` — FROM ghcr.io/mhieuhonda/tubi-app:0.9.19
  - `README.md` — version + changelog entry
  - `CHANGELOG.md` — entry mới

### Mục tiêu Giai đoạn 24
- Fix bug user report: "không thể gửi tin nhắn trong live chat của cộng đồng"
- Thêm hiệu ứng đặc biệt cho tin nhắn admin (tech admin = coder, admin khác = khung riêng)
- Thêm chức vụ "mod" — dưới admin, trên thành viên, có quyền quản trị cơ bản
- Quét và fix toàn bộ lỗi logic + UI liên quan

---

## [0.9.18] — 2026-08-15 — Giai đoạn 23: Mobile UI Overhaul + Admin Nav Logic Fix + Logout/Profile State Bug Fix

### Sửa lỗi (Critical Bug Fixes — user report)

- **[BF-1] Mobile drawer hiện nút "Thoát" khi đã đăng xuất** — Bug nghiêm trọng: form logout trong mobile drawer (3 gạch) được render UNCONDITIONAL, không kiểm tra `user.is_some()`. Kết quả: khách chưa login vẫn thấy nút "Thoát" → bấm vào → submit form `/dang-xuat` → server không có session → redirect về `/dang-nhap` (loop). **Fix**: wrap toàn bộ logout form trong `{% if let Some(_u) = user %}...{% else %}...{% endif %}` — khi chưa login thì hiển thị nút "Đăng Nhập Bằng Google" thay vì nút "Thoát".
- **[BF-2] Mobile drawer hiện "Hồ Sơ" link khi đã đăng xuất** — Link `/ca-nhan` được render cho mọi visitor. Click → redirect về `/dang-nhap` (loop). **Fix**: chỉ render "Hồ Sơ" khi `{% if let Some(_u) = user %}`.
- **[BF-3] Admin placeholder back button → 403 Forbidden** — Bug user report: "ấn vào quản lí [module] trên trang admin kỹ thuật rồi muốn quay lại nó lại hiện là 'quay về admin [role khác]' ấn vào lại báo không có quyền". **Root cause**: `user_admin_dashboard_back()` hardcode back_path/back_label theo "module owner" (ví dụ `/admin/cong-dong/nhom` → luôn back về `/admin/cong-dong`). Khi admin_ky_thuat click back → bị redirect tới `/admin/cong-dong` → `admin_cong_dong_dashboard` check `is_admin_cong_dong()` = FALSE → 403. **Fix**: `user_admin_dashboard_back(user)` giờ trả về `(user.admin_dashboard_path(), user.role_display())` — luôn trỏ về dashboard của CHÍNH user đang login. Áp dụng cho cả 4 placeholder handler (groups/books/comments/fund).
- **[BF-4] admin/quan-li tabs "Tổng quan" + "Nhóm & Cộng đồng" trỏ tới `/admin/cong-dong` → 403** — admin_quan_li click tab → bị redirect tới dashboard của admin_cong_dong → 403. **Fix**: tab "Tổng quan" giờ trỏ tới `/admin/quan-li` (dashboard của chính user), tab "Nhóm" trỏ tới `/admin/cong-dong/nhom` (placeholder, tất cả admin đều vào được). Tabs cũng được rút gọn label + thêm `overflow-x-auto` + `whitespace-nowrap` để scroll ngang trên mobile thay vì vỡ layout.
- **[BF-5] users.html back button hardcode "Về trang Quản Trị"** — Không role-aware, gây nhầm lẫn. **Fix**: `<a href="{{ u.admin_dashboard_path() }}">← Về {{ u.role_display() }}</a>` — text + URL đều theo role thực tế của user.

### Cải thiện (Mobile UI Overhaul — admin dashboards responsive)

- **[MU-1] admin/quan-li/index.html** — Header `h-16 px-8` → `h-14 sm:h-16 px-3 sm:px-6 lg:px-8`. Title `text-lg` → `text-sm sm:text-lg`. User info ẩn trên mobile (`hidden sm:flex`). "Về trang chủ" rút gọn thành "Home" trên mobile. Tabs `overflow-x-auto` + `whitespace-nowrap` + `scrollbar-thin`. Stats cards `gap-6` → `gap-3 sm:gap-6`, padding `p-6` → `p-4 sm:p-6`. Quick actions cards responsive `flex items-center gap-3 sm:gap-4` + `truncate`.
- **[MU-2] admin/cong-dong/index.html** — Header `px-6` → `px-3 sm:px-6`. Title `text-base` → `text-xs sm:text-base`. Role badge ẩn trên mobile (`hidden sm:inline-block`). Tabs `overflow-x-auto scrollbar-thin -mb-px` + `whitespace-nowrap` + `px-3 sm:px-4`. Stats cards responsive `gap-3 sm:gap-4`, icon `w-9 h-9 sm:w-10 sm:h-10`, số `text-xl sm:text-2xl`. Quick actions responsive.
- **[MU-3] admin/cong-dong/cam-ngo.html** — Header `px-6` → `px-3 sm:px-6`. Back button "← Dashboard" rút gọn "← DS" trên mobile. Title `text-base` → `text-xs sm:text-base`. "Về trang chủ" → "Home" trên mobile. Main `px-6 py-6` → `px-3 sm:px-6 py-4 sm:py-6`. Review cards `p-5` → `p-4 sm:p-5`. Buttons `px-4` → `px-3 sm:px-4`, `text-sm` → `text-xs sm:text-sm`. Footer version sync v0.9.18.
- **[MU-4] admin/users.html** — Section `px-4` → `px-3 sm:px-4`. H1 `text-2xl md:text-3xl` → `text-xl sm:text-2xl md:text-3xl`. Cards `p-4` → `p-3 sm:p-4`. Avatar `w-9 h-9` → `w-8 h-8 sm:w-9 sm:h-9`. Card text `text-base` → `text-sm sm:text-base`. Alerts/notification boxes responsive padding.
- **[MU-5] admin/placeholder.html** — Đã có mobile-first từ v0.9.17, chỉ cập nhật version string → v0.9.18.

### Đồng bộ version (Cleanup & Version Sync)

- **[CL-1] Version strings đồng bộ 0.9.18 ở mọi nơi**:
  - `Cargo.toml` — `version = "0.9.18"`
  - `src/main.rs` — 3 nơi (log line 45, log line 52, health_check JSON line 345)
  - `src/handlers/mod.rs` — placeholder footer line 579
  - `templates/layout.html` — footer line 414
  - `templates/admin/ky-thuat/index.html` — title + chip + stat card + footer (4 nơi)
  - `templates/admin/cong-dong/index.html` — footer
  - `templates/admin/quan-li/index.html` — footer
  - `templates/admin/cong-dong/cam-ngo.html` — footer
  - `templates/admin/placeholder.html` — title + banner + footer (3 nơi)
  - `templates/khong-gian/index.html` — footer line 259
  - `templates/admin/users.html` — version note line 159
  - `README.md` — version header + history line
  - `CHANGELOG.md` — new entry (this one)
- **[CL-2] Health check API** bổ sung 6 features mới trong mảng `features`: `mobile-ui-overhaul-v0.9.18`, `admin-placeholder-back-role-aware-v0.9.18`, `admin-quan-li-tabs-fix-v0.9.18`, `mobile-drawer-auth-state-fix-v0.9.18`, `admin-dashboards-responsive-v0.9.18`, `users-page-back-role-aware-v0.9.18`.
- **[CL-3] Historical comments `v0.9.17`** trong code (như `// v0.9.17: fix admin nav bug`) được GIỮ NGUYÊN — đây là documentation ghi lại khi nào feature được thêm, không phải version string cần bump.
- **[CL-4] Rust 1.97.1** — `rust-version = "1.97"` trong Cargo.toml, Dockerfile dùng `rust:1.97.1-slim-bookworm`.

### Mục tiêu đạt được

- ✅ Fix bug mobile drawer hiện nút Thoát + Hồ Sơ khi đã đăng xuất (user report)
- ✅ Fix bug admin placeholder back button → 403 Forbidden (user report)
- ✅ Fix bug admin/quan-li tabs trỏ sai dashboard → 403 Forbidden
- ✅ Fix bug users.html back button hardcode text
- ✅ Mobile UI overhaul cho 4 admin templates (quan-li, cong-dong, cam-ngo, users)
- ✅ Version sync 0.9.18 ở mọi nơi
- ✅ Health check API báo đúng 6 features mới của v0.9.18

---

## [0.9.17] — 2026-08-15 — Giai đoạn 22: Mobile-first Polish + Dark Mode + Admin Nav Fix

### Thay đổi (Dark Mode — chế độ sáng/tối)

- **[DM-1] Toggle button trong header** — icon 🌙 (chuyển sang tối) / ☀️ (chuyển sang sáng), đặt ở vị trí dễ thấy trên cả desktop và mobile.
- **[DM-2] Toggle trong mobile drawer** — nút riêng full-width dễ chạm, label "Chế độ tối" / "Chế độ sáng" thay đổi theo trạng thái hiện tại.
- **[DM-3] Anti-FOUC script** — apply `class="dark"` lên `<html>` TRƯỚC khi paint, tránh hiện tượng flash sáng/tối khi load trang. Script inline trong `<head>` đọc cookie `theme` trước, fallback localStorage, fallback tiếp `prefers-color-scheme`.
- **[DM-4] Cookie persistence** — `theme=lotus|dark` set với `max-age=1 năm`, `SameSite=Lax`, `http_only=false` (JS đọc được để fallback). Server có thể đọc cookie này để render đúng theme ngay từ server side (future improvement).
- **[DM-5] localStorage fallback** — khách chưa login vẫn nhớ theme preference qua `localStorage.tubi_theme`.
- **[DM-6] API endpoint `POST /api/theme`** — nhận form `{ theme: "lotus"|"dark"|"minimal" }`, upsert `user_settings.theme` trong DB (sync giữa các thiết bị), set cookie, trả về JSON `{ ok: true, theme }`.
- **[DM-7] Tailwind `darkMode: 'class'`** — config chính thức trong `tailwind.config` ở layout.html và placeholder.html.
- **[DM-8] CSS overrides cho dark mode** trong `app.css`:
  - Scrollbar (track, thumb, hover)
  - Chat popup (background, header, messages, input, message bubble)
  - Prayer ripple (màu sáng hơn cho visibility)
  - HTMX indicator
  - Chat bubble badge
  - Active nav tab
  - Selection color
- **[DM-9] Smooth transitions** — `transition: background-color, border-color, color` 150ms ease-out cho mọi element, nhưng giữ animation (pulse, float, glow, prayer-pulse, prayer-ripple) không bị transition chậm.
- **[DM-10] Admin placeholder page** (`templates/admin/placeholder.html`) có dark mode built-in đầy đủ.

### Thay đổi (Admin Nav Fix — bug user report)

- **[AN-1] Bug mô tả**: User report "tôi vào quản lí cộng đồng thì nó không vào phần quản lí mà nó lại vào phần Cộng đồng bình thường của user". Tương tự cho Quản lý Kinh Sách, Quản lý Bình luận, Quản lý Quỹ Từ Bi.
- **[AN-2] Root cause**: các nav tile trong 3 admin dashboard (ky-thuat, cong-dong, quan-li) trỏ tới USER pages (`/cong-dong`, `/kinh-sach`, `/quy-tu-bi`, `/bang-xep-hang`) — admin click vào rồi bị redirect ra khỏi admin context.
- **[AN-3] Fix — tạo 4 route admin placeholder mới**:
  - `GET /admin/cong-dong/nhom` — Quản lý Nhóm Cộng Đồng (read-only list 20 nhóm mới nhất + stats)
  - `GET /admin/kinh-sach` — Quản lý Kinh Sách (read-only list 20 sách mới nhất + stats)
  - `GET /admin/binh-luan` — Quản lý Bình luận (read-only list 20 comment mới nhất + stats)
  - `GET /admin/quy-tu-bi` — Quản lý Quỹ Từ Bi (read-only list 20 đóng góp mới nhất + stats)
- **[AN-4] Mỗi trang placeholder có**: header với icon + tên module + back button, stats grid 2x2 (mobile) / 4 (desktop), banner "Module đang được phát triển" với danh sách tính năng sắp ra (duyệt, ẩn, xóa, ghim, khoá, tìm kiếm, lọc), danh sách items read-only với link tới trang chi tiết, nút "Trở về [dashboard]".
- **[AN-5] Permission**: tất cả admin role (admin_ky_thuat, admin_quan_li, admin_cong_dong) đều có quyền xem placeholder pages. Module moderation đầy đủ sẽ ra mắt ở các phiên bản tiếp theo.
- **[AN-6] Template struct `AdminPlaceholderTemplate`** — shared template với 4 module_key (groups/books/comments/fund), conditional render theo module.
- **[AN-7] Helper functions**: `fetch_admin_groups_list`, `fetch_admin_books_list`, `fetch_admin_comments_list`, `fetch_admin_funds_list` — mỗi hàm fetch 20 items mới nhất với JOIN để lấy thông tin liên quan (member count, topic count, author name, topic title, v.v.).
- **[AN-8] Nav links cập nhật**:
  - `admin/ky-thuat/index.html` — 4 tiles (Quản lý nhóm, Kinh sách, Bình luận, Quỹ Từ Bi) trỏ tới admin pages thay vì user pages
  - `admin/cong-dong/index.html` — 2 tabs (Quản lý Nhóm, Kiểm duyệt) + 1 quick action card (Quản lý Nhóm) trỏ tới admin pages
  - `admin/quan-li/index.html` — 3 tabs (Nhóm & Cộng đồng, Kinh Sách, Báo cáo quỹ) + 2 quick action cards (Kinh Sách, Quỹ Từ Bi) trỏ tới admin pages; quick actions grid đổi từ 2 cột → 3 cột

### Thay đổi (Mobile-first Polish)

- **[MF-1] Bottom nav touch targets** — mỗi nút có `min-h-[44px]` (Apple HIG minimum tap target size).
- **[MF-2] Border cho nút giữa 🪷** — `dark:border-slate-800` để đúng contrast trong dark mode (trước đây `border-white` bị hòa với nền tối).
- **[MF-3] Mobile drawer dark mode** — tất cả 7 mục (Không Gian, Cộng Đồng, Bạn Bè, Kinh Sách, Hồ Sơ, Quản Trị nếu admin, Thoát) + theme toggle + nút đăng nhập Google có đầy đủ `dark:` variants.
- **[MF-4] Header dark mode** — `bg-white/95 dark:bg-slate-800 backdrop-blur border-b border-gray-200 dark:border-slate-700` cho header sticky.
- **[MF-5] Body dark mode** — `bg-paper dark:bg-slate-900 text-ink dark:text-slate-100` cho body chính.
- **[MF-6] Theme toggle button trên mobile** — đặt trong mobile drawer (vì header mobile đã có 3 gạch + logo, không còn chỗ cho nút toggle).
- **[MF-7] Theme toggle button trên desktop** — đặt trong header nav, cạnh nút search 🔎.

### Thay đổi (Cleanup & Version Sync)

- **[CL-1] Version strings đồng bộ 0.9.17 ở mọi nơi**:
  - `Cargo.toml` — `version = "0.9.17"`
  - `src/main.rs` — 3 nơi (log line 45, log line 52, health_check JSON line 335)
  - `templates/layout.html` — footer line 414
  - `templates/admin/ky-thuat/index.html` — 4 nơi (title, comment, chip, stat card, footer)
  - `templates/admin/cong-dong/index.html` — footer + permission count
  - `templates/admin/quan-li/index.html` — footer + permission count + dashboard subtitle
  - `templates/admin/placeholder.html` — title + footer (NEW file)
  - `templates/khong-gian/index.html` — footer line 259
  - `src/handlers/mod.rs` — placeholder page footer line 579
  - `README.md` — version header
  - `CHANGELOG.md` — new entry
- **[CL-2] Permission counts chính xác**: trước đây hiển thị hardcoded sai "4/20 quyền", "4/30 quyền", "100/100 quyền", "75/75 quyền". Giờ dùng `{{ u.permission_count() }}/{{ u.system_permission_count() }}` tự động theo role:
  - admin_ky_thuat: 150/150
  - admin_quan_li: 100/150
  - admin_cong_dong: 75/150
  - member: 0/0
- **[CL-3] Health check API** bổ sung 6 features mới trong mảng `features`: `admin-nav-fix-v0.9.17`, `dark-mode-toggle`, `theme-cookie-persistence`, `admin-groups-placeholder`, `admin-kinh-sach-placeholder`, `admin-binh-luan-placeholder`, `admin-quy-tu-bi-placeholder`, `mobile-first-touch-targets`.
- **[CL-4] `cargo check --release` sạch** — 0 errors, 0 warnings.
- **[CL-5] `cargo clippy --release` sạch** — 0 warnings.
- **[CL-6] Rust 1.97.1** — `rust-version = "1.97"` trong Cargo.toml, Dockerfile dùng `rust:1.97.1-slim-bookworm`, rustup toolchain `1.97.1` (8bab26f4f 2026-07-14).

### Routes mới (v0.9.17)

| Method | Path | Mô tả | Auth |
|--------|------|-------|------|
| GET | `/admin/cong-dong/nhom` | **[v0.9.17]** Quản lý Nhóm Cộng Đồng (placeholder, read-only list) | Auth + admin |
| GET | `/admin/kinh-sach` | **[v0.9.17]** Quản lý Kinh Sách (placeholder, read-only list) | Auth + admin |
| GET | `/admin/binh-luan` | **[v0.9.17]** Quản lý Bình luận (placeholder, read-only list) | Auth + admin |
| GET | `/admin/quy-tu-bi` | **[v0.9.17]** Quản lý Quỹ Từ Bi (placeholder, read-only list) | Auth + admin |
| POST | `/api/theme` | **[v0.9.17]** Cập nhật theme preference (lotus/dark/minimal) | Public (cookie only) hoặc Auth (DB sync) |

### Mục tiêu đạt được

- ✅ Fix bug admin nav → user pages (user report)
- ✅ Thêm dark/light mode với persistence (cookie + localStorage + DB)
- ✅ Mobile-first polish (touch targets, dark mode variants, smooth transitions)
- ✅ Version sync 0.9.17 ở mọi nơi
- ✅ Permission counts chính xác theo role
- ✅ cargo check + clippy sạch
- ✅ Rust 1.97.1 verified

---

## [0.9.16] — 2026-08-15 — Giai đoạn 21: UI Redesign + Route Hub + Polish

### Thay đổi (UI/UX — Redesign tổng thể)

- **[UI-1] Layout tổng thể redesign** — Header nhỏ gọn hơn (h-14 thay vì h-16), logo 🪷 + tên rút gọn "TỪ BI", bỏ subtitle dài dòng trên mobile. Background giấy (paper) nhẹ nhàng thay vì gray-50.
- **[UI-2] Mega menu desktop 4 cột** — Thay vì 3 cột (Hệ Thống / Cá Nhân / Khám Phá), giờ là 4 cột (Hệ Thống / Cá Nhân / Kinh Sách / Cộng Đồng) với đầy đủ 24 link — fix lỗi route mồ côi.
- **[UI-3] Footer 6 cột** — Footer cũ 5 cột, giờ 6 cột: Logo + 5 nhóm link (Chuyên Mục / Hệ Thống / Cá Nhân / Kinh Sách / Khám Phá). Mỗi cột 6 link → tổng 30+ route đều có link.
- **[UI-4] Home page redesign** — Hero compact, bỏ "Prayer Counter Demo" dài dòng. Thêm section "Khám Phá Thêm" với 12 card link nhanh tới tất cả tính năng (Quỹ, BXH, Thành Tích, Thương Thành, Tìm Kiếm, 5 thư viện Kinh Sách, Tạo Nhóm). Quote ngắn gọn "Tu cũng niệm Phật. Chơi cũng niệm Phật."
- **[UI-5] Trang /tong-quan redesign thành Hub đẹp** — Bỏ hero dài dòng, tổ chức theo 8 nhóm gọn gàng: Chuyên Mục (4 card) / Hệ Thống (6 card incl. Health Check) / Kinh Sách — 5 Thư Viện / Bảng Xếp Hạng — 5 Tabs / Cá Nhân (7 card) / Cộng Đồng (4 card) / Quản Trị (7 card nếu admin) / Liên Kết Nhanh (8 link).
- **[UI-6] Bottom nav giữ nguyên** — Theo yêu cầu user, không thay đổi icon/label của bottom nav mobile (Trang Chủ / Cộng Đồng / 🪷 Tổng Quan / Bạn Bè / Niệm Phật).
- **[UI-7] Mobile menu 3 gạch giữ nguyên 7 mục** — Theo yêu cầu user, không thêm mục nào vào hamburger menu. Các route khác truy cập qua: nút giữa 🪷 (Tổng Quan), mega menu desktop, footer desktop, trang /tong-quan, và trong các trang con (/ban-be, /ca-nhan, /kinh-sach, /cong-dong).

### Thêm (Features — Route Hub mở rộng)

- **[HUB-1] Health Check link** — `/api/health` giờ có link từ trang /tong-quan (icon 💓).
- **[HUB-2] 5 Thư Viện Kinh Sách** — `/kinh-sach/thu-vien/phat-gia`, `dao-gia`, `kinh-van`, `sach-quy`, `quan-trong` giờ đều có link từ /tong-quan, mega menu, footer.
- **[HUB-3] 5 BXH tabs** — `/bang-xep-hang?tab=a|today|streak|i|k` giờ đều có link từ /tong-quan, mega menu, footer.
- **[HUB-4] Admin Dashboard quick links** — /admin/ky-thuat, /admin/ky-thuat/nhat-ky, /admin/cong-dong, /admin/cong-dong/cam-ngo, /admin/quan-li, /admin/thanh-vien giờ có link từ /tong-quan (chỉ admin thấy).
- **[HUB-5] Cộng Đồng quick links** — Lướt Nhóm, Lướt Chủ Đề, Tạo Nhóm, Tạo Chủ Đề đều có link từ /tong-quan.

### Sửa lỗi (Bug Fixes — Code Quality)

- **[FIX-1] Bỏ 7 warnings Rust** — `cargo check` giờ sạch sẽ:
  - Bỏ `Redirect` import không dùng trong `tong_quan.rs`
  - Bỏ field `now: DateTime<Utc>` không dùng trong `thanh_tich.rs`
  - Bỏ field `total_features: u32` không dùng trong `tong_quan.rs` (v0.9.14 đã đếm hardcode 24, giờ không cần)
  - `#[allow(dead_code)]` cho `VowType::color()`, `DonationType::as_str()`, `FundDonation` struct, `FundSummary::total_k_in_system_label()` (model methods/structs để dùng cho API/UI tương lai)
- **[FIX-2] Version strings đồng bộ 0.9.16** — `Cargo.toml`, `main.rs` log, `/api/health` JSON, `Dockerfile.coolify`, footer template, `khong-gian/index.html` footer.
- **[FIX-3] Health check `phase` và `phase_name`** — Updated `phase: 21`, `phase_name: "Giai đoạn 21 — UI Redesign + Route Hub + Polish"`. Thêm 5 feature mới vào `features` list.

### Bảo trì (Maintenance)

- **[MAINT-1] Cargo.toml version** — `0.9.15` → `0.9.16`.
- **[MAINT-2] Dockerfile.coolify** — `FROM ghcr.io/mhieuhonda/tubi-app:0.9.15` → `:0.9.16`.
- **[MAINT-3] main.rs version string** — `v0.9.15` → `v0.9.16`.
- **[MAINT-4] Layout footer version** — `v0.9.15` → `v0.9.16`.
- **[MAINT-5] khong-gian footer version** — `v0.9.15` → `v0.9.16`.

### Giai đoạn

Giai đoạn 21 — UI Redesign + Route Hub + Polish. Tiếp theo sau Giai đoạn 20 (v0.9.15) — Niệm Phật Fix + Admin Redesign + Mobile UX.

**Triết lý redesign:** Ít chữ hơn, nhiều icon hơn, cards nhỏ gọn, màu sắc mềm mại (paper background, tubi-50/100 accents, lotus-50/100 cho highlight). Tất cả route đều có UI truy cập qua /tong-quan hoặc mega menu hoặc footer — không còn "route mồ côi".

---

## [0.9.15] — 2026-08-15 — Giai đoạn 20: Niệm Phật Fix + Admin Redesign + Mobile UX

### Sửa (Bug Fixes — Critical)

- **[FIX-1] Niệm Phật counter không bị lệch trái sau click** — HTMX response bị mất class `text-center mb-4` (chỉ có `hx-target="this" hx-swap="outerHTML"`), khiến số `0` ban đầu ở giữa nhưng sau khi niệm +1 thì bị lệch sang trái. v0.9.15 giữ nguyên class `text-center mb-4` trong response.
- **[FIX-2] Streak (số ngày tu liên tiếp) tính đúng** — `compute_streak()` dùng `chrono::Local::now().date_naive()` nhưng DB `CURRENT_DATE` trong Docker container TZ=UTC → mismatch timezone. Nếu user niệm lúc 23:00 UTC (06:00 hôm sau VN), record được lưu theo UTC nhưng local time lại là ngày hôm sau → `days_diff > 1` → streak = 0 dù user có niệm. Fix: dùng `Utc::now().date_naive()` đồng bộ với DB.
- **[FIX-3] Tổng niệm / niệm hôm nay cập nhật ngay lập tức** — HTMX response trước đây chỉ swap `#niem-counter`, không cập nhật stats card (`#stat-today-niem`, `#stat-total-niem`, `#stat-streak`, `#stat-k-balance`). User phải F5 mới thấy cập nhật. v0.9.15 dùng `hx-swap-oob="outerHTML"` để swap nhiều element cùng lúc — counter + 4 stats card + footer đều cập nhật ngay sau khi niệm.
- **[FIX-4] Form Cầu Nguyện / Sám Hối / Hồi Hướng gửi được** — Bug v0.9.14: form có `hx-post=""` (rỗng) → HTMX intercept submit và POST đến URL rỗng → không gửi được. v0.9.15 tách thành 3 form riêng biệt với `hx-post` URL cố định (`/tuong-phat/cau-nguyen`, `/tuong-phat/sam-hoi`, `/tuong-phat/hoi-huong`), Alpine `x-show` toggle hiển thị form tương ứng.
- **[FIX-5] `practice_logs` upsert không còn nuốt error** — Code cũ `let _ = sqlx::query(...)` bỏ qua kết quả → nếu upsert fail (vd. table không tồn tại), `a_balance` vẫn tăng nhưng `practice_logs` rỗng → `today_niem` luôn = 0, streak không đếm. v0.9.15 log error + rollback nếu fail.
- **[FIX-6] `create_vow` cũng thêm log error + rollback** thay vì `let _ =` cho insert vow.

### Thay đổi (UI/UX — Admin Redesign)

- **[ADMIN-1] Bảng quản trị Admin Kỹ Thuật redesign theo ảnh tham chiếu** — Dark theme, mobile-first, layout 2 cột:
  - **Stats grid 2×4**: 4 cards (Người dùng, Nhóm, Sách, Bình luận) với số liệu lớn màu đỏ coral (`#ff6b6b`) — đúng theo ảnh.
  - **Nav tiles grid 2×8**: 15 tiles điều hành (Hướng dẫn, Phê duyệt, Thành viên, Nhóm, Kinh sách, Báo cáo, Bình luận, Từ vựng cấm, Nội dung đánh dấu, Quản lý tag, VIP, Quỹ Từ Bi, Bảng xếp hạng, Nhật ký, Health check, Thành tích) — mỗi tile có icon + label + count (nếu có).
  - **Permission matrix 10 nhóm × 10 quyền** — hiển thị đầy đủ 150 quyền chia 10 nhóm (system, users, content, community, kinh_sach, fund, achievements, security, media, analytics).
- **[ADMIN-2] Hiển thị đúng 150 quyền** — Fix hardcoded "6/50 quyền" trong template → dùng `{{ u.permission_count() }}/{{ u.system_permission_count() }}` → admin_ky_thuat thấy `150/150`, admin_quan_li thấy `100/100`, admin_cong_dong thấy `75/75`.
- **[ADMIN-3] `permission_count()` đồng bộ với `system_permission_count()`** — Trước đây `permission_count` trả về 18/12/9 (chỉ đếm UI visible), giờ trả về 150/100/75 (đồng bộ với potential permissions) vì UI đã có đủ nav tiles cho tất cả 10 nhóm.

### Thay đổi (UI/UX — Navigation Overhaul)

- **[NAV-1] Menu 3 gạch rút gọn chỉ còn 7 mục** (theo yêu cầu user): Không Gian, Cộng Đồng, Bạn Bè, Kinh Sách, Hồ Sơ, Quản Trị (nếu admin), Thoát. Các mục khác được phân bổ:
  - **Tổng Quan, Quỹ Từ Bi, Bảng Xếp Hạng, Thành Tích, Thương Thành, Tìm Kiếm** → truy cập qua nút giữa 🪷 (Tổng Quan) trên bottom nav, hoặc qua trang `/tong-quan`.
  - **Cài Đặt, Tin Nhắn, Hộp Thư, Thông Báo, Tìm Bạn** → trong trang `/ban-be` và `/ca-nhan`.
  - **Kinh Phật, Kinh Đạo, Tìm Sách, Tạo Nhóm** → trong trang `/kinh-sach` và `/cong-dong`.
- **[NAV-2] Bottom nav đổi icon** — Trang Chủ: 🪷 → 🏠 ngôi nhà; nút giữa: 🧭 → 🪷 hoa sen (Tổng Quan).
- **[NAV-3] Mobile text overflow fix** — Thêm `truncate`, `break-words`, `whitespace-nowrap` cho các button/card/text dễ bị tràn trên mobile.
- **[NAV-4] Vow card header** — Thêm `min-w-0` + `truncate` cho author_name + `whitespace-nowrap` cho timestamp để không bị tràn trên mobile.

### Bảo trì (Maintenance)

- **[MAINT-1] Cargo.toml version** — `0.9.14` → `0.9.15`.
- **[MAINT-2] Dockerfile.coolify** — `FROM ghcr.io/mhieuhonda/tubi-app:0.9.14` → `:0.9.15` (tránh Docker cache stale digest).
- **[MAINT-3] main.rs version string** — `v0.9.14` → `v0.9.15` ở log khởi động + health check JSON.
- **[MAINT-4] Health check `permission_counts`** — Cập nhật `{admin_ky_thuat: 150, admin_quan_li: 100, admin_cong_dong: 75}` (trước đây là 18/12/9).

---

## [0.9.14] — 2026-08-15 — Giai đoạn 18 + 19: Navigation Overhaul + 150 Quyền + Thành Tích

### Thêm (Features — Giai đoạn 18: Navigation Overhaul)

- **[NAV-1] Trang Tổng Quan (User Hub) `/tong-quan`** — Trung tâm điều hướng liệt kê TẤT CẢ 24 tính năng của app trong 4 nhóm (Chuyên Mục / Hệ Thống / Cá Nhân / Liên Kết Nhanh). Fix lỗi route mồ côi — user không cần biết URL cũng truy cập được mọi trang.
- **[NAV-2] Mega Menu Desktop** — Thêm dropdown "🧭 Khám Phá" trên header desktop, chia 3 cột (Hệ Thống / Cá Nhân / Khám Phá) với 18 link. Hover là mở, click outside là đóng.
- **[NAV-3] Mobile Drawer đầy đủ** — Mobile menu cũ chỉ có 4 chuyên mục + Hồ sơ + Thoát. Bây giờ có 6 nhóm (Chuyên Mục / Hệ Thống / Cá Nhân / Khám Phá / Tìm Kiếm / Quản Trị) với 22 link. Scroll được, không bị cắt.
- **[NAV-4] Bottom Nav redesign** — Đổi icon giữa từ 🪷 (Trang Chủ) sang 🧭 (Tổng Quan) để phản ánh đúng vai trò hub trung tâm. Tab "Không Gian" đổi sang "🙏 Niệm Phật" cho rõ nghĩa.
- **[NAV-5] Footer mở rộng 5 cột** — Footer cũ 4 cột, giờ 5 cột: Giới Thiệu + Chuyên Mục + Hệ Thống + Cá Nhân + Khám Phá. Mọi route đều có link từ footer.
- **[NAV-6] Search icon trên header** — Icon 🔎 dẫn nhanh đến `/tim-kiem` cho user đã đăng nhập.
- **[SET-1] Trang Cài Đặt `/cai-dat`** — User settings page với 4 nhóm: Riêng Tư (profile_visibility, show_balance/activity/email), Thông Báo (5 toggle), Giao Diện (theme lotus/dark/minimal, language vi/en/zh), Chat & Niệm Phật (4 toggle).
- **[SET-2] Migration 017** — Bảng `user_settings` với 16 cột + trigger `updated_at` + seed default cho user hiện có.

### Thêm (Features — Giai đoạn 19: 150 Quyền + Thành Tích + Tìm Kiếm)

- **[PERM-1] Mở rộng 50 → 150 quyền chi tiết** — Thêm 100 quyền mới chia 10 nhóm × 10 quyền:
  - `fund` (10) — Quản lý Quỹ Từ Bi (xem tất cả, duyệt, tạo chiến dịch, chi tiêu, refund, ...)
  - `achievements` (10) — Quản lý Thành Tích (tạo, sửa, xóa, grant, revoke, ...)
  - `security` (10) — Bảo mật (audit log, login log, IP blocklist, 2FA, CAPTCHA, ...)
  - `navigation` (10) — Quản lý UI (menu, footer, themes, announcement, feature flags, ...)
  - `analytics` (10) — Phân tích (dashboard, user stats, revenue, funnel, cohort, ...)
  - `media` (10) — Quản lý upload (xem tất cả, xóa, approve, quota, compress, ...)
  - `friends` (10) — Quản lý Bạn Bè/DM (xem tất cả, mute, blocklist, force unfriend, ...)
  - `mail` (10) — Quản lý Thư/Thông báo (broadcast, template, queue, filter, ...)
  - `events` (10) — Quản lý Sự kiện (tạo, sửa, xóa, attendance, recording, ...)
  - `shop` (10) — Quản lý Thương Thành (sản phẩm, đơn hàng, refund, danh mục, ...)
- **[PERM-2] Phân bổ mới**:
  - `admin_ky_thuat`: 150 quyền (TẤT CẢ — toàn quyền hệ thống)
  - `admin_quan_li`: 100 quyền (30 cũ + 70 mới)
  - `admin_cong_dong`: 75 quyền (20 cũ + 55 mới)
  - `member`: 0 quyền admin
- **[PERM-3] Migration 018** — INSERT 100 quyền mới + INSERT role_permissions (150+100+75 = 325 row). Idempotent với ON CONFLICT DO NOTHING.
- **[ACH-1] Hệ thống Thành Tích** — 30 thành tích mẫu chia 6 nhóm (Niệm Phật, Tượng Phật, Cộng Đồng, Kinh Sách, Bạn Bè, Quỹ Từ Bi) với 5 độ hiếm (common → mythic).
- **[ACH-2] Migration 019** — 3 bảng mới (`achievements`, `user_achievements`, `achievement_progress`) + 2 view (`v_user_achievements`, `v_user_achievement_progress`) + function `check_and_grant_achievement()` + trigger `updated_at`.
- **[ACH-3] Trang `/thanh-tich`** — Hiển thị thành tích đã đạt (cards với rarity badge + reward) + thành tích đang tiến hành (progress bar) + 5 stats tổng quan (đã đạt / điểm / A / I / K).
- **[ACH-4] API `/api/thanh-tich/stats`** — JSON tổng quan cho dashboard tích hợp.
- **[SRCH-1] Trang Tìm Kiếm toàn cục `/tim-kiem`** — Search đồng thời 4 loại: users (tên + pháp danh + email), books (title + author + description), topics (title + body), groups (name + description). Tối đa 10 kết quả mỗi loại, ILIKE pattern, order by view_count/friend_count.
- **[USR-1] User model mở rộng** — `has_permission_code` hỗ trợ 150 mã quyền, `permission_count` (UI features: 18/12/9/0), `system_permission_count` (150/100/75/0).

### Sửa lỗi (Bug Fixes)

- **[FIX-1] Lỗi thiếu giao diện cho route mồ côi** — `/bang-xep-hang`, `/quy-tu-bi`, `/thuong-thanh`, `/ban-be/tin-nhan`, `/ban-be/thu`, `/ban-be/thong-bao`, `/ban-be/tim-kiem`, `/kinh-sach/tim-kiem`, `/cong-dong/tao-nhom` giờ đều có link từ mega menu (desktop) + mobile drawer + footer + trang Tổng Quan.
- **[FIX-2] Mobile bottom nav khó hiểu** — Icon giữa 🪷 không rõ nghĩa, đổi sang 🧭 (Tổng Quan). Tab cuối đổi từ "Không Gian" sang "🙏 Niệm Phật" cho rõ hơn.
- **[FIX-3] Footer thiếu nhiều route** — Footer cũ chỉ có 3 route Hệ Thống (Quỹ Từ Bi, Thương Thành, Bảng Xếp Hạng). Bây giờ có 5 cột với 22 link.
- **[FIX-4] Health check version mismatch** — Cập nhật `version`, `phase`, `phase_name`, `permission_counts`, `system_permission_counts`, `features` list trong `/api/health`.

### Giai đoạn

Giai đoạn 18 — Navigation Overhaul + User Hub + Settings
Giai đoạn 19 — 150 Quyền + Thành Tích + Tìm Kiếm toàn cục

---

## [0.9.13] — 2025-08-15 — Giai đoạn 17: Admin UI Compact + Bug Fixes + Audit Log

### Sửa lỗi
- Fix giao diện bảng quản trị admin: bớt chữ, sắp xếp gọn gàng, không tràn nút
- Fix gửi kết bạn chuyển hướng trang trắng → dùng HTMX inline
- Fix đồng bộ A/K: tự động chuyển 1000 A = 1 K khi niệm Phật
- Xóa mọi liên quan đến game siêu độ trong codebase
- Fix số quyền hiển thị cho chuẩn với giao diện thực tế

### Thêm mới
- Nhật ký hoạt động (audit log) cho Admin Kỹ Thuật
- HTMX inline cho kết bạn, chấp nhận, từ chối
- Auto-convert A→K khi niệm Phật
- Content moderation UI (duyệt cảm ngộ)
- User ban/activate endpoints

### Giai đoạn
Giai đoạn 17 — Admin UI Compact + Audit Log + Bug Fixes

---

## [0.9.12] — 2026-08-15 — Giai đoạn 16: Mobile UX + Admin Kỹ Thuật Redesign + Security Hardening

### Thêm (Features — Giai đoạn 16)

- **[FEAT-1] Redesign toàn diện giao diện Admin Kỹ Thuật** (`/admin/ky-thuat`)
  - Phong cách coder hiện đại: bảng màu slate-900 + emerald accent (không phải neon terminal)
  - Font Inter (sans-serif) cho UI + JetBrains Mono cho code/technical — dễ đọc
  - Mobile-first responsive: hamburger toggle sidebar, `text-sm md:text-base`, grid 2 cột mobile / 4 cột desktop
  - Tiếng Việt phổ thông 100%: "Tổng quan", "Tình trạng hệ thống", "Quản lý thành viên", "Phân quyền chi tiết", "Cơ sở dữ liệu", "Nhật ký hoạt động", "Hành động nhanh"
  - Sidebar dùng anchor links (single-page) — không cần 5 route riêng, không 404
  - Bảng phân quyền dạng card accordion trên mobile, table đầy đủ trên desktop
  - 50 quyền dịch sang tiếng Việt + mô tả ngắn gọn, chia 5 nhóm màu (Hệ thống/Thành viên/Nội dung/Cộng đồng/Kinh Sách)
  - Bỏ hoàn toàn: shell prompts (`root@tubi:~#`), `cat /proc/...`, fake `.bash_history`, blinking cursor `█`, scanlines, matrix background, `[EXIT]`, `UPTIME` counter, `whoami`, `env` block

- **[FEAT-2] Sidebar mobile collapsible** — Admin Kỹ Thuật có thể dùng trên điện thoại không cần zoom
  - Overlay backdrop khi sidebar mở (mobile)
  - Smooth transform animation
  - `scroll-padding-top` cho anchor link không bị header che

- **[FEAT-3] Route mới**: `GET /admin/ky-thuat/users` — redirect sang `/admin/thanh-vien` (fix 404)

### Sửa (Bug Fixes — CRITICAL: Live Chat mobile keyboard)

- **[FIX-1] CRITICAL: Bàn phím ảo liên tục đóng khi nhập tin nhắn trên mobile**
  - Nguyên nhân: `:disabled="!connected"` reactive binding trên `<input>` — mỗi lần WebSocket reconnect (thường xuyên trên mobile), Alpine toggle `disabled` attribute → mobile OS dismiss bàn phím
  - Fix: bỏ `:disabled="!connected"` khỏi `<input>` ở 3 nơi (group live chat, DM conversation, Chat Chung popup). Submit button vẫn giữ `:disabled` để disable khi không kết nối. `send()` đã có guard `if (!this.connected) return error`.
  - Thêm `enterkeyhint="send"` + `inputmode="text"` + `autocomplete="off"` + `autocapitalize="sentences"` cho UX mobile tốt hơn

- **[FIX-2] Viewport meta thêm `interactive-widget=resizes-content` + `viewport-fit=cover`**
  - Android: layout viewport giờ co lại đúng khi bàn phím ảo mở → input không bị che
  - iOS: hỗ trợ notch / safe area

- **[FIX-3] Chat panel heights đổi từ `vh`/fixed-px sang `dvh` (dynamic viewport height)**
  - `.chat-panel`: `60dvh` (desktop) / `50dvh` (mobile) với min/max-height
  - `.dm-panel`: `calc(100dvh - 200px)` (desktop) / `calc(100dvh - 160px)` (mobile)
  - `.chat-chung-popup`: `60dvh` max `480px` (desktop) / `55dvh` max `60dvh` (mobile)
  - `dvh` tự co lại khi bàn phím mở — input luôn nhìn thấy được

### Sửa (Bug Fixes — Security: Stored XSS)

- **[FIX-4] CRITICAL: Stored XSS qua `other_display_name` trong DM conversation** (`templates/ban-be/conversation.html`)
  - Nguyên nhân: inject trực tiếp `{{ other_display_name }}` vào `x-data="dmChat({...})"` — Askama HTML-escape `"` thành `&#34;`, browser decode lại thành `"` → break JS string → arbitrary code execution
  - Attack vector: user đặt `display_name` thành payload JS → admin mở DM với user đó → JS chạy trong session admin → có thể tự promote lên admin_ky_thuat qua POST `/admin/thanh-vien/{id}/role`
  - Fix: thêm field `init_json: String` vào `ConversationTemplate`, serialize toàn bộ init object bằng `serde_json::to_string` (escape đúng JS string context), render `x-data="dmChat({{ init_json|safe }})"`

- **[FIX-5] CRITICAL: Stored XSS qua `f.other_display_name` trong friends list** (`templates/ban-be/index.html`)
  - Nguyên nhân: `onsubmit="return confirm('...{{ f.other_display_name }}?');"` — cùng vấn đề escape
  - Fix: bỏ user-controlled data khỏi confirm message, dùng generic "Bạn có chắc muốn hủy kết bạn?"

### Sửa (Bug Fixes — Security: Permission gaps)

- **[FIX-6] HIGH: `mail_send` cho phép gửi thư cho bất kỳ user nào (không cần kết bạn)**
  - Nguyên nhân: handler chỉ check `recipient_id != user.id` + validate subject/body, không check `friendships` table
  - Risk: spam vector chéo toàn userbase — attacker craft POST với bất kỳ `recipient_id`
  - Fix: thêm query `SELECT EXISTS(... friendships ... status='accepted')` — reject nếu không phải bạn bè

- **[FIX-7] HIGH: `create_comment` không validate `parent_id` thuộc cùng topic**
  - Nguyên nhân: `parent_id` parse thành Uuid và bind thẳng vào INSERT, không check parent comment có `topic_id` khớp
  - Risk: cross-topic reply — comment của topic A xuất hiện làm reply của comment ở topic B
  - Fix: nếu `parent_id` được cung cấp, query `SELECT EXISTS(... comments WHERE id=$1 AND topic_id=$2 ...)` — reject nếu không khớp

### Sửa (Bug Fixes — 404s)

- **[FIX-8] Báo 404 khi click "Quản lý thành viên" trong sidebar Admin Kỹ Thuật**
  - Nguyên nhân: sidebar link `href="/admin/ky-thuat/users"` nhưng route không tồn tại (route thật là `/admin/thanh-vien`)
  - Fix: thêm route `GET /admin/ky-thuat/users` → handler `admin_ky_thuat_users_redirect` → redirect sang `/admin/thanh-vien`

- **[FIX-9] Báo 404 khi click các tab trong Admin Quản Lý** (`/admin/quan-li/nhom`, `/admin/quan-li/kinh-sach`, `/admin/quan-li/bao-cao`)
  - Fix: đổi link sang route có thật — `/cong-dong`, `/kinh-sach`, `/quy-tu-bi`

- **[FIX-10] Báo 404 khi click các tab trong Admin Cộng Đồng** (`/admin/cong-dong/nhom`, `/admin/cong-dong/noi-dung`, `/admin/cong-dong/cam-ngo`, `/admin/cong-dong/thanh-vien`)
  - Fix: đổi link sang route có thật — `/cong-dong`, `/kinh-sach`, `/admin/thanh-vien`

### Sửa (Bug Fixes — Code quality)

- **[FIX-11] Clippy: `return Err(...)` dư trong `auth.rs:577`** — bỏ `return`, trả giá trị trực tiếp
- **[FIX-12] Clippy: collapsed `if` trong `quy_tu_bi.rs:175`** — gộp 2 if lồng nhau thành `if let ... && ...`

### Routes mới (v0.9.12)

| Method | Path | Mô tả | Auth |
|--------|------|-------|------|
| GET | `/admin/ky-thuat/users` | Redirect → `/admin/thanh-vien` (fix 404) | Admin Kỹ Thuật |

### Đổi (Refactors)

- **[REF-1] `templates/admin/ky-thuat/index.html`** — rewrite hoàn toàn (~550 dòng), bỏ terminal aesthetic, đổi sang modern dark dashboard
- **[REF-2] `templates/admin/quan-li/index.html`** — fix 3 dead links, bump version
- **[REF-3] `templates/admin/cong-dong/index.html`** — fix 4 dead links, add footer, bump version
- **[REF-4] `src/handlers/admin.rs`** — thêm `admin_ky_thuat_users_redirect` handler
- **[REF-5] `src/handlers/friends.rs::ConversationTemplate`** — thêm field `init_json` để escape Alpine.js init object
- **[REF-6] `src/handlers/friends.rs::mail_send`** — thêm friendship check
- **[REF-7] `src/handlers/community.rs::create_comment`** — thêm parent_id topic validation

### Cập Nhật Tài Liệu

- `Cargo.toml` — version `0.9.11` → `0.9.12`
- `src/main.rs` — log khởi động v0.9.12, phase 16, health check JSON, thêm 4 features flags
- `templates/layout.html` — footer v0.9.12, viewport meta `interactive-widget=resizes-content`, bỏ `:disabled` khỏi chat chung input
- `templates/community/group.html` — `.chat-panel` dùng `dvh`, bỏ `:disabled` khỏi live chat input
- `templates/ban-be/conversation.html` — `.dm-panel` dùng `dvh`, bỏ `:disabled` khỏi DM input, dùng `init_json`
- `templates/admin/users.html` — version note v0.9.12
- `templates/khong-gian/index.html` — footer v0.9.12
- `templates/bang-xep-hang/index.html` — footer v0.9.12
- `templates/quy-tu-bi/index.html` — footer v0.9.12
- `src/static/css/app.css` — `.chat-chung-popup` dùng `dvh` với min/max-height
- `Dockerfile.coolify` — `FROM :0.9.11` → `FROM :0.9.12`

### Build & Test

- ✅ `cargo check --release` — pass (3 pre-existing dead_code warnings, không phải lỗi)
- ✅ `cargo clippy --release` — pass (sau khi fix 2 style issues)
- ✅ Rust 1.97.1 toolchain xác nhận
- ✅ Tất cả templates Askama render thành công (compile-time checked)

---

## [0.9.11] — 2026-08-14 — Giai đoạn 15: Quỹ Từ Bi & Fix lỗi đăng nhập triệt để

### Thêm (Features — Giai đoạn 15: Quỹ Từ Bi)

- **[FEAT-1] Chuyên mục Quỹ Từ Bi chính thức ra mắt** (`/quy-tu-bi`)
  - Bỏ placeholder "Giai đoạn 10", thay bằng trang quỹ cộng đồng đầy đủ
  - Theo `HieuLouis/Hệ Thống Và Chức Năng Chi Tiết.docx` mục VI

- **[FEAT-2] Hệ thống đóng góp K vào Quỹ Từ Bi**
  - `POST /quy-tu-bi/dong-gop` — form đóng góp K (trừ từ `k_balance` của user)
  - 5 loại quỹ: 🪷 Quỹ Chung · 📚 Quỹ Sách · 🕉️ Quỹ Tu · 🎁 Quỹ Quà · 🤝 Quỹ Thiện Nguyện
  - Transaction-safe: trừ K + insert donation trong cùng transaction
  - Validate: amount_k > 0, ≤ 1.000.000 K/lần, ≤ user.k_balance
  - Hỗ trợ đóng góp ẩn danh (is_anonymous = "Đạo hữu ẩn danh")
  - Lời nhắn tùy chọn (max 500 ký tự)
  - Notification tự động cho admins khi có donation mới (best-effort)

- **[FEAT-3] Dashboard tổng quan Quỹ Từ Bi**
  - Hero số dư Quỹ (gradient xanh tubi, chữ vàng amber)
  - Stats grid: tổng K hệ thống · tổng A · tổng I · tổng lượt đóng góp
  - Quỹ theo chuyên mục: 5 card màu khác nhau (general/sach/tu/qua/thien_nguyen)
  - Top 10 nhà hảo tâm (medal 🥇🥈🥉 cho top 3)
  - 20 đóng góp gần nhất (table với avatar, loại quỹ badge, lời nhắn, thời gian tương đối)
  - 10 khoản chi tiêu gần đây (công khai, minh bạch)

- **[FEAT-4] API endpoint** `GET /api/quy-tu-bi/stats` — JSON tổng quan

- **[FEAT-5] Migration 016**
  - `fund_donations` (id, user_id, amount_k, donation_type, message, is_anonymous, status, created_at, updated_at)
  - `fund_campaigns` (id, name, slug, description, campaign_type, target_amount_k, current_amount_k, start_date, end_date, is_active, created_by)
  - `fund_expenses` (id, amount_k, expense_type, description, receipt_url, spent_at, approved_by, is_public)
  - View `v_fund_summary` — tổng quan thu/chi/số dư/theo loại
  - CHECK constraints: donation_type, status, expense_type, campaign_type
  - 6 index cho performance
  - 2 trigger updated_at
  - Seed: tặng 50 K cho admin_ky_thuat để test donation

- **[FEAT-6] Resilient queries — graceful degradation**
  - Tất cả query `fund_*` trả về empty vec / default value nếu bảng chưa tồn tại
  - Migration 016 chưa chạy? Trang vẫn render được (với số 0 và empty list)
  - `fetch_summary` dùng `COALESCE` cho mọi column → không NULL lỗi

### Sửa (Bug Fixes — CRITICAL: Lỗi đăng nhập)

- **[FIX-1] CRITICAL: Fix production vẫn chạy v0.9.9 dù v0.9.10 đã build & push lên GHCR**
  - Nguyên nhân: `Dockerfile.coolify` dùng `FROM ghcr.io/mhieuhonda/tubi-app:latest`
  - Docker daemon cache stale digest khi `:latest` được update nhưng local cache vẫn giữ digest cũ
  - Triệu chứng: production health check báo `version: 0.9.9` dù v0.9.10 đã push lên GHCR
  - Giải pháp: đổi `Dockerfile.coolify` sang `FROM ghcr.io/mhieuhonda/tubi-app:0.9.11`
    (dùng tag semver thay vì `:latest` — tag semver unique per release → Docker chắc chắn pull image mới)

- **[FIX-2] Đảm bảo v0.9.10's safety schema fix được deploy**
  - v0.9.10 đã thêm `db::ensure_schema_safety()` chạy `ALTER TABLE users ADD COLUMN IF NOT EXISTS i_balance`
    TRƯỚC khi sqlx migrations chạy — fix lỗi "column i_balance does not exist" (Database 42703)
  - Nhưng do FIX-1 (Docker cache stale), v0.9.10 chưa bao giờ được deploy
  - v0.9.11 deploy sẽ mang cả safety schema fix + fallback SELECT lên production
  - Login sẽ KHÔNG BAO GIỜ bị block chỉ vì schema drift (i_balance/role column missing)

- **[FIX-3] Resilient /quy-tu-bi page khi migration 016 chưa chạy**
  - `fetch_recent_donations`, `fetch_top_donors`, `fetch_recent_expenses` trả về empty vec nếu bảng chưa tồn tại
  - `fetch_summary` dùng `fetch_optional` + `unwrap_or_default()` → trả về FundSummary 0 nếu view chưa tồn tại
  - Log debug (không log error) để tránh spam log khi migration chưa chạy

### Routes mới (v0.9.11)

| Method | Path | Mô tả | Auth |
|--------|------|-------|------|
| GET | `/quy-tu-bi` | Trang Quỹ Từ Bi (dashboard + form + lists) | Public |
| POST | `/quy-tu-bi/dong-gop` | Đóng góp K vào quỹ | Auth |
| GET | `/api/quy-tu-bi/stats` | JSON tổng quan | Public |

### Đổi (Refactors)

- **[REF-1] `src/handlers/mod.rs::quy_tu_bi`** delegate sang `handlers::quy_tu_bi::quy_tu_bi_index`
  (bỏ placeholder_page, dùng template đầy đủ)

- **[REF-2] Module structure**
  - `src/models/quy_tu_bi.rs` — DonationType enum, FundDonation, FundDonationWithUser, DonationForm, FundSummary, TopDonor, FundExpense
  - `src/handlers/quy_tu_bi.rs` — quy_tu_bi_index, quy_tu_bi_dong_gop, quy_tu_bi_stats_api, fetch_*, notify_admins_of_donation
  - `templates/quy-tu-bi/index.html` — trang quỹ (Hero + Stats + Funds by Type + Form + Top Donors + Expenses + Recent Donations table)

### Cập Nhật Tài Liệu

- `Cargo.toml` — version `0.9.8` → `0.9.9` → `0.9.10` → `0.9.11`
- `src/main.rs` — log khởi động v0.9.11, phase 15, health check JSON
- `templates/layout.html` — footer version v0.9.10 → v0.9.11
- `templates/admin/ky-thuat/index.html` — version badge + footer v0.9.11
- `templates/admin/quan-li/index.html` — footer v0.9.11
- `templates/admin/users.html` — hierarchy note v0.9.11
- `templates/khong-gian/index.html` — footer v0.9.11
- `templates/bang-xep-hang/index.html` — footer v0.9.11
- `Dockerfile.coolify` — `FROM :latest` → `FROM :0.9.11` (CRITICAL deploy fix)
- `README.md` — thêm entry Giai đoạn 15
- `CHANGELOG.md` — entry này

---

## [0.9.10] — 2026-08-14 — Giai đoạn 14: Bảng Xếp Hạng & Bug Fixes

### Thêm (Features — Giai đoạn 14: Bảng Xếp Hạng & Thống Kê)

- **[FEAT-1] Chuyên mục Bảng Xếp Hạng chính thức ra mắt** (`/bang-xep-hang`)
  - Bỏ placeholder "Giai đoạn 19", thay bằng trang leaderboard đầy đủ
  - 5 tabs: Niệm Lực A · Nguyên Lực I · Tài Phú K · Hôm Nay · Streak

- **[FEAT-2] Leaderboard — Top Niệm Lực A** (tab mặc định)
  - `SELECT ... ORDER BY a_balance DESC` — top 50 active users
  - Hiển thị medal 🥇🥈🥉 cho top 3, highlight hàng đặc biệt

- **[FEAT-3] Leaderboard — Top Nguyên Lực I**
  - `SELECT ... ORDER BY i_balance DESC` — top 50 active users có I > 0
  - Phần thưởng từ Tượng Phật (cầu nguyện / sám hối / hồi hướng)

- **[FEAT-4] Leaderboard — Top Tài Phú K**
  - `SELECT ... ORDER BY k_balance DESC` — top 50 active users có K > 0

- **[FEAT-5] Leaderboard — Top Niệm Phật Hôm Nay**
  - JOIN practice_logs WHERE log_date = CURRENT_DATE — top 50
  - Hiển thị số lần niệm trong ngày

- **[FEAT-6] Leaderboard — Top Streak**
  - SQL window function tính số ngày liên tiếp tu học
  - Điều kiện: ngày cuối cùng ≥ yesterday (streak còn hiệu lực)

- **[FEAT-7] Summary Stats**
  - Tổng users, tổng A, tổng I, tổng K, tổng niệm Phật
  - Hiển thị trên dashboard head của trang

- **[FEAT-8] API endpoint** `GET /api/bang-xep-hang/stats` — JSON tổng quan

### Sửa (Bug Fixes — CRITICAL)

- **[FIX-1] CRITICAL: Fix lỗi đăng nhập "column i_balance does not exist" (Database 42703)**
  - Nguyên nhân: migration 015 chưa chạy trên production (checksum mismatch / partial deploy)
  - Giải pháp triệt để:
    1. Thêm `db::ensure_schema_safety()` chạy TRƯỚC sqlx migrations — chạy trực tiếp
       `ALTER TABLE users ADD COLUMN IF NOT EXISTS i_balance` (idempotent)
    2. Thêm fallback trong `get_user_from_session()`: nếu SELECT với USER_COLUMNS thất bại,
       thử SELECT với minimal columns rồi populate defaults cho i_balance/role
    3. Thêm fallback tương tự trong `auth.rs::upsert_google_user()`:
       nếu INSERT RETURNING thất bại, thử SELECT fallback, rồi minimal SELECT fallback
  - Kết quả: đăng nhập sẽ KHÔNG BAO GIỜ bị block chỉ vì schema drift

- **[FIX-2] Fix `AdminUserRow` thiếu `i_balance` trong SELECT/struct**
  - Thêm `i_balance: i64` vào struct và `COALESCE(u.i_balance, 0) AS i_balance` vào query

- **[FIX-3] Fix mọi stale version strings**
  - Cargo.toml: 0.9.9 → 0.9.10
  - main.rs: v0.9.9 → v0.9.10 (3 locations + health check)
  - templates/layout.html: v0.9.8 → v0.9.10
  - templates/admin/ky-thuat: v0.9.8 → v0.9.10
  - templates/admin/quan-li: v0.9.8 → v0.9.10
  - handlers/mod.rs footer: v0.9.8 → v0.9.10

- **[FIX-4] Safety schema check ensures critical columns/tables always exist**
  - `i_balance` on users, `role` on users, `practice_logs`, `buddha_vows`, `permissions`, `role_permissions`
  - Chạy idempotent DDL trực tiếp, không phụ thuộc sqlx migration tracking

### Routes mới

| Method | Path | Mô tả | Auth |
|--------|------|-------|------|
| GET | `/bang-xep-hang` | Trang Bảng Xếp Hạng (5 tabs) | Public |
| GET | `/api/bang-xep-hang/stats` | JSON tổng quan | Public |

---

## [0.9.9] — 2026-08-14 — Giai đoạn 13: Không Gian Cá Nhân & Niệm Phật

### Thêm (Features — Giai đoạn 13: Không Gian Cá Nhân)

- **[FEAT-1] Chuyên mục Không Gian chính thức ra mắt** (`/khong-gian`)
  - Là 1 trong 4 trụ cột chính của app (Không Gian · Cộng Đồng · Bạn Bè · Kinh Sách)
  - Bỏ placeholder "Giai đoạn 5", thay bằng trang personal space đầy đủ
  - Layout 3 cột: Tượng Phật (trái) · Niệm Phật counter (giữa) · Stats grid (phải)

- **[FEAT-2] Niệm Phật Counter (HTMX realtime)**
  - Nút 🪷 lớn (128x128px) ở giữa trang, mỗi lần nhấn = +1 Niệm Lực A
  - `POST /api/niem-phat` — upsert `practice_logs` (1 row/user/day) + increment `a_balance` trong transaction
  - HTMX swap counter mà không reload page
  - Hiệu ứng pulse-lotus animation (CSS keyframes) khi nhấn — mở 🪷 phóng to + glow vàng
  - Trả về HTML partial mới thay JSON — frontend không cần JS logic

- **[FEAT-3] Tượng Phật (4 chức năng theo HieuLouis/Hệ Thống Và Chức Năng Chi Tiết.docx mục I.6)**
  - 🙏 **Cầu Nguyện** — `POST /tuong-phat/cau-nguyen` → +1 Nguyên lực I
  - 🙇 **Sám Hối** — `POST /tuong-phat/sam-hoi` → +2 Nguyên lực I
  - 🌸 **Hồi Hướng** — `POST /tuong-phat/hoi-huong` → +3 Nguyên lực I
  - (Ủng Hộ chưa làm — cần tích hợp quảng cáo, sẽ thêm ở giai đoạn sau)
  - UI: 3 button → Alpine.js toggle hiện form modal với textarea + checkbox "công khai"
  - Form action binding động bằng Alpine.js (`:action` phụ thuộc `activeVow`)
  - Validate: nội dung 10–2000 ký tự, `vow_type` phải khớp endpoint (chống inject)

- **[FEAT-4] Nhật Ký Tu Học (7 ngày gần nhất)**
  - Biểu đồ cột (bar chart CSS thuần) hiển thị số lần niệm mỗi ngày
  - `DailyNiem::height_pct()` — tính chiều cao cột 5–100% dựa trên max_count
  - Tính `streak` (số ngày liên tiếp có niệm) — `compute_streak()` lùi từ today
  - Empty state: 🪷 + "Chưa có dữ liệu tu học. Bắt đầu niệm Phật để ghi nhận!"

- **[FEAT-5] Bảng Kính Nguyện** — danh sách vow công khai gần nhất (20 cái)
  - `fetch_public_vows()` — JOIN users, filter `is_public=true AND is_active=true`
  - Card layout: badge loại vow (🙏/🙇/🌸) + tên tác giả + timestamp + content
  - `PublicVow::icon()`, `label()`, `color()` — helper methods cho template

- **[FEAT-6] Hệ thống điểm mở rộng**
  - `a_balance` (Niệm Lực A) — có từ Giai đoạn 1
  - `k_balance` (Tiền K) — có từ Giai đoạn 1
  - **`i_balance` (Nguyên lực I) — MỚI v0.9.9** — phần thưởng từ Tượng Phật
  - Stats grid hiển thị: niệm hôm nay / tổng niệm / K / I / streak / cấp bậc

- **[FEAT-7] Migration 015**
  - `ALTER TABLE users ADD COLUMN i_balance BIGINT NOT NULL DEFAULT 0`
  - `CREATE TABLE practice_logs (id, user_id, log_date, niem_count, last_niem_at, ...) UNIQUE(user_id, log_date)`
  - `CREATE TABLE buddha_vows (id, user_id, vow_type, content, is_public, created_at) CHECK vow_type IN (...)`
  - 3 index (user_id, log_date DESC, is_public+created_at DESC)
  - Trigger `practice_logs_updated_at` tự cập nhật `updated_at`
  - Seed: tặng 10 I cho admin_ky_thuat để test UI

- **[FEAT-8] 5 endpoint mới + 1 API JSON**
  - `GET /khong-gian` — Trang personal space
  - `POST /api/niem-phat` — HTMX niệm Phật counter
  - `POST /tuong-phat/cau-nguyen` — Tạo vow Cầu Nguyện
  - `POST /tuong-phat/sam-hoi` — Tạo vow Sám Hối
  - `POST /tuong-phat/hoi-huong` — Tạo vow Hồi Hướng
  - `GET /api/khong-gian/stats` — JSON stats cho dashboard

- **[FEAT-9] Health check `/api/health` mở rộng**
  - Version: `0.9.8` → `0.9.9`, phase: `12` → `13`
  - Thêm `khong_gian` object: status, features, vow_types, i_rewards
  - Thêm 4 features vào mảng: `khong-gian-personal-space`, `niem-phat-counter`, `tuong-phat-vows`, `practice-diary`, `i-balance-nguyen-luc`

### Sửa (Bug Fixes)

- **[BUG-1] CRITICAL: Login "lỗi ghi nhận người dùng" fix triệt để**
  - Root cause: `auth.rs::upsert_google_user` `INSERT ... RETURNING` fail khi struct/column mismatch → bắn lỗi về client → user không login được
  - Fix 1: Thêm SELECT fallback sau khi INSERT fail — nếu row đã được insert (RETURNING fail vì decode), SELECT lại theo `google_sub` để lấy User
  - Fix 2: Truncate `display_name` về 100 ký tự (Google profile name có thể dài hơn `VARCHAR(100)` → PostgreSQL error "value too long")
  - Fix 3: Log chi tiết lỗi theo loại (`ColumnNotFound` / `Database(code,msg)` / `Decode`) kèm `sub`, `email`, `name_len` để debug nhanh
  - Fix 4: Error message hiển thị loại lỗi cho user (thay vì generic "Lỗi khi ghi nhận người dùng")
  - Lưu ý: bản deploy live vẫn đang chạy v0.9.7 (có bug USER_COLUMNS thiếu `role`), v0.9.9 deploy lên sẽ fix toàn bộ

- **[BUG-2] Admin user list — chuyển từ table sang card-based compact layout (theo ảnh tham chiếu)**
  - Trước: `<table>` 8 cột → quá rộng, khó đọc, actions select inline lộn xộn
  - Sau: CSS grid 2 cột, mỗi card có:
    - Header: avatar + tên (bold, lớn) + role badge (top-right, pill màu)
    - Body: @handle (localpart email) + email đầy đủ (muted, nhỏ)
    - Footer: A/K metrics (trái) + online status + actions dropdown ⋮ (phải)
  - `last_session_at` từ subquery `(SELECT MAX(s.created_at) FROM sessions s WHERE s.user_id = u.id)`
  - `last_seen_text()` — helper method trả về (css_class, dot_color, text):
    - "Đang hoạt động" (green dot pulse) — < 5 phút
    - "X phút trước" (gray dot) — < 1 giờ
    - "X giờ trước" — < 1 ngày
    - "X ngày trước" — > 1 ngày
    - "chưa đăng nhập" — NULL session
    - "Bị khóa" (red dot) — `is_active=false`
  - Actions dropdown (Alpine.js `x-data="{ open: false }"`) — 4 form POST submit role change
  - Helper methods trên AdminUserRow: `role_color_hint()`, `handle()`, `role_badge_html()`, `last_seen_text()`

### Đổi (Refactors)

- **[REF-1] `User` model + `USER_COLUMNS` thêm `i_balance`**
  - `src/models/user.rs`: thêm `pub i_balance: i64` field
  - `src/handlers/auth.rs::USER_COLUMNS`: thêm `i_balance` vào SELECT/RETURNING
  - `src/handlers/mod.rs::USER_COLUMNS`: thêm `u.i_balance`

- **[REF-2] Bỏ wrapper `handlers::khong_gian()` không dùng**
  - `main.rs` gọi trực tiếp `handlers::khong_gian::khong_gian_index`
  - Wrapper delegate trong `handlers/mod.rs` bị dead-code → xóa

- **[REF-3] Nav "Không Gian" trỏ tới `/khong-gian` thay vì `/`**
  - `templates/layout.html` (3 chỗ: desktop nav, mobile menu, footer)
  - `src/handlers/mod.rs::placeholder_page` (4 chỗ: nav_kg, bottom_kg, mobile, footer)
  - `active_page` check: `"home"` → `"khong_gian"`

- **[REF-4] Module structure**
  - `src/models/khong_gian.rs` — PracticeLog, BuddhaVow, PublicVow, VowType, BuddhaVowForm, KhongGianStats, DailyNiem
  - `src/handlers/khong_gian.rs` — khong_gian_index, niem_phat, tuong_phat_*, khong_gian_stats_api, fetch_*, compute_streak
  - `templates/khong-gian/index.html` — trang personal space (Hero + Nhật ký + Bảng Kính Nguyện)

### Cập Nhật Tài Liệu

- `Cargo.toml` — version `0.9.8` → `0.9.9`
- `src/main.rs` — log khởi động v0.9.9, phase 13, health check JSON
- `README.md` — thêm entry Giai đoạn 13
- `CHANGELOG.md` — entry này

---

## [0.9.8] — 2026-08-14 — Giai đoạn 12: 50 quyền chi tiết + 3 giao diện admin riêng biệt

### Thêm (Features)

- **[FEAT-1] Hệ thống 50 quyền chi tiết (Granular Permissions)**
  - Bảng `permissions` — 50 quyền chia 5 nhóm x 10 quyền:
    - `system` (10): system_view_status, system_manage_config, system_manage_migrate, system_view_logs, system_manage_cache, system_restart_server, system_manage_cron, system_view_metrics, system_manage_backup, system_debug_mode
    - `users` (10): users_view_list, users_view_detail, users_edit_profile, users_change_role, users_activate, users_delete, users_ban, users_view_sessions, users_manage_oauth, users_export_data
    - `content` (10): content_view_pending, content_approve, content_edit_any, content_delete_any, content_pin_lock, content_manage_cat, content_manage_tags, content_mod_comments, content_mod_reviews, content_feature
    - `community` (10): community_view_stats, community_manage_grp, community_create_off, community_manage_evt, community_manage_chat, community_manage_mem, community_broadcast, community_manage_inv, community_archive, community_merge
    - `kinh_sach` (10): ksach_manage_books, ksach_manage_chap, ksach_upload, ksach_manage_cat, ksach_review_mod, ksach_donation_mgr, mail_view_all, notif_send_all, analytics_view, api_manage_keys

- **[FEAT-2] Bảng role_permissions — gán quyền cho role**
  - `admin_ky_thuat` → 50/50 quyền (TẤT CẢ — toàn quyền hệ thống)
  - `admin_quan_li` → 30/50 quyền (users + content + community)
  - `admin_cong_dong` → 20/50 quyền (content + community)
  - `member` → 0 quyền admin

- **[FEAT-3] Nâng Admin Kỹ Thuật lên chức vụ CAO NHẤT**
  - Hierarchy mới: admin_ky_thuat (level 4) > admin_quan_li (level 3) > admin_cong_dong (level 2) > member (level 1)
  - Admin Kỹ Thuật có toàn bộ 50 quyền — quyền cao nhất trong hệ thống
  - Admin Quản Lý không thể nâng ai lên Admin Kỹ Thuật (chỉ Admin Kỹ Thuật mới được)

- **[FEAT-4] 3 giao diện bảng quản trị riêng biệt**
  - `/admin/ky-thuat` — Phong cách Coder/Terminal — tối, Matrix-like, cực ngầu
    - JetBrains Mono font, terminal-style layout, scanline effects, green glow
    - Permission matrix table, terminal commands sidebar, uptime counter
    - KHÔNG dùng layout.html — standalone dark theme hoàn toàn khác web
  - `/admin/cong-dong` — Phong cách Community Moderator — xanh dương, social, ấm áp
    - Tab navigation, pending content badges, warm color palette
    - Content moderation cards, community stats overview
    - KHÔNG dùng layout.html — standalone blue theme
  - `/admin/quan-li` — Phong cách Executive/Premium — vàng, luxury dashboard
    - Gold gradient header, premium stat cards, executive navigation
    - Permission summary per category, user management quick links
    - KHÔNG dùng layout.html — standalone premium theme
  - `/admin` — Redirect tự động đến dashboard tương ứng role

- **[FEAT-5] Migration 014**: `014_granular_permissions.sql`
  - Bảng `permissions` (50 rows seed) + bảng `role_permissions` + view `v_user_permissions`
  - Function `user_has_permission(UUID, VARCHAR)` cho permission check nhanh
  - Indexes cho truy vấn nhanh

- **[FEAT-6] User model mở rộng** (v0.9.8):
  - `role_level()` cập nhật: admin_ky_thuat → 4 (cao nhất)
  - `has_permission_code(code)` — kiểm tra quyền chi tiết
  - `permission_count()` — tổng số quyền (50/30/20/0)
  - `admin_dashboard_path()` — path dashboard riêng theo role
  - `can_manage_admin()` — quyền quản trị (level ≥ 3)

### Sửa (Bug Fixes)

- **[FIX-1] CRITICAL**: `USER_COLUMNS` trong `auth.rs` thiếu cột `role` — Google OAuth login bị hỏng hoàn toàn
  - Thêm `, role` vào USER_COLUMNS trong auth.rs
  - Bug này tồn tại từ v0.9.7 khi migration 013 thêm cột role nhưng quên cập nhật auth.rs
- **[FIX-2]**: Migration 007 `CREATE TABLE` thiếu `IF NOT EXISTS` — không idempotent
- **[FIX-3]**: Role change permission mở rộng — admin_ky_thuat + admin_quan_li đều đổi role được
- **[FIX-4]**: Admin tự đổi role protection — không cho admin tự hạ/thăng cấp chính mình

### Thay đổi (Changed)

- Hierarchy đổi từ: quan_li > cong_dong > ky_thuat → **ky_thuat > quan_li > cong_dong**
- Admin change role: chỉ admin_ky_thuat được nâng ai lên admin_ky_thuat
- `/admin` route giờ redirect đến dashboard role-specific thay vì render template chung
- Health check `/api/health` thêm `permission_counts` và dashboard paths
- Version: 0.9.7 → 0.9.8

---

## [0.9.7] — 2026-08-14 — Giai đoạn 11: Hệ thống vai trò Admin & Phân quyền cộng đồng

### Thêm (Features)

- **[FEAT-1] Hệ thống phân quyền 4 cấp** — lần đầu tiên app có vai trò quản trị rõ ràng
  - `member` — Thành Viên (mặc định)
  - `admin_ky_thuat` — Admin Kỹ Thuật (hệ thống, server, DB, mã nguồn)
  - `admin_cong_dong` — Admin Cộng Đồng (duyệt nội dung, mod diễn đàn)
  - `admin_quan_li` — Admin Quản Lý (super admin — quyền cao nhất)
  - Hierarchy: Admin Quản Lý > Admin Cộng Đồng > Admin Kỹ Thuật > Thành Viên

- **[FEAT-2] Migration 013**: `013_admin_roles.sql`
  - Thêm cột `role VARCHAR(30) NOT NULL DEFAULT 'member'` vào bảng `users`
  - CHECK constraint: `role IN ('member', 'admin_ky_thuat', 'admin_cong_dong', 'admin_quan_li')`
  - Index `idx_users_role` cho truy vấn nhanh
  - UPSERT `khongdich.admin@gmail.com` → `admin_ky_thuat` (theo yêu cầu dự án)
  - Khi user này đăng nhập Google lần đầu, `google_sub` được tự động link vào record
    hiện có (qua logic `upsert_google_user`), giữ nguyên role `admin_ky_thuat`

- **[FEAT-3] Hiển thị chức vụ trên hồ sơ** (`templates/profile.html`)
  - Role badge mới bên cạnh rank badge, dùng màu + icon riêng:
    - 👑 Admin Quản Lý (vàng cam #FF6F00)
    - 🛡️ Admin Cộng Đồng (xanh dương #1565C0)
    - ⚙️ Admin Kỹ Thuật (tím #6A1B9A)
    - 🪷 Thành Viên (xanh lá #2E7D32)
  - Mục "Chức vụ" mới trong bảng Thông Tin Tài Khoản
  - Nút "Vào trang Quản Trị" (chỉ hiện với admin) ở cuối bảng

- **[FEAT-4] Hiển thị chức vụ trong header** (`templates/layout.html`)
  - Desktop header: role badge nhỏ bên cạnh tên user
  - Mobile menu: hiển thị role + link /admin
  - Link "⚙️ Quản Trị" trong header (chỉ admin nhìn thấy)

- **[FEAT-5] Trang Quản Trị** (`/admin`)
  - `GET /admin` — Dashboard với stats: tổng users, active users, admin count,
    tổng groups/topics/comments/books/mails, cảm ngộ chờ duyệt
  - `GET /admin/thanh-vien` — Danh sách thành viên + role + rank + trạng thái
    (sort theo role hierarchy: admin_quan_li → admin_cong_dong → admin_ky_thuat → member)
  - `POST /admin/thanh-vien/{user_id}/role` — Đổi role user
    - **Chỉ Admin Quản Lý mới được đổi role user khác**
    - Không cho admin tự demote chính mình (tránh khoá mình ra khỏi hệ thống)
    - Validate role phải thuộc 4 giá trị hợp lệ
  - Permission gate: user không phải admin → render 403 Forbidden page

- **[FEAT-6] User model mở rộng** (`src/models/user.rs`)
  - Thêm field `role: String`
  - Helper methods:
    - `role_display()` → "Thành Viên" / "Admin Kỹ Thuật" / "Admin Cộng Đồng" / "Admin Quản Lý"
    - `role_icon()` → 🪷 / ⚙️ / 🛡️ / 👑
    - `role_color()` → hex color cho badge
    - `role_level()` → 1-4 (dùng để so sánh quyền)
    - `is_admin()` — true nếu user là bất kỳ admin nào
    - `is_admin_ky_thuat()` / `is_admin_cong_dong()` / `is_admin_quan_li()` — check chính xác role
    - `can_manage_technical()` — admin_ky_thuat trở lên
    - `can_manage_community()` — admin_cong_dong trở lên

- **[FEAT-7] Health check endpoint** mở rộng
  - `/api/health` giờ trả về thêm `admin` stats (total_users, active_users, admins)
  - `roles` object mô tả hierarchy + admin panel access
  - `phase: 11`, `version: "0.9.7"`
  - `features` list thêm: `admin-roles`, `admin-panel`, `role-based-permissions`

- **[FEAT-8] Placeholder pages** (handlers/mod.rs `render_user_menu_html`,
  `render_mobile_user_menu_html`) — giờ hiển thị role badge + link /admin nếu user là admin

### Sửa (Bug Fixes)

- Không có bug nghiêm trọng trong giai đoạn này — đây là giai đoạn feature-driven
  nên không có regression nào từ v0.9.6.

### Thay đổi (Changes)

- `Cargo.toml` — version `0.9.6` → `0.9.7`
- `Cargo.lock` — sync version
- `src/main.rs` — version string, phase number, thêm routes `/admin/*`, thêm
  `fetch_admin_stats_summary()` helper cho health check
- `src/handlers/mod.rs` — `USER_COLUMNS` thêm `u.role`; thêm `admin` module;
  thêm `handlers::admin` delegate function; `render_user_menu_html` và
  `render_mobile_user_menu_html` giờ hiển thị role badge + link /admin
- `src/handlers/admin.rs` — file mới (~400 dòng): handlers + templates structs
  + helpers cho /admin, /admin/thanh-vien, /admin/thanh-vien/{id}/role
- `src/models/user.rs` — thêm field `role` + 8 helper methods
- `templates/profile.html` — thêm role badge + "Chức vụ" trong account info +
  nút "Vào trang Quản Trị"
- `templates/layout.html` — role badge trong header + mobile menu + link /admin
- `templates/admin/index.html` — file mới: dashboard admin
- `templates/admin/users.html` — file mới: danh sách thành viên + UI đổi role
- `templates/layout.html` + `src/handlers/mod.rs` — footer version `v0.9.6` → `v0.9.7`
- `Dockerfile.coolify` — comment update + dùng `:latest` tag (sẽ được GH Actions
  update tự động)
- `migrations/013_admin_roles.sql` — file mới: schema cho role system
- `README.md` — thêm Giai đoạn 11 + update version references
- `CHANGELOG.md` — entry v0.9.7

### Mục tiêu đạt được

- ✅ Email `khongdich.admin@gmail.com` được gán `admin_ky_thuat` qua migration seed
- ✅ Chức vụ hiển thị trên hồ sơ (bên cạnh rank badge + trong account info)
- ✅ Hierarchy rõ ràng: Admin Quản Lý > Admin Cộng Đồng > Admin Kỹ Thuật > Thành Viên
- ✅ Trang /admin cơ bản (dashboard + user list + đổi role)
- ✅ Permission gate: chỉ admin mới vào /admin được
- ✅ Build pipeline giữ nguyên (GitHub Actions → GHCR → Coolify auto-deploy)

---

## [0.9.6] — 2026-08-14 — Giai đoạn 10: Kinh Sách (Thư viện kinh sách Phật giáo & Đạo giáo)

### Thêm (Features)

- **[FEAT-1] Chuyên mục Kinh Sách chính thức ra mắt** — không còn placeholder
  - 1 trong 4 chuyên mục chính của app (Không Gian, Cộng Đồng, Bạn Bè, Kinh Sách) cuối cùng hoàn thiện
  - Theo thiết kế `HieuLouis/Hệ Thống Và Chức Năng Chi Tiết.docx` mục IV. Kinh Sách

- **[FEAT-2] 5 Thư Viện Chính**:
  - 🪷 Phật Gia — Kinh điển, luận thư, pháp thoại Phật giáo
  - ☯️ Đạo Gia — Đạo Đức Kinh, Nam Hoa Kinh, thư tịch Đạo giáo
  - 📜 Kinh Văn — Kinh văn tụng đọc, chú giải, nghi thức
  - 💎 Sách Quý — Khoa học, Triết học, Tâm học, Văn học
  - ⭐ Quan Trọng — Bài viết quan trọng do Quản Lý chọn lựa

- **[FEAT-3] Hệ thống Sách (Books)**:
  - Sách điện tử hoàn chỉnh (`book_type=single`) hoặc theo chương (`book_type=multi`)
  - Trang sách `/kinh-sach/{slug}` hiển thị thông tin + danh sách chương + cảm ngộ
  - Tự tăng `view_count` khi xem sách/chương
  - Phân loại theo 5 thư viện + 3 ngôn ngữ (vi/en/zh, ưu tiên Tiếng Việt)
  - Hỗ trợ `download_url` để tải offline

- **[FEAT-4] Hệ thống Chương (Chapters)**:
  - Trang đọc chương `/kinh-sach/{slug}/chuong/{chapter_slug}` với sidebar mục lục sticky
  - Điều hướng trước/sau giữa các chương (prev/next)
  - Tự tăng `view_count` chương khi đọc
  - Highlight chương hiện tại trong sidebar

- **[FEAT-5] Hệ thống Cảm Ngộ (Reviews)**:
  - Form cảm ngộ ngay trên trang sách (tối thiểu 100 chữ, tối đa 10.000 chữ)
  - **Phải qua xét duyệt** mới hiển thị công khai (status: `pending` → `approved`)
  - 1 user chỉ được viết 1 cảm ngộ/sách (partial unique index `WHERE is_active = true`), có thể edit
  - Hiển thị trạng thái cảm ngộ của user (chờ duyệt / đã duyệt / bị từ chối)
  - Theo HieuLouis/: "Cảm ngộ phải có tối thiểu 100 chữ và qua xét duyệt thì mới được hiển thị."

- **[FEAT-6] Tặng Hoa (Flowers)**:
  - 1 user chỉ tặng 1 hoa/sách (unique index `book_flowers`)
  - Tự tăng counter `flower_count` qua trigger
  - Button disable + label "Đã tặng hoa" nếu user đã tặng

- **[FEAT-7] Tìm kiếm sách** `/kinh-sach/tim-kiem?q=`:
  - Dùng ILIKE để match title/author/description
  - Priority: title match trước, sau đó sort theo view_count
  - Giới hạn 50 kết quả

- **[FEAT-8] Lọc theo thư viện** `/kinh-sach/thu-vien/{category_slug}`:
  - Hiển thị danh sách sách trong 1 thư viện cụ thể
  - Hero banner riêng cho từng thư viện với icon + description

- **[FEAT-9] Health check mở rộng**:
  - Thêm `kinh_sach` stats: số sách, chương, cảm ngộ đã duyệt, tổng lượt xem

- **[FEAT-10] Seed 4 cuốn sách mẫu**:
  - Kinh A Di Đà (single, Phật Gia) — featured
  - Đạo Đức Kinh (multi, Đạo Gia, 3 chương: 1, 2, 8) — featured
  - Kinh Tam Đại Hải (single, Phật Gia)
  - Kinh Pháp Cú (multi, Phật Gia, 1 chương) — featured

### Migration

- **012_kinh_sach.sql**: 6 bảng mới
  - `book_categories` — 5 thư viện + seed data
  - `books` — sách điện tử (title, author, description, category, language, cover_url, download_url, book_type, status, is_featured)
  - `book_chapters` — chương mục (title, content, sort_order, view_count)
  - `book_reviews` — cảm ngộ (body, status, flower_count) với partial unique index
  - `book_donations` — donate K (sẽ activate khi có hệ thống tiền tệ)
  - `book_flowers` — tặng hoa với unique(book_id, user_id)
  - 5 triggers: `set_updated_at` × 3, `update_book_chapter_count`, `update_book_flower_count`, `update_book_review_count`
  - `CREATE EXTENSION IF NOT EXISTS pg_trgm` cho fuzzy search
  - Index: title gin_trgm, category, language, status, featured, active, created, view_count

### Sửa (Bug Fixes)

- **[BUG-1] Fix clippy warning `collapsible_if`** trong `handlers/friends.rs` (mail_view):
  - Collapse 2 nested `if` thành `if let Some(ref m) = mail && m.recipient_id == user.id && !m.is_read { ... }`
  - Sử dụng let-chains (Rust 1.88+)

### Routes mới (12 endpoints)

| Method | Path | Mô tả | Auth |
|--------|------|-------|------|
| GET | `/kinh-sach` | Trang chính Kinh Sách | Public |
| GET | `/kinh-sach/tim-kiem` | Tìm kiếm sách `?q=` | Public |
| GET | `/kinh-sach/thu-vien/{category_slug}` | Lọc theo thư viện | Public |
| GET | `/kinh-sach/{slug}` | Trang sách + chương + cảm ngộ | Public |
| GET | `/kinh-sach/{slug}/chuong/{chapter_slug}` | Đọc chương | Public |
| POST | `/kinh-sach/{slug}/cam-ngo` | Gửi cảm ngộ | Auth |
| POST | `/kinh-sach/{slug}/tang-hoa` | Tặng hoa | Auth |

### Files mới

- `src/models/kinh_sach.rs` — Book, BookWithCategory, BookChapter, BookChapterSummary, BookReview, BookReviewWithAuthor, BookCategory, BookReviewForm
- `src/handlers/kinh_sach.rs` — 7 handlers + 1 stats helper
- `templates/kinh-sach/index.html` — trang chính
- `templates/kinh-sach/category.html` — lọc theo thư viện
- `templates/kinh-sach/search.html` — kết quả tìm kiếm
- `templates/kinh-sach/book.html` — trang sách + form cảm ngộ
- `templates/kinh-sach/chapter.html` — đọc chương với sidebar mục lục
- `migrations/012_kinh_sach.sql` — schema + seed data

### Files sửa

- `Cargo.toml` — version `0.9.5` → `0.9.6`
- `src/main.rs` — version strings, log messages, routes mới, health check `kinh_sach` stats
- `src/handlers/mod.rs` — đăng ký module `kinh_sach`, `kinh_sach()` delegate, version footer
- `src/handlers/friends.rs` — fix clippy warning
- `src/models/mod.rs` — đăng ký module `kinh_sach` + re-export
- `templates/layout.html` — version footer `v0.9.5` → `v0.9.6`
- `README.md` — Giai đoạn 10 entry, routes mới, cấu trúc dự án, version list
- `CHANGELOG.md` — entry v0.9.6

### Mục tiêu

Thành viên có thể đọc kinh sách online, viết cảm ngộ (qua xét duyệt), tặng hoa kính dâng — bù lỗ hổng kiến thức của v0.9.5.

---

## [0.9.5] — 2026-08-14 — Giai đoạn 9: Module Bạn Bè + Fix live chat bugs

### Sửa (Bug Fixes — Critical)

- **[BUG-1] Fix Live Chat tổng (Chat Chung) luôn báo "đang kết nối..."**:
  - Nguyên nhân: `src/static/js/app.js` `globalChat().init()` có check `if (!document.cookie.includes('session_id')) return;` — nhưng cookie `session_id` được set với `http_only(true)` trong `auth.rs`, nên `document.cookie` KHÔNG đọc được → check luôn false → `connect()` không được gọi → `connected` vẫn `false` → UI hiển thị "đang kết nối..." mãi mãi.
  - Fix: bỏ check `document.cookie.includes('session_id')` vì layout đã chỉ render global chat khi user đăng nhập (`{% if let Some(_u) = user %}`). Server sẽ trả 401 nếu chưa đăng nhập.
  - Triết lý: HttpOnly cookies KHÔNG thể đọc qua `document.cookie` (security feature), browser tự gửi chúng cho same-origin WebSocket.

- **[BUG-2] Fix Live Chat cộng đồng (group chat) không gửi được tin nhắn**:
  - Nguyên nhân gián tiếp: cùng pattern bug-1, plus thiếu server-side logging khi INSERT fail → khó debug.
  - Fix: thêm `log::error!` + gửi error payload JSON cho client khi INSERT thất bại (cả group chat và global chat). Trước đây error bị silently drop, giờ client nhận được thông báo "Không lưu được tin nhắn. Vui lòng thử lại."

- **[BUG-3] Version string mismatch**: `templates/layout.html` và `src/handlers/mod.rs` `placeholder_page` hiển thị `v0.9.3` trong khi thực tế là v0.9.4 → cập nhật lên v0.9.5.

- **[BUG-4] `placeholder_page` cho `/ban-be` ghi sai "Giai đoạn 15"** — theo tài liệu HieuLouis, Bạn Bè là **Giai đoạn 9**. Đã sửa và delegate sang `handlers::friends::ban_be_index`.

### Thêm (Features — Giai đoạn 9: Module Bạn Bè)

- **BB-01 Kết bạn (Friend System)**:
  - `POST /ban-be/keu-ban/{user_id}` — Gửi lời mời kết bạn (check existing friendship, không self-friend)
  - `POST /ban-be/chap-nhan/{friendship_id}` — Chấp nhận (chỉ addressee mới được accept, tạo notification cho requester)
  - `POST /ban-be/tu-choi/{friendship_id}` — Từ chối (xóa friendship để cho phép gửi lại sau)
  - `POST /ban-be/huy-ket-ban/{user_id}` — Hủy kết bạn (xóa friendship accepted)
  - `GET /ban-be` — Trang chính: danh sách bạn bè + lời mời đang chờ + lời mời đã gửi
  - `GET /ban-be/tim-kiem?q=...` — Tìm user theo display_name/email/phap_danh/phap_hieu/but_danh (hiển thị trạng thái: đã là bạn / đã gửi / đã nhận / chưa)

- **BB-02 Nhắn tin 1-1 (Direct Messaging)**:
  - `GET /ban-be/tin-nhan` — Inbox DM (danh sách conversation + last message preview)
  - `GET /ban-be/tin-nhan/{conversation_id}` — Xem conversation + chat realtime
  - `WS /ws/ban-be/tin-nhan/{conversation_id}` — WebSocket DM (Axum 0.8 WebSocketUpgrade, reuse ChatHub pattern)
  - `GET /api/ban-be/tin-nhan/{conversation_id}/history?limit=&before=` — REST history (paginated)
  - `POST /ban-be/tao-conversation` — Tạo (hoặc lấy) conversation 1-1 với user khác
  - `DmChatHub` struct: broadcast per-conversation (capacity 128)
  - `dmChat()` Alpine.js component: auto-reconnect exponential backoff, auto-scroll, formatTime vi-VN
  - Max 1000 ký tự/tin nhắn (gấp đôi group chat)

- **BB-03 Gửi thư (Mail/Inbox)**:
  - `GET /ban-be/thu` — Hộp thư đến (đếm unread, hiển thị sender + subject + preview body)
  - `GET /ban-be/thu/gui` — Form soạn thư (chọn recipient từ danh sách bạn bè)
  - `POST /ban-be/thu/gui` — Gửi thư (validate subject max 200, body không rỗng, không self-mail; tạo notification cho recipient)
  - `GET /ban-be/thu/{mail_id}` — Xem thư (auto mark as read nếu user là recipient)

- **Notification Center**:
  - `GET /ban-be/thong-bao` — Danh sách thông báo (auto mark all as read sau khi load)
  - `GET /api/ban-be/thong-bao/chua-doc` — Đếm unread (cho badge, poll mỗi 30s)
  - `POST /api/ban-be/thong-bao/{notification_id}/da-doc` — Mark 1 thông báo đã đọc
  - Types: `friend_request`, `friend_accept`, `friend_decline`, `mail`, `dm`, `system`, `group_invite`
  - `notificationBadge()` Alpine.js component: bell icon ở header với red badge số unread
  - Payload JSONB linh hoạt cho mỗi loại

- **Migrations mới**:
  - `008_friendships.sql` — Bảng `friendships` (requester_id, addressee_id, status, unique pair, no self-friend check, updated_at trigger)
  - `009_conversations_direct_messages.sql` — 3 bảng: `conversations` (direct/group), `conversation_participants` (unique user per conv), `direct_messages` (max 1000 chars)
  - `010_mails.sql` — Bảng `mails` (sender, recipient, subject max 200, body TEXT, is_read, read_at)
  - `011_notifications.sql` — Bảng `notifications` (user_id, type, actor_id, payload JSONB, is_read, read_at)

- **Models mới (`src/models/friends.rs`)**:
  - `Friendship`, `FriendshipWithUser` (join users để lấy other_user info)
  - `Conversation`, `DirectMessage`, `DirectMessageWithAuthor`
  - `ConversationWithParticipant` (join users + LATERAL last_message)
  - `Mail`, `MailWithUsers` (join cả sender + recipient)
  - `Notification`, `NotificationWithActor`
  - **Quan trọng**: field `r#type` được rename thành `kind` để tránh Rust keyword conflict trong Askama templates

- **Templates mới (`templates/ban-be/`)**:
  - `index.html` — Trang chính Bạn Bè (hero + pending requests + friends list + sent requests)
  - `dm_inbox.html` — Inbox DM với last message preview
  - `conversation.html` — Conversation view với WebSocket DM chat (message bubbles differentiate author vs other)
  - `mail_inbox.html` — Hộp thư với unread highlighting
  - `mail_compose.html` — Form soạn thư (select recipient from friends)
  - `mail_view.html` — Xem thư chi tiết (auto mark as read)
  - `notifications.html` — Danh sách thông báo với type-specific messages
  - `search.html` — Tìm user với friend status indicators

- **Frontend (`src/static/js/app.js`)**:
  - `dmChat()` Alpine.js component — tương tự `liveChat()` + `globalChat()` nhưng cho DM 1-1
  - `notificationBadge()` Alpine.js component — poll `/api/ban-be/thong-bao/chua-doc` mỗi 30s, hiển thị red badge
  - Cập nhật `layout.html`: thêm notification bell + "Bạn Bè" link ở header

- **AppState (`src/main.rs`)**:
  - Thêm `dm_chat_hub: DmChatHub` vào `AppState`
  - Update version strings v0.9.4 → v0.9.5
  - Update phase 8 → phase 9 trong health_check JSON
  - Thêm 16 routes mới cho Friends module

- **Cargo.toml**:
  - Version `0.9.4` → `0.9.5`
  - Thêm feature `json` cho sqlx (hỗ trợ JSONB mapping với `serde_json::Value` cho notification payload)

### Kiểm tra (Verification)
- ✅ `cargo check` pass với Rust 1.97.1 (không warnings sau khi thêm `#![allow(dead_code)]`)
- ✅ Askama templates compile thành công (sau khi fix 3 lỗi syntax: inline if/else → `{% if %}{% endif %}`, closure → `{% if let %}`)
- ✅ Code structure tuân thủ pattern hiện có (ChatHub/GlobalChatHub → DmChatHub)
- ✅ Migrations SQL tuân thủ pattern (COMMENT ON TABLE/COLUMN, index, trigger updated_at)

### Ghi chú hạ tầng
- Coolify infrastructure đã OK từ v0.9.4: Traefik forward WebSocket đúng, SSL Let's Encrypt OK, health check `/api/health` pass, Sentinel enabled (push metrics 60s, history 7 ngày).
- Migration 006 + 007 đã chạy trên production DB (kiểm tra bằng `curl /api/chat-chung/history` → 200 []).
- Nguyên nhân live chat không hoạt động KHÔNG phải do Coolify/Sentinel/VPS mà do BUG trong frontend `app.js` (check HttpOnly cookie qua `document.cookie`).

---

## [0.9.4] — 2026-08-14 — Giai đoạn 8: CI/CD tự động (GitHub Actions + Docker Image + Coolify)

### Thêm
- **Workflow `.github/workflows/docker.yml`** — pipeline CI/CD hoàn toàn tự động:
  - Trigger: push lên branch `main` hoặc tạo tag `v*` (hỗ trợ cả `workflow_dispatch` để chạy thủ công)
  - **Job `build-and-push`**: build multi-stage Docker image với Rust 1.97.1-slim-bookworm, push lên GHCR (`ghcr.io/mhieuhonda/tubi-app`) với multi-tag: `latest`, `sha-<short>`, `vX.Y.Z`, `vX.Y`, `<branch>`. *(Image name `tubi-app` thay vì `ungdungtubi` để tránh conflict với GHCR package cũ `ungdungtubi` không linked với repo — GITHUB_TOKEN bị 403 Forbidden khi push vào package cũ.)*
  - Buildx cache (type=gha) — build sau chỉ mất ~30s (chỉ rebuild các layer thay đổi)
  - `concurrency` group để tránh xung đột deploy (không cancel build đang chạy)
  - Permissions tối thiểu: `contents: read`, `packages: write`
  - Summary panel hiển thị image digest + tags
- **Job `trigger-coolify`**: gọi Coolify API `/api/v1/applications/{uuid}/start` để queue deploy. Coolify nhận → pull image `:latest` từ GHCR → stop container cũ → start container mới → run health check.
- **GitHub Secrets** setup: `COOLIFY_API_TOKEN`, `COOLIFY_APP_UUID` (đã add vào repo settings).
- **Coolify app** chuyển `build_pack` từ `dockerfile` (build từ source trên VPS — tốn CPU/RAM VPS) sang `dockerimage` (pull image đã build sẵn từ GHCR — chỉ mất vài giây).
- **README.md** cập nhật:
  - Bảng công nghệ: thêm dòng `CI/CD` (GitHub Actions + Coolify API) và `Registry` (GHCR)
  - Mục "Giai đoạn 8: CI/CD tự động" mới trong lộ trình 25 giai đoạn
  - Section "Production (CI/CD tự động qua GitHub Actions + Coolify)" thay thế cho section deploy thủ công cũ
  - Lịch sử thay đổi CI/CD (v0.5 → v0.9.1 → v0.9.4)
  - Cấu trúc dự án cập nhật với `.github/workflows/docker.yml`
- **CHANGELOG.md** bổ sung entry v0.9.4.

### Sửa
- **`Cargo.toml`**: bump version `0.9.3` → `0.9.4`.
- **`src/main.rs`**: cập nhật version string trong log khởi động (`v0.9.3` → `v0.9.4`), trong health check JSON response (`"version": "0.9.4"`, `"phase": 8`), và phase_name.

### Lợi ích so với v0.9.1 (deploy thủ công)
- **Không tốn CPU/RAM VPS để build**: GitHub-hosted runner build image, VPS chỉ pull image đã build sẵn (~30 MB).
- **Deploy nhanh**: chỉ mất ~10-30s (pull + restart) thay vì 5-10 phút build từ source.
- **Rollback dễ dàng**: đổi tag image trong Coolify từ `:latest` sang `:sha-<old>` hoặc `:v0.9.3` → deploy lại.
- **Reproducible build**: image được build trong môi trường GitHub runner chuẩn, không phụ thuộc VPS state.
- **Multi-arch ready**: chỉ cần thêm `platforms: linux/amd64,linux/arm64` vào workflow để hỗ trợ ARM VPS.
- **Audit trail**: mỗi image có SHA + tag version, dễ trace ngược về commit nào.

---

## [0.9.2] — 2026-08-14 — Giai đoạn 7: Live Chat WebSocket trong Nhóm

### Thêm
- **Live Chat real-time (WebSocket) trong Nhóm** — điểm khác biệt cốt lõi của Cộng Đồng Ứng Dụng Từ Bi so với Telegram/Zalo/Facebook Group. Theo thiết kế trong `HieuLouis/Giao Diện Cộng Đồng Trong Ứng Dụng.docx`: Live Chat kết hợp với List Chủ Đề trong mỗi nhóm, Live Chat chỉ để giao lưu / kết bạn / tán gẫu / hỏi nhanh, mọi nội dung có giá trị nên được chuyển thành Chủ Đề.
- **Migration 006**: bảng `group_chat_messages` (id, group_id, author_id, body VARCHAR(500), is_active, created_at) + 2 index (group+created_at DESC cho history, author cho profile) + comments. Phân biệt rõ với `comments`: comments gắn trên Chủ Đề (lưu trữ tri thức), còn `group_chat_messages` là chat real-time (kết nối, giao lưu).
- **WebSocket endpoint** `GET /ws/cong-dong/nhom/{slug}` — Axum 0.8 `WebSocketUpgrade`:
  - Auth bằng `session_id` cookie trước khi upgrade (HTTP 401 nếu chưa đăng nhập)
  - Resolve `group_id` từ slug + kiểm tra `is_active`
  - Kiểm tra user có membership `active` trong nhóm (HTTP 403 nếu không phải member)
  - Upgrade WebSocket → spawn 2 task song song:
    - `send_task`: forward từ broadcast channel → client (tin nhắn từ người khác)
    - recv loop: đọc từ client → persist DB → broadcast (tin nhắn của mình)
  - Khi client ngắt, `send_task` bị abort để tránh task leak
- **REST endpoint** `GET /api/cong-dong/nhom/{slug}/chat-history?limit=50&before={iso8601}` — paginated chat history:
  - Public (ai cũng xem được chat history của nhóm public)
  - `limit` clamp [1, 100], mặc định 50
  - `before` (RFC 3339) cho cursor pagination — lấy tin nhắn có `created_at < before`
  - Trả về JSON array các ChatMessageWithAuthor (mới nhất trước)
- **ChatHub** — quản lý `HashMap<Uuid, broadcast::Sender<String>>` trong `Arc<Mutex<...>>`:
  - Mỗi nhóm có một broadcast channel (capacity 256)
  - Client subscribe khi kết nối WebSocket, unsub khi ngắt
  - `broadcast()` gửi payload JSON đến tất cả client online trong nhóm
  - Bỏ qua lỗi "no receivers" (không ai online khi tin nhắn được gửi — bình thường)
  - Bỏ qua `Lagged` (client chậm, bỏ qua tin cũ, tiếp tục)
- **Models `ChatMessage` + `ChatMessageWithAuthor`** trong `models/community.rs` — derive Serialize cho JSON response + FromRow cho sqlx.
- **Helper `recent_messages(pool, group_id)`** trong `handlers/chat.rs` — trả về 20 tin nhắn gần nhất (đã đảo ngược để render oldest-first) cho SSR template.
- **Template `community/group.html` cập nhật** — thêm Live Chat panel phía dưới Topics List:
  - Panel cao 360px (desktop) / 300px (mobile) — chiếm ~35% chiều cao, List Chủ Đề chiếm 65%
  - Render 20 tin nhắn gần nhất từ SSR (`chat_messages_json` truyền từ Rust)
  - Alpine.js `liveChat()` component quản lý WebSocket state
  - Hiển thị avatar (Google avatar hoặc chữ cái đầu tên), tên, thời gian, bubble chat
  - Auto-scroll xuống cuối khi có tin nhắn mới
  - Input field + nút "🙏 Gửi" (Enter để gửi)
  - Disabled input khi chưa kết nối / chưa tham gia nhóm / chưa đăng nhập
  - Hiển thị trạng thái kết nối: "đang kết nối…", "● đã kết nối", "● N người online", lỗi
  - Validation: maxlength=500, không gửi tin rỗng
  - Gợi ý: "Live Chat chỉ để giao lưu. Nội dung quý giá nên tạo thành Chủ Đề"
- **Alpine.js `liveChat(opts)` component** trong `static/js/app.js`:
  - State: `messages`, `draft`, `connected`, `error`, `socket`, `reconnectAttempts`
  - `connect()`: mở WebSocket với URL `wss://host/ws/cong-dong/nhom/{slug}` (production) hoặc `ws://` (dev)
  - `handleIncoming(raw)`: parse JSON, xử lý 2 loại payload:
    - `{ type: "error", message: "..." }` — error từ server, hiển thị 3s
    - `{ id, body, author_display_name, ... }` — chat message, thêm vào `messages` (tránh duplicate)
  - `send()`: gửi text qua `socket.send()`, validate length, clear draft
  - `scheduleReconnect()`: exponential backoff (1s, 2s, 4s, 8s, 16s — max 30s, max 5 lần thử)
  - `onclose` handler: code 1008 = policy violation (auth/permission) → không reconnect; khác → reconnect
  - `formatTime(isoStr)`: format `dd/MM HH:mm` bằng `Intl.DateTimeFormat('vi-VN')`
  - `scrollToBottom()`: auto-scroll khi có tin nhắn mới
- **`AppState` thêm field `chat_hub: ChatHub`** — ChatHub clone-able (Arc inside), share giữa các handler.

### Sửa
- **`Cargo.toml`**: 
  - Bump version `0.9.1` → `0.9.2`
  - Thêm feature `"ws"` cho `axum` (cần thiết cho `axum::extract::ws::WebSocketUpgrade`)
- **`src/main.rs`**:
  - Import `handlers::chat::ChatHub`, thêm field `chat_hub: ChatHub::default()` vào AppState
  - Log khởi động: v0.9.1 → v0.9.2, đổi thông điệp thành "Giai đoạn 7: Live Chat WebSocket trong Nhóm + Cộng Đồng Foundation"
  - Health endpoint: `version: 0.9.2`, `phase: 7`, `phase_name: "Giai đoạn 7 — Live Chat WebSocket trong Nhóm"`, thêm mảng `features` liệt kê 5 tính năng chính
  - Thêm 2 routes mới: `GET /ws/cong-dong/nhom/{slug}` + `GET /api/cong-dong/nhom/{slug}/chat-history`
- **`src/handlers/mod.rs`**: export thêm `pub mod chat`
- **`src/handlers/community.rs`**: 
  - `GroupTemplate` thêm field `chat_messages_json: String` (JSON-serialised cho Alpine.js init)
  - `view_group` handler gọi `recent_messages()` để lấy 20 tin gần nhất, serialize sang JSON, truyền vào template
- **`src/models/community.rs`**: thêm `ChatMessage` + `ChatMessageWithAuthor` structs + impl helpers `time_ago()` + `author_initial()`

### Đổi
- **README.md**: cập nhật version v0.9.1 → v0.9.2, thêm 2 routes mới vào bảng routes, thêm mục "Giai đoạn 7: Live Chat WebSocket" vào lộ trình, cập nhật Cargo.toml description (thêm feature ws).
- **CHANGELOG.md**: thêm mục v0.9.2.
- **Footer version** trong `layout.html` + `placeholder_page()`: giữ nguyên "v0.9" (số minor hiển thị — chi tiết version nằm trong health endpoint + CHANGELOG).

### Lộ trình tiếp theo (v0.10+)
- **v0.10**: Quỹ Từ Bi + Thương Thành (placeholder hiện có)
- **v0.11**: Ghim/khoá chủ đề, sticky topics, pagination
- **v0.12**: Nested comments (reply tree), vote/like chủ đề và bình luận
- **v0.15**: Bạn Bè — kết bạn, nhắn tin, gửi thư
- **v0.17**: Kinh Sách — thư viện sách điện tử
- **v0.19**: Bảng Xếp Hạng + Thành tích

---

## [0.9.1] — 2026-08-14 — Giai đoạn 1 finalization: Fix UI mobile + Bỏ GitHub Actions + Deploy thủ công qua Coolify

### Sửa lỗi UI trên mobile
- **Fix logo hoa sen chưa căn giữa chuẩn trên bottom nav mobile**: Đổi layout từ `flex justify-around` sang `grid grid-cols-5 items-end` — 5 cột bằng nhau, mỗi mục chiếm đúng 1/5 chiều rộng, logo hoa sen nằm chính giữa cột thứ 3. Trước đó dùng `flex justify-around` khiến logo bị lệch do các mục có chiều rộng khác nhau (4 mục có text label, 1 mục chỉ icon).
- **Fix bottom nav che content trên mobile (đặc biệt ở trang Cộng Đồng)**: Thêm class CSS `.main-with-bottom-nav` áp dụng `padding-bottom: calc(4rem + env(safe-area-inset-bottom, 0px))` cho `<main>` trên mobile (md trở xuống). Desktop không bị ảnh hưởng (padding: 0). Có thêm safe-area-inset-bottom cho iPhone notch / home indicator.
- **Fix hamburger menu flicker trên trang chủ**: Thêm `x-cloak` attribute vào mobile menu div trong `layout.html` và 5 element dùng `x-show="show"` trong `home.html` (lotus animation, h1 title, 2 paragraphs, button container). Thêm CSS rule `[x-cloak] { display: none !important; }` trong `<head>` của layout. Trước Alpine.js init, element có `x-cloak` bị ẩn → không còn flash "hiện rồi biến mất".

### Bỏ GitHub Actions — chuyển sang deploy thủ công qua Coolify
- **Xóa hoàn toàn `.github/workflows/docker.yml`** (và thư mục `.github/workflows/`).
- **Lý do**: Workflow cũ gặp nhiều vấn đề — permission_denied khi push GHCR, webhook trả 302 redirect, double trigger. Deploy trực tiếp qua Coolify API đơn giản hơn và không cần GHCR.
- **Coolify app đã cấu hình sẵn** inline Dockerfile (clone repo + build với Rust 1.97.1 trên sub VPS 10.187.247.3). App UUID: `xsrqp8xrcwwk57dvtcwt6393`, domain: `tubi.louis.vangioitutien.com`, status: `running:healthy`.
- **Quy trình deploy mới**:
  1. Push code lên branch `main`
  2. Trigger Coolify deploy qua Web UI hoặc `GET /api/v1/applications/{uuid}/start` với Bearer token
  3. Coolify pull source → build Docker image Rust 1.97.1 → redeploy → Traefik + Let's Encrypt SSL

### Cập nhật
- **README.md**: Thay thế section "Production (qua Coolify + GitHub Actions)" bằng "Production (Deploy thủ công qua Coolify trên sub VPS)" với hướng dẫn chi tiết 2 cách trigger deploy (Web UI + API). Cập nhật bảng công nghệ: bỏ dòng CI/CD, thêm dòng Deploy. Cập nhật cấu trúc dự án: bỏ `.github/workflows/`, ghi chú Dockerfile chỉ dùng cho local/dev.
- **CSS `app.css`**: Thêm class `.main-with-bottom-nav` với `padding-bottom: calc(4rem + env(safe-area-inset-bottom, 0px))` và media query md+ bỏ padding.
- **`templates/layout.html`**: Thêm `<style>[x-cloak] { display: none !important; }</style>` trong `<head>`, thêm `x-cloak` vào mobile menu div, đổi bottom nav từ flex sang grid-cols-5, thêm class `.main-with-bottom-nav` cho `<main>`.
- **`templates/home.html`**: Thêm `x-cloak` vào 5 element dùng `x-show="show"` trong hero section.

---

## [0.9.1-pre] — 2026-08-14 — Fix GitHub Actions CI/CD (đã bị thay thế bởi bản v0.9.1 final phía trên)

### Sửa (đã bị revert)
- **Fix GitHub Actions CI/CD**: Workflow cũ bị lỗi `permission_denied: write_package` khi push Docker image lên GHCR
  - Thêm top-level `permissions` (contents:read, packages:write) cho GITHUB_TOKEN
  - Bỏ trigger `push branches:main` — chỉ giữ tag trigger để tránh double run
  - Sửa secrets syntax trong `trigger-coolify` job (dùng `env.` thay vì `secrets.` trong if condition)
  - Thêm `provenance: false` cho docker/build-push-action
- **Chuyển sang Coolify-native deploy strategy**: Thay vì build image trên GitHub Actions → push GHCR → Coolify pull, giờ Coolify tự build trên VPS từ source
- **Thêm COOLIFY_API_TOKEN secret** cho Coolify API fallback trong workflow
- **Cập nhật RUST_LOG** trên Coolify: `actix_web=info` → `axum=info,tower_http=info` (Axum 0.8 migration)
- **Deploy thủ công** lên `tubi.louis.vangioitutien.com` thành công qua Coolify API

---

## [0.9.0] — 2026-08-14 — Ứng Dụng Từ Bi v0.9: Codebase sạch lỗi, Axum 0.8 ổn định

### Sửa
- **Fix triệt để 126 clippy warnings (pedantic + nursery)** — toàn bộ codebase giờ pass `cargo clippy --all-targets --all-features -- -D warnings -D clippy::all -D clippy::pedantic -D clippy::nursery` với 0 lỗi.
  - `src/config.rs`: `doc_markdown`, `map_unwrap_or` → `is_ok_and`/`map_or_else`, `manual_assert`, `use_self`
  - `src/errors/mod.rs`: `use_self`, `match_same_arms`
  - `src/handlers/auth.rs`: `doc_markdown`, `format_collect`, `redundant_closure`, `items_after_statements`, `redundant_clone`, `uninlined_format_args`, `too_many_lines` (extracted `build_session_redirect_response`), `let_and_return`
  - `src/handlers/community.rs`: `doc_markdown`, `manual_let_else` (7 instances), `map_unwrap_or`, `too_many_lines` (extracted `render_create_group_error` + `insert_group_with_owner`), `too_many_arguments`, `ref_option`
  - `src/handlers/uploads.rs`: `manual_let_else`, `unnecessary_debug_formatting`, `redundant_closure`, `option_if_let_else`, `format_push_string` → `write!`, `uninlined_format_args`, `cast_possible_wrap`, `cast_lossless`, `too_many_lines` (extracted 4 helpers), `items_after_statements`
  - `src/handlers/mod.rs`: `doc_markdown`, `manual_let_else`, `ref_option`, `needless_pass_by_value`, `option_if_let_else`, `too_many_lines`
  - `src/main.rs`: `uninlined_format_args`, `unnecessary_debug_formatting`, `map_unwrap_or` → `is_ok_and`, `ignored_unit_patterns`, `duration_suboptimal_units`, `option_if_let_else`
  - `src/models/user.rs`: `match_same_arms`, `missing_const_for_fn`, `doc_markdown`
  - `src/models/community.rs`: `doc_markdown`
- **Bump version**: 0.7.0 → 0.9.0

  - `Cargo.toml`: version `0.9.0`
  - `main.rs` health endpoint: `version: 0.9.0`, `phase: 9`, `phase_name` cập nhật
  - `templates/layout.html` footer: v0.9
  - `handlers/mod.rs` placeholder_page footer: v0.9

### Giữ nguyên
- **Toàn bộ logic nghiệp vụ** — không thay đổi bất kỳ chức năng nào.
- **Toàn bộ SQL queries** — không thay đổi schema hay migration files.
- **Toàn bộ Askama templates** — chỉ cập nhật số version trong footer.
- **Toàn bộ model structs** — không đổi.
- **Yêu cầu Rust 1.97.1** — `Cargo.toml` vẫn `rust-version = "1.97"`.
- **API surface** — tất cả endpoint giữ nguyên hoàn toàn.

---

## [0.7.0] — 2026-08-14 — Migration Actix-web → Axum (giữ nguyên feature Cộng Đồng)

### Thay đổi
- **Migration toàn bộ backend từ Actix-web 4 sang Axum 0.8** — cùng họ tower-ecosystem, async-native trên tokio.
  - `Cargo.toml`: thay `actix-web`, `actix-files`, `actix-multipart` bằng `axum 0.8` (features `macros`, `multipart`), `axum-extra 0.10` (features `cookie`, `cookie-signed`), `tower 0.5`, `tower-http 0.6` (features `fs`, `trace`, `compression-gzip`, `cors`).
  - Thêm dependency tường minh `tokio = { version = "1", features = ["full"] }` (trước đó được actix-web kéo về).
  - Tăng version `bytes` từ `0.5` (cũ, đi kèm actix) lên `1` (chuẩn tower/axum).
- **`src/main.rs`** — viết lại bằng `axum::Router` + `axum::serve` + `tokio::net::TcpListener`:
  - `AppState` struct (clone) thay cho `web::Data<T>` — chứa `PgPool` + `Arc<Config>`.
  - Routes khai báo bằng `.route(path, get(handler).post(handler2))` thay cho `web::get().to(...)` / `web::post().to(...)`.
  - Static files qua `tower_http::services::ServeDir::new(static_dir).nest_service("/static", ...)`.
  - Logger qua `tower_http::trace::TraceLayer`, nén gzip qua `tower_http::compression::CompressionLayer`.
  - `actix_web::rt::spawn` → `tokio::spawn`; `actix_web::rt::time::interval` → `tokio::time::interval`.
  - Graceful shutdown bằng `axum::serve(listener, app).with_graceful_shutdown(...)` lắng nghe Ctrl+C + SIGTERM.
  - `#[actix_web::main]` → `#[tokio::main]`.
- **`src/errors/mod.rs`** — `ResponseError::error_response` → `axum::response::IntoResponse::into_response`; `HttpResponse` → `(StatusCode, Json(...))`.
- **`src/handlers/mod.rs`** — convert tất cả handler:
  - `web::Data<PgPool>` → `State<AppState>`; `HttpRequest` + `req.cookie("session_id")` → `CookieJar` + `jar.get("session_id")`.
  - `impl Responder` → `Response` (dùng `Html(...).into_response()`, `Redirect::to(...).into_response()`, tuple `(StatusCode, &str).into_response()`).
  - `web::Form<T>` → `axum::Form<T>`.
- **`src/handlers/auth.rs`** — viết lại OAuth flow:
  - Cookie build/parse dùng `axum_extra::cookie::{Cookie, SameSite}` (backend là crate `cookie` nhưng API gọi khác actix).
  - Set/Clear cookies trong response bằng cách append header `SET_COOKIE` thủ công (vì `CookieJar` chỉ thêm cookie vào request outbound, không tự set response).
  - Query string `?next=...` trích bằng `axum::extract::Query<LoginQuery>` thay vì `req.query_string().split('&')`.
  - `web::Query<T>` → `axum::extract::Query<T>`.
- **`src/handlers/community.rs`** — convert 10 endpoint Cộng Đồng (giữ nguyên logic nghiệp vụ, chỉ đổi framework API):
  - `web::Path<String>` → `axum::extract::Path<String>`.
  - `HttpResponse::Found().append_header(("Location", ...)).finish()` → `Redirect::to(&url).into_response()`.
  - `HttpResponse::NotFound().body(...)` → `(StatusCode::NOT_FOUND, "...").into_response()`.
- **`src/handlers/uploads.rs`** — convert multipart:
  - `actix_multipart::Multipart` → `axum::extract::Multipart`.
  - `field.next()` (actix) → `field.bytes().await` (axum đọc cả field một lần).
  - `field.content_disposition()`, `field.content_type()` → `field.name()`, `field.file_name()`, `field.content_type()` (trực tiếp, không qua `ContentDisposition` struct).
  - `bytes::BytesMut` → `bytes::Bytes` (qua Vec trung gian) cho simplify.
- **`Dockerfile`** — cập nhật `RUST_LOG`: bỏ `actix_web=info`, thêm `axum=info,tower_http=info`.
- **`.env.example`** — cập nhật `RUST_LOG` tương tự.
- **`README.md`** — cập nhật bảng công nghệ: `Actix-web` → `Axum 0.8`; cập nhật Giai đoạn 1 mô tả.
- **`Cargo.lock`** — regenerated sau `cargo build`.

### Giữ nguyên
- **Toàn bộ logic nghiệp vụ** — Google OAuth flow, session management, member ranks, Cộng Đồng (Nhóm + Chủ Đề + Bình luận), upload ảnh (multipart + SHA-256 + dedup), auto-migrations, graceful shutdown.
- **Toàn bộ SQL queries** — không thay đổi schema, không thay đổi migration files.
- **Toàn bộ Askama templates** — không thay đổi HTML/CSS/JS.
- **Toàn bộ model structs** — `User`, `MemberRank`, `Group`, `Topic`, `Comment`, ... không đổi.
- **Yêu cầu Rust 1.97.1** — `Cargo.toml` vẫn `rust-version = "1.97"`; `Dockerfile` vẫn `rust:1.97.1-slim-bookworm`.
- **API surface** — tất cả endpoint (path + method) giữ nguyên hoàn toàn.

### Lỗi đã sửa trong quá trình migration
- Cookie build API khác nhau giữa actix (`Cookie::build((name, value))` đã OK từ cookie 0.16+) và axum-extra (dùng cùng crate `cookie` mới hơn) — chọn API builder tuple mới.
- `Redirect` của axum không tự mang cookie headers — phải append `SET_COOKIE` thủ công vào `Response` sau khi `Redirect::to(...).into_response()`.
- `axum::extract::Multipart::next_field` trả `Result<Option<Field>, _>` thay vì actix `Stream<Item=Result<Field, _>>` — đổi loop pattern.
- `field.bytes()` đọc toàn bộ field một lần — không cần inner `while let Some(chunk) = field.next().await` như actix.
- `tower_http::services::ServeDir` không có method `.show_files_listing()` mặc định — hành vi "không list thư mục" đã đúng với actix-files cũ, khớp với yêu cầu.

---

## [0.6.0] — 2026-08-14 — Giai đoạn 6: Cộng Đồng Foundation (Nhóm + Chủ Đề + Bình luận)

### Thêm
- **Chuyên mục Cộng Đồng chính thức ra mắt** — `/cong-dong` giờ là trang thật (không còn placeholder).
  - Trang chính hiển thị danh sách nhóm công khai + chủ đề mới nhất (Lướt Nhóm / Lướt Chủ Đề tabs)
  - Hero banner với nút "Tạo Nhóm Mới" (auth) hoặc "Đăng nhập để tham gia" (guest)
- **Hệ thống Nhóm (Groups)** — đơn vị tổ chức cộng đồng:
  - Tạo nhóm: tên (bắt buộc), mô tả, phân loại, visibility (public/private/hidden), require_approval
  - Trang nhóm `/cong-dong/nhom/{slug}` hiển thị thông tin + danh sách chủ đề
  - Tham gia / rời nhóm (POST-only để chống CSRF)
  - Slug tự sinh từ tên (loại bỏ dấu tiếng Việt, collapse dashes)
  - Owner tự động trở thành `group_members.role = 'owner'`
  - Owner không thể rời nhóm (phải chuyển quyền trước)
- **Hệ thống Chủ Đề (Topics)** — bài viết trong nhóm (diễn đàn):
  - Tạo chủ đề: title (bắt buộc, max 200 ký tự), body (bắt buộc, hỗ trợ multiline)
  - Chỉ thành viên active của nhóm mới được tạo chủ đề
  - Chủ đề hiển thị: tiêu đề, body, author (avatar + tên + cấp bậc), thời gian tương đối
  - Hỗ trợ ghim (`is_pinned`) và khoá (`is_locked`) — schema sẵn sàng, UI ở giai đoạn sau
  - Tự tăng `view_count` mỗi lần xem trang chủ đề
- **Hệ thống Bình luận (Comments)** — bình luận trên chủ đề:
  - Form bình luận nhanh ngay trên trang chủ đề
  - Hỗ trợ reply (parent_id) — schema sẵn sàng, UI nested ở giai đoạn sau
  - Validate body (không rỗng, tối đa 5000 ký tự)
  - Không bình luận được nếu chủ đề bị khoá (`is_locked`)
- **Phân loại nhóm (group_categories)** — 9 phân loại mặc định:
  - Tu Học · Niệm Phật · Kinh Sách · Thiền Định · Pháp Thoại · Chia Sẻ · Thiện Nguyện · Âm Nhạc · Khác
- **Migration 005**: 5 bảng mới + 4 triggers + 9 seed categories
  - `group_categories` (slug, name, icon, sort_order)
  - `groups` (id, slug, name, description, category_id, owner_id, cover_upload_id, visibility, require_approval, member_count, topic_count, is_active, timestamps)
  - `group_members` (group_id, user_id, role, status, joined_at) — UNIQUE(group_id, user_id)
  - `topics` (id, group_id, author_id, title, body, is_pinned, is_locked, comment_count, view_count, is_active, timestamps)
  - `comments` (id, topic_id, author_id, parent_id, body, is_active, timestamps)
  - Trigger `trg_*_set_updated_at` — tự cập nhật `updated_at` cho groups/topics/comments
  - Trigger `trg_*_count` — tự cập nhật `member_count`, `topic_count`, `comment_count` khi INSERT/DELETE
- **Models `community.rs`** — 8 struct:
  - `Group`, `GroupWithCategory`, `Topic`, `TopicWithAuthor`, `Comment`, `CommentWithAuthor`, `GroupMember`, `GroupCategory`
  - Form structs: `GroupCreateForm`, `TopicCreateForm`, `CommentCreateForm`
  - Helper methods: `visibility_display()`, `visibility_icon()`, `category_icon_or_lotus()`, `category_name_or_other()`, `time_ago()`, `body_excerpt()`, `author_initial()`, `role_display()`, `role_icon()`, `is_staff()`
- **Handlers `community.rs`** — 10 endpoint:
  - `GET /cong-dong` — trang chính (list groups + hot topics)
  - `GET /cong-dong/tao-nhom` — form tạo nhóm (auth)
  - `POST /cong-dong/tao-nhom` — tạo nhóm mới (auth, transaction)
  - `GET /cong-dong/nhom/{slug}` — xem nhóm + danh sách chủ đề
  - `POST /cong-dong/nhom/{slug}/tham-gia` — tham gia nhóm (auth, ON CONFLICT DO NOTHING)
  - `POST /cong-dong/nhom/{slug}/roi-khoi` — rời nhóm (auth, owner không được rời)
  - `GET /cong-dong/nhom/{slug}/tao-chu-de` — form tạo chủ đề (auth + member)
  - `POST /cong-dong/nhom/{slug}/tao-chu-de` — tạo chủ đề (auth + member)
  - `GET /cong-dong/chu-de/{id}` — xem chủ đề + bình luận
  - `POST /cong-dong/chu-de/{id}/binh-luan` — đăng bình luận (auth)
- **Templates `templates/community/`** — 5 template Askama:
  - `index.html` — trang chính với tabs Lướt Nhóm / Lướt Chủ Đề
  - `group.html` — trang nhóm với topic list + action buttons (tham gia/rời/tạo chủ đề)
  - `topic.html` — trang chủ đề với comment form + comment list
  - `create_group.html` — form tạo nhóm với visibility radio + category select
  - `create_topic.html` — form tạo chủ đề
- **Helper `time_ago_display()`** — format thời gian tương đối tiếng Việt ("vừa xong", "5 phút trước", "2 ngày trước", ...)
- **Helper `slugify()`** — tạo slug từ tên tiếng Việt (loại bỏ dấu, thay whitespace bằng `-`)
- **Helper `ensure_unique_slug()`** — thêm hậu tố UUID 6 ký tự nếu slug trùng
- **Helper `fetch_categories()`** — lấy danh sách category theo sort_order
- **Helper `get_membership()`** — kiểm tra user có phải thành viên nhóm không

### Sửa
- **[REFACTOR] Handler `cong_dong` trong `handlers/mod.rs`** — không còn dùng `placeholder_page`. Nay delegate cho `community::cong_dong_index`.
- **[DRY] `placeholder_page`** vẫn được dùng cho các chuyên mục chưa phát triển (Không Gian, Bạn Bè, Kinh Sách, Quỹ Từ Bi, Thương Thành, Bảng Xếp Hạng).

### Đổi
- **`Cargo.toml`**: version `0.5.0` → `0.6.0`
- **`main.rs`**:
  - Log khởi động: v0.5 → v0.6, đổi phase_name thành "Cộng Đồng Foundation"
  - Health endpoint: `version: 0.6.0`, `phase: 6`, `phase_name: "Cộng Đồng Foundation — Nhóm + Chủ Đề + Bình luận"`
  - Thêm 8 routes mới cho Cộng Đồng
- **`templates/layout.html`**: footer v0.5 → v0.6
- **`src/handlers/mod.rs`**: footer placeholder_page v0.5 → v0.6
- **`src/models/mod.rs`**: export thêm `community` module
- **`src/handlers/mod.rs`**: export thêm `community` module

### Bump version
- `Cargo.toml`: 0.5.0 → 0.6.0
- `main.rs` health endpoint: 0.5.0 → 0.6.0, phase 5 → 6
- `templates/layout.html` footer: v0.5 → v0.6
- `README.md`: cập nhật routes + version v0.6

### Lộ trình tiếp theo (v0.7+)
- **v0.7**: Live Chat thời gian thực (WebSocket) trong nhóm
- **v0.8**: Ghim/khoá chủ đề, sticky topics, pagination
- **v0.9**: Nested comments (reply tree), vote/like chủ đề và bình luận
- **v0.10**: Quỹ Từ Bi + Thương Thành
- **v0.12**: Cộng Đồng hoàn thiện (theo placeholder): pagination, search, filter, mod tools

---

## [0.5.0] — 2026-08-14 — Giai đoạn 5: Hạ tầng deploy (Docker + GitHub Actions + Coolify) + storage ảnh

### Thêm
- **Dockerfile multi-stage** với Rust 1.97.1, image final ~30 MB (glibc + stripped binary + static assets)
  - Stage 1 (builder): `rust:1.97.1-slim-bookworm` + pkg-config/libssl-dev/libpq-dev
  - Stage 2 (runtime): `debian:bookworm-slim` + libssl3/libpq5/ca-certificates/curl/tini
  - Run as non-root user `tubi` (UID 1001)
  - HEALTHCHECK bằng `curl /api/health`
  - ENTRYPOINT bằng `tini` để xử lý signals đúng (graceful shutdown)
- **`.dockerignore`** — loại bỏ target/, .git, .env, docs khỏi Docker context
- **`docker-compose.yml`** cho dev — Postgres 17-alpine + app cùng lúc
- **GitHub Actions workflow** `.github/workflows/docker.yml`:
  - Trigger: push tag `v*` hoặc push lên main (filter path)
  - Build & push Docker image lên `ghcr.io/mhieuhonda/ungdungtubi` với tags: version, `latest`, SHA
  - Cache build qua `cache-from/cache-to: type=gha`
  - Trigger Coolify webhook sau khi push xong (lấy URL từ secret `COOLIFY_WEBHOOK_URL`)
- **Migration 004**: bảng `images` (id, uploader_id, original_name, stored_filename, mime_type, size_bytes, sha256, width, height, purpose, is_public, created_at) + bảng `audit_log` (append-only, ghi lại mọi giao dịch quan trọng) + trigger `trigger_set_updated_at` tự cập nhật `updated_at` khi UPDATE users
- **API upload ảnh** `POST /api/upload-image` (form-data field `file`):
  - Giới hạn 5 MB/ảnh (configurable qua `MAX_UPLOAD_BYTES`)
  - Chỉ chấp nhận JPEG, PNG, WebP, GIF
  - Tính SHA-256 để chống trùng lặp (cùng user upload lại ảnh cũ → trả về URL cũ)
  - Parse width/height từ header ảnh (PNG/JPEG/GIF/WebP)
  - Lưu file vào `upload_dir` với tên `<uuid>.<ext>`
  - Trả về JSON `{ id, url, size, mime_type, width, height, sha256 }`
- **`GET /api/upload-info`** — trả về giới hạn upload + danh sách MIME types cho phép
- **Auto-run migrations** khi khởi động (set `RUN_MIGRATIONS=true` hoặc `APP_ENV=production` tự động bật)
- **Health check cải tiến** — giờ check cả DB (`SELECT version()`) và trả về `database.status` + `database.version`
- **Graceful shutdown** — `shutdown_timeout(30s)`, 4 workers
- **DB pool size tunable** qua env `DB_MAX_CONNECTIONS` (mặc định 10)
- **Static dir + upload dir configurable** qua env `STATIC_DIR` / `UPLOAD_DIR` / `UPLOAD_URL_PREFIX` (cho Docker volume mapping)
- **Release profile tối ưu**: `opt-level=3`, `lto="thin"`, `codegen-units=1`, `strip="symbols"`, `panic="abort"` (binary nhỏ + chạy nhanh)

### Sửa
- **[SECURITY] Bỏ GET `/dang-xuat`** — chỉ còn POST để chống CSRF (kẻ tấn công không thể nhúng `<img src="/dang-xuat">` để log user ra ngoài)
- **[SECURITY] Logout form dùng JavaScript submit** thay vì link GET trong `layout.html` và `placeholder_page()` — giữ UX click như cũ nhưng an toàn
- **[BUG] `static_dir` dùng relative path** `src/static` — khi chạy trong Docker với working directory khác, không tìm thấy static files. Nay lấy từ env `STATIC_DIR` (Docker set `/app/static`)
- **[BUG] `USER_COLUMNS` thiếu `avatar_upload_id`** — sau migration 004 thêm cột này, `SELECT ... RETURNING` sẽ fail nếu không liệt kê. Đã thêm vào cả `handlers::USER_COLUMNS` và `handlers::auth::USER_COLUMNS`
- **[BUG] `render_user_menu_html` dùng `format!` với `'` trong string** — Rust 2024 format macro strict về `'` trong string literals. Đổi sang `push_str` để tránh vấn đề này
- **[CLEANUP] `fs::Files::new(...).disable_listing()` không tồn tại** — actix-files mặc định đã không list thư mục. Bỏ method không tồn tại này
- **[CLEANUP] `let _ = u` trong `render_mobile_user_menu_html`** — clippy warn, đã bỏ

### Đổi
- **`Cargo.toml`**:
  - version `0.4.0` → `0.5.0`
  - Thêm dependencies: `actix-multipart` (upload), `sha2` (checksum), `bytes` (stream), `mime` (validation), `futures-util` (stream)
  - Thêm `[profile.release]` tối ưu
- **`src/config.rs`**:
  - Thêm fields: `static_dir`, `upload_dir`, `max_upload_bytes`, `db_max_connections`, `upload_url_prefix`
  - Tất cả đều lấy từ env với default hợp lý cho cả dev và Docker
- **`src/main.rs`**:
  - Auto-migrate với `sqlx::migrate!`
  - DB ping khi khởi động (log PostgreSQL version)
  - Tạo `upload_dir` nếu chưa tồn tại
  - Đăng ký routes `/api/upload-info`, `/api/upload-image`
  - Health check giờ nhận `pool` và check DB
  - `workers(4)`, `shutdown_timeout(30)`
- **`src/handlers/mod.rs`**:
  - Export `pub const USER_COLUMNS` và `pub async fn get_user_from_session` cho handler uploads dùng
  - Thêm `pub mod uploads;`
- **`src/handlers/auth.rs`**: đồng bộ `USER_COLUMNS` (thêm `avatar_upload_id`)
- **`src/models/user.rs`**: thêm field `avatar_upload_id: Option<Uuid>`
- **`templates/layout.html`**: logout dùng JavaScript submit thay vì GET link
- **`.env.example`**: thêm các biến mới (`STATIC_DIR`, `UPLOAD_DIR`, `UPLOAD_URL_PREFIX`, `MAX_UPLOAD_BYTES`, `DB_MAX_CONNECTIONS`, `RUN_MIGRATIONS`)
- **`README.md`**: cập nhật cho v0.5 — thêm Docker + GitHub Actions + Coolify workflow
- **`CHANGELOG.md`**: thêm mục v0.5

### Bump version
- `Cargo.toml`: 0.4.0 → 0.5.0
- `main.rs` health endpoint: 0.4.0 → 0.5.0, thêm `phase: 5` + `phase_name` + `database` object
- `templates/layout.html` footer: v0.4 → v0.5

---

## [0.4.0] — 2026-08-13 — Giai đoạn 4: Hồ sơ thành viên & Hệ thống cấp bậc

### Thêm
- **Trang hồ sơ cá nhân** `/ca-nhan` với:
  - Avatar từ Google (hoặc chữ cái đầu tên nếu không có)
  - Hiển thị cấp bậc, màu sắc, icon riêng cho từng cấp
  - Thống kê Niệm Lực A, Tiền K, giới tính
  - Pháp danh, pháp hiệu, bút danh, tiểu sử
- **Form chỉnh sửa hồ sơ** với các trường:
  - Tên hiển thị (bắt buộc, tối đa 100 ký tự)
  - Pháp danh (tùy chọn)
  - Pháp hiệu (tùy chọn)
  - Bút danh (tùy chọn)
  - Giới tính (Nam/Nữ/Khác)
  - Tiểu sử (tùy chọn, tối đa 500 ký tự)
- **Endpoint POST `/ca-nhan/cap-nhat`** — cập nhật hồ sơ an toàn:
  - Validate input (tên không rỗng, gender thuộc enum)
  - Không cho phép sửa email, rank, số dư A/K, is_active
  - Chuẩn hoá optional fields (None nếu rỗng)
- **Hệ thống 9 cấp bậc** với bảng `member_ranks`:
  - 🌱 Người Mới (0 K) → 👑 Đại Gia (100.000 K)
  - Mỗi cấp bậc có: tên, mô tả, màu hex, emoji, min_k_balance
  - Hiển thị tiến độ cấp bậc trên trang profile
- **Migration 003**: thêm cột `phap_danh`, `phap_hieu`, `but_danh`, `gender`, `bio` vào `users`; tạo bảng `member_ranks` + seed 9 cấp bậc mặc định
- **Hiển thị icon + tên cấp bậc** trên header (gần tên user, có tooltip)
- **Model `MemberRank`** + helper `ProfileUpdate` cho form data
- **Helper methods trên User**:
  - `rank_display()` — tên tiếng Việt cấp bậc
  - `rank_icon()` — emoji tương ứng
  - `rank_color()` — mã màu hex
  - `display_label()` — tên hiển thị ưu tiên theo pháp danh > pháp hiệu > bút danh > display_name
  - `gender_display()` — Nam/Nữ/Khác
- **Document `USER_COLUMNS`** — danh sách cột cố định để tránh drift giữa các SQL queries

### Sửa
- **[BUG] Login button trong `login.html` trỏ sai endpoint** — trước đây trỏ về `/dang-nhap` (GET render lại template), nay trỏ đúng `/auth/google` để trigger OAuth flow
- **[BUG] Mobile menu trong `layout.html` thiếu nút "Hồ sơ/Thoát" khi đã đăng nhập** — nay hiển thị đầy đủ cả 2 trạng thái
- **[CLEANUP] Migration 002 có chữ Hán "映射" sót lại** — đổi thành "ánh xạ" tiếng Việt
- **[DRY] SQL queries trong `auth.rs` và `handlers/mod.rs`** dùng chung `USER_COLUMNS` constant thay vì lặp lại danh sách cột
- **[SECURITY] Logout endpoint** — đã có POST (chuẩn CSRF-safe), giữ GET cho link đơn giản
- **[CODE] Loại bỏ dependency `actix-rt` không dùng trực tiếp** (actix-web đã re-export)

### Bump version
- `Cargo.toml`: 0.3.0 → 0.4.0
- `main.rs` health endpoint: 0.3.0 → 0.4.0, thêm `phase: 4` + `phase_name`
- `README.md`, `templates/layout.html`, `.env.example`: cập nhật nhãn v0.4

---

## [0.3.0] — Giai đoạn 3: Chuyển sang Google OAuth

### Thêm
- Tích hợp Google OAuth 2.0 (Authorization Code Flow)
- Endpoint `/auth/google` + `/auth/google/callback`
- Migration 002: `password_hash` NULL được, thêm `google_sub`, `avatar_url`, `email_verified`
- State chống CSRF (cookie HttpOnly, TTL 10 phút)
- Auto-link tài khoản Google với tài khoản cũ (theo email)
- Bỏ hoàn toàn form đăng ký email/password

---

## [0.2.0] — Giai đoạn 2: Hệ thống xác thực email/password

### Thêm
- Form đăng ký thành viên (email, mật khẩu, tên hiển thị)
- Đăng nhập (email + password)
- Session management (cookie-based, SQLx session store)
- Logout & bảo vệ route
- Migration 001: bảng `users`, `sessions`

---

## [0.1.0] — Giai đoạn 1: Nền móng hạ tầng cốt lõi

### Thêm
- Khởi tạo project Rust (Actix-web + Askama + SQLx + PostgreSQL)
- Cấu hình HTMX + Alpine.js + Tailwind CSS
- Trang landing page / trang chủ
- Hệ thống template layout (header, footer, navigation)
- Cấu hình domain `tubi.louis.vangioitutien.com`

---

*Nguyện công đức vô lượng. Nam Mô A Di Đà Phật.* 🪷
