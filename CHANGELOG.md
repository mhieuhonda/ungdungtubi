# Changelog — Ứng Dụng Từ Bi

Tất cả thay đổi đáng chú ý của dự án sẽ được ghi lại trong file này.
Định dạng dựa trên [Keep a Changelog](https://keepachangelog.com/vi/1.1.0/),
tuân thủ [Semantic Versioning](https://semver.org/lang/vi/).

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
