# Changelog — Ứng Dụng Từ Bi

Tất cả thay đổi đáng chú ý của dự án sẽ được ghi lại trong file này.
Định dạng dựa trên [Keep a Changelog](https://keepachangelog.com/vi/1.1.0/),
tuân thủ [Semantic Versioning](https://semver.org/lang/vi/).

---

## [0.9.41] — 2026-08-17 — Giai đoạn 45: Admin Moderation Hoàn Thiện + Từ Vựng Cấm + Heartbeat Fix + Mobile Menu Compact + Music Submit Error Log 🪷

### 🎯 Mục tiêu giai đoạn

Bản phát hành này hoàn thiện **3 module admin còn sót** từ các giai đoạn trước,
fix **4 bug user báo cáo**, và cải thiện UX mobile menu:

1. **Hoàn thiện Quản lý Bình luận** — trước v0.9.41, trang `/admin/binh-luan`
   chỉ hiển thị placeholder "Module đang hoàn thiện" + read-only list (và list
   còn bị lỗi type mismatch UUID vs i64 → luôn rỗng).
2. **Hoàn thiện Quản lý Nhóm Cộng Đồng** — tương tự bình luận,
   `/admin/cong-dong/nhom` chỉ là placeholder.
3. **Hoàn thiện module Từ vựng cấm** — trước v0.9.41, nút "Từ vựng cấm" trong
   admin dashboard chỉ là anchor `#tu-vung-cam` (không có trang).
4. **Fix bug "hoạt động 6 giờ trước dù đang online"** — heartbeat client
   setInterval 10 phút, không fire ngay khi user login.
5. **Fix lỗi "không thể lưu bài hát vào cơ sở dữ liệu"** — error message chỉ
   nói chung chung, không có cách debug.
6. **Rút gọn mobile menu thêm** — vertical list với padding lớn → 2-column grid
   compact.

### 💬 Quản lý Bình luận — `/admin/binh-luan`

Trước v0.9.41, đây là placeholder read-only. User report: "Module Quản lý
Bình luận hiện đang ở giai đoạn hoàn thiện. Phiên bản v0.9.32 hiển thị danh
sách read-only bên dưới..." dù đã qua rất nhiều giai đoạn. Nguyên nhân root:
`AdminCommentRow.id` và `topic_id` declared là `i64`, nhưng DB `comments.id`
và `topics.id` là `UUID` → `sqlx::query_as` fail silently, `unwrap_or_else`
trả `vec![]` → list luôn rỗng. v0.9.41 thay bằng moderation UI đầy đủ:

- **[CMT-1]** `migrations/029_admin_moderation_and_forbidden_words.sql` — Thêm
  cột `comments.is_pinned BOOLEAN`, `comments.is_locked BOOLEAN`,
  `comments.moderation_status VARCHAR(20)` (pending/approved/rejected/flagged),
  `comments.moderated_by UUID`, `comments.moderated_at TIMESTAMPTZ`. Index cho
  pinned + moderation_status.
- **[CMT-2]** `src/db/mod.rs::ensure_schema_safety()` — Safety schema check
  idempotent cho 5 cột mới của comments + CHECK constraint (DO $$ BEGIN).
- **[CMT-3]** `src/handlers/admin.rs::AdminCommentModerationRow` — Row struct
  mới với `id: Uuid` + `topic_id: Uuid` (FIX type mismatch cũ), thêm
  `author_id`, `author_avatar`, `author_role`, `group_id`, `group_name`,
  `group_slug`, `is_pinned`, `is_locked`, `moderation_status`.
- **[CMT-4]** `src/handlers/admin.rs::fetch_admin_comments_moderation()` —
  Fetch 100 comments mới nhất JOIN users + topics + groups, ORDER BY
  `is_pinned DESC, created_at DESC` (pinned lên đầu).
- **[CMT-5]** `src/handlers/admin.rs::admin_binh_luan_list` — `GET
  /admin/binh-luan` — render `admin/binh-luan/index.html` với stats (visible,
  hidden, pinned, flagged) + list comments + actions.
- **[CMT-6]** `src/handlers/admin.rs::admin_binh_luan_hide` — `POST
  /admin/binh-luan/{id}/an` — Set `is_active=false, moderated_by, moderated_at`.
- **[CMT-7]** `src/handlers/admin.rs::admin_binh_luan_show` — `POST
  /admin/binh-luan/{id}/hien` — Set `is_active=true`.
- **[CMT-8]** `src/handlers/admin.rs::admin_binh_luan_delete` — `POST
  /admin/binh-luan/{id}/xoa` — Hard delete + update topic comment_count.
- **[CMT-9]** `src/handlers/admin.rs::admin_binh_luan_toggle_pin` — `POST
  /admin/binh-luan/{id}/ghim` — Toggle `is_pinned`.
- **[CMT-10]** `src/handlers/admin.rs::admin_binh_luan_toggle_lock` — `POST
  /admin/binh-luan/{id}/khoa` — Toggle `is_locked` (khoá nhánh trả lời).
- **[CMT-11]** `templates/admin/binh-luan/index.html` — Full moderation UI:
  header + banner "Module đã hoàn thiện — v0.9.41" + 4 stats cards + list
  comments với author avatar, role badge, topic link, group link, 4 action
  buttons (ẩn/hiện, ghim, khoá, xóa).
- **[CMT-12]** `src/main.rs` — Routes mới: 5 POST endpoints cho moderation
  actions. Replace `admin_binh_luan_placeholder` với `admin_binh_luan_list`.

### 🏛️ Quản lý Nhóm Cộng Đồng — `/admin/cong-dong/nhom`

Trước v0.9.41, đây là placeholder read-only. User report: "phần quản lí nhóm
cộng đồng cũng bị vậy" (cũng hiện placeholder). v0.9.41 thay bằng moderation
đầy đủ:

- **[NHOM-1]** Migration 029 — Thêm cột `groups.is_featured BOOLEAN`,
  `groups.moderation_status VARCHAR(20)`, `groups.moderated_by UUID`,
  `groups.moderated_at TIMESTAMPTZ`. Index cho featured.
- **[NHOM-2]** Safety schema check idempotent cho 4 cột mới của groups.
- **[NHOM-3]** `src/handlers/admin.rs::AdminGroupModerationRow` — Row struct
  mới với `is_featured`, `moderation_status`, `owner_name`, `owner_avatar`.
- **[NHOM-4]** `src/handlers/admin.rs::fetch_admin_groups_moderation()` —
  Fetch 100 groups JOIN users để lấy owner, ORDER BY `is_featured DESC,
  created_at DESC`.
- **[NHOM-5]** `src/handlers/admin.rs::admin_nhom_list` — `GET
  /admin/cong-dong/nhom` — render `admin/cong-dong/nhom.html` với stats
  (active, locked, featured) + list groups + actions.
- **[NHOM-6]** `src/handlers/admin.rs::admin_nhom_toggle_lock` — `POST
  /admin/cong-dong/nhom/{id}/khoa` — Toggle `is_active` (lock/unlock nhóm).
- **[NHOM-7]** `src/handlers/admin.rs::admin_nhom_toggle_featured` — `POST
  /admin/cong-dong/nhom/{id}/dac-biet` — Toggle `is_featured`.
- **[NHOM-8]** `src/handlers/admin.rs::admin_nhom_delete` — `POST
  /admin/cong-dong/nhom/{id}/xoa` — Soft delete (`is_active=false,
  moderation_status='rejected'`). Yêu cầu `is_admin()` (chỉ admin, không phải
  mod).
- **[NHOM-9]** `templates/admin/cong-dong/nhom.html` — Full moderation UI:
  header + banner + 4 stats cards + list groups với owner avatar, member/
  topic count, 3 action buttons (khoá, đặc biệt, xóa).
- **[NHOM-10]** `src/main.rs` — Routes mới: 3 POST endpoints. Replace
  `admin_groups_placeholder` với `admin_nhom_list`.

### 🚫 Từ Vựng Cấm — `/admin/tu-vung-cam`

Trước v0.9.41, nút "Từ vựng cấm" trong admin dashboard chỉ là anchor
`#tu-vung-cam` (không có trang). v0.9.41 thêm module đầy đủ:

- **[VOC-1]** Migration 029 — Tạo bảng `forbidden_words` (id, word, action,
  category, reason, is_system, is_active, created_by, created_at, updated_at).
  CHECK constraints cho action (block/flag) + category (profanity/spam/
  politics/religious/scam/other). Index cho active + category.
- **[VOC-2]** Migration 029 — Seed 9 từ cấm hệ thống mặc định (`is_system=true`):
  7 từ tục tĩu (block) + 2 keyword lừa đảo (flag). `ON CONFLICT DO NOTHING`.
- **[VOC-3]** Safety schema check idempotent cho bảng `forbidden_words` + seed
  system words (chạy trước sqlx migrations, đảm bảo bảng luôn tồn tại).
- **[VOC-4]** `src/handlers/admin.rs::ForbiddenWordRow` — Row struct với
  `created_by_name` JOIN từ users.
- **[VOC-5]** `src/handlers/admin.rs::admin_tu_vung_cam_list` — `GET
  /admin/tu-vung-cam` — render `admin/tu-vung-cam/index.html` với stats
  (active, inactive, system) + form tạo mới + list words.
- **[CMT-6]** `src/handlers/admin.rs::admin_tu_vung_cam_create` — `POST
  /admin/tu-vung-cam/tao` — Validate word (lowercase, max 100), action,
  category. `ON CONFLICT (word) DO UPDATE` để bật lại nếu đã tắt.
- **[VOC-7]** `src/handlers/admin.rs::admin_tu_vung_cam_enable` — `POST
  /admin/tu-vung-cam/{id}/bat` — Set `is_active=true`.
- **[VOC-8]** `src/handlers/admin.rs::admin_tu_vung_cam_disable` — `POST
  /admin/tu-vung-cam/{id}/tat` — Set `is_active=false`.
- **[VOC-9]** `src/handlers/admin.rs::admin_tu_vung_cam_delete` — `POST
  /admin/tu-vung-cam/{id}/xoa` — Hard delete, `WHERE is_system = false` (system
  words không xóa được, chỉ tắt được).
- **[VOC-10]** `src/handlers/admin.rs::check_forbidden_words()` — Helper async
  function: load tất cả active forbidden_words, lowercase content + check
  `contains`. Trả về `Option<(word, action)>`. Sẵn sàng integrate vào
  `create_comment` / `create_topic` / chat handlers (sẽ dùng ở v0.9.42+).
- **[VOC-11]** `templates/admin/tu-vung-cam/index.html` — Full UI: header +
  banner + 3 stats cards + form tạo mới (word, action, category, reason) +
  list words với badges (Hệ thống / Bật / Tắt / Block / Flag) + 3 action
  buttons (bật/tắt, xóa).
- **[VOC-12]** `src/main.rs` — Routes mới: GET + 4 POST endpoints.
- **[VOC-13]** `templates/admin/ky-thuat/index.html` — Fix link "Từ vựng cấm"
  từ `#tu-vung-cam` → `/admin/tu-vung-cam`.

### 💓 Heartbeat Fix — "hoạt động 6 giờ trước dù đang online"

User report: "còn bị lỗi khi tôi đang online nó lại hiện hoạt động 6 giờ
trước". Nguyên nhân root:

- Heartbeat client (`src/static/js/app.js::sessionHeartbeat`) dùng
  `setInterval(..., 10 * 60 * 1000)` — fire lần đầu sau 10 phút, không fire
  ngay khi user login.
- Admin stats `active_users` đếm `WHERE last_seen_at > NOW() - INTERVAL '5 min'`.
- User vừa login (chưa đủ 10 phút) → `last_seen_at` NULL hoặc stale (giá trị
  từ session trước cách đây 6 giờ) → không nằm trong top "active 5 phút" →
  admin thấy "6 giờ trước".

Fix v0.9.41:

- **[HB-1]** `src/static/js/app.js::sessionHeartbeat` — Fire heartbeat NGAY
  khi DOM ready (không chờ interval). Đảm bảo `last_seen_at = NOW()` ngay sau
  khi user login / refresh page.
- **[HB-2]** Giảm interval từ 10 phút → 2 phút (120000ms). Đảm bảo user active
  luôn trong cửa sổ 5 phút "đang hoạt động" của admin stats.
- **[HB-3]** Thêm `visibilitychange` listener — khi tab visible trở lại sau
  1 phút idle, fire heartbeat ngay (không chờ interval tiếp theo).
- **[HB-4]** Thêm `click` + `keydown` listeners — track `lastActive` timestamp
  cho visibilitychange logic.

### 🎵 Music Submit Error — "không thể lưu bài hát vào cơ sở dữ liệu"

User report: "khi tôi đăng bài hát lên web nó báo 'Lỗi gửi bài — không thể
lưu bài hát vào cơ sở dữ liệu', cần fix siêu triệt để". Nguyên nhân root:
error message chỉ nói chung chung, không có cách debug. v0.9.41 cải thiện:

- **[MUS-1]** `src/handlers/nha_nhac.rs::nha_nhac_submit_music` (YouTube) —
  Phân loại lỗi `sqlx::Error` (ColumnNotFound / Database / Decode), hiển thị
  error chi tiết cho user (để report admin), log đầy đủ user_id + title +
  category + youtube_id để admin trace.
- **[MUS-2]** `src/handlers/nha_nhac.rs::nha_nhac_submit_music_file` (audio) —
  Tương tự: phân loại lỗi + log chi tiết + cleanup file/audio_files row khi
  fail.
- **[MUS-3]** Error message mới hiển thị:
  - "Thiếu cột DB: {col}" — khi ColumnNotFound.
  - "Bài hát đã tồn tại (trùng youtube_id)" — khi unique constraint violation.
  - "Vi phạm ràng buộc DB: {msg}" — khi check constraint violation.
  - "Bảng DB không tồn tại: {msg}" — khi relation does not exist.
  - "Lỗi database: {msg}" — fallback cho các lỗi DB khác.
  - "Lỗi không xác định: {e}" — fallback cho các lỗi khác.

### 📱 Mobile Menu Compact

User report: "tôi thấy hiện tại vào thanh ba gạch, dù đã rút gọn nhưng nó vẫn
rất dài, cần rút gọn thêm nhưng không gây xấu hoặc lỗi". v0.9.41:

- **[MOB-1]** `templates/layout.html` — Chuyển sub-items từ vertical list
  (py-2, gap-3, text-xl icon, text-xs label) sang **2-column grid** (gap-0.5,
  py-1.5, text-sm icon, text-[11px] label). Compact gấp đôi chiều cao.
- **[MOB-2]** 4 nút quick-access thu nhỏ: text-lg icon (từ text-xl), text-[9px]
  label (từ text-[10px]), py-1.5 (từ py-2), p-1.5 (từ p-2).
- **[MOB-3]** Section headers thu nhỏ: text-base icon (từ text-xl), text-xs
  font (từ text-sm), py-1.5 (từ py-2.5), px-2.5 (từ px-3), gap-2 (từ gap-3).
- **[MOB-4]** Chevron icon thu nhỏ: w-3.5 h-3.5 (từ w-4 h-4).
- **[MOB-5]** "Tìm Bạn" (5th item trong Bạn Bè) và "Quản Trị" (3rd item trong
  Tài Khoản) dùng `col-span-2` để chiếm cả hàng (vì label dài).

### 📦 Version Sync v0.9.41

- Bump version `0.9.40` → `0.9.41` ở: `Cargo.toml`, `src/main.rs` (startup
  log + health check public + health check inner + phase 44 → 45), `templates/
  layout.html` (footer), `templates/khong-gian/index.html` (footer), `templates/
  admin/placeholder.html` (footer), `templates/admin/thuong-thanh/index.html`
  (footer), `templates/admin/thuong-thanh/danh-muc.html` (footer), `templates/
  admin/ky-thuat/index.html` (footer), `templates/admin/cong-dong/index.html`
  (footer), `templates/admin/quan-li/index.html` (footer), `templates/admin/
  phat-trien/index.html` (phase badge + roadmap), `Dockerfile.coolify`
  (comment), `src/db/mod.rs` (comment), `src/middleware/rate_limit.rs`
  (comment), `src/handlers/admin.rs` (comment), `src/handlers/thuong_thanh.rs`
  (comment), `src/models/thuong_thanh.rs` (comment).
- Update phase 44 → 45 trong health check + main log.
- Thêm 38 feature flags v0.9.41 vào `HEALTH_FEATURES` array (admin moderation
  + forbidden words + heartbeat fix + mobile menu + music submit error).
- Cập nhật roadmap `/admin/phat-trien`: Giai đoạn 44 → "Hoàn thành" (green),
  Giai đoạn 45 → "Đang triển khai" (indigo).

### 🛠️ Technical Notes

- **Build verified**: `SQLX_OFFLINE=true cargo build --release` thành công
  với Rust 1.97.1, 0 errors, 25 warnings (pre-existing dead_code).
- **Migration 029**: idempotent (CREATE TABLE IF NOT EXISTS / ADD COLUMN IF
  NOT EXISTS), an toàn chạy lại.
- **Safety schema**: `ensure_schema_safety()` chạy idempotent DDL trước sqlx
  migrations, đảm bảo các cột/bảng mới luôn tồn tại ngay cả khi migration
  checksum mismatch.
- **Backward compat**: các placeholder functions cũ (`admin_binh_luan_placeholder`,
  `admin_groups_placeholder`) vẫn giữ trong code nhưng không còn được route
  sử dụng — có thể xóa ở v0.9.42+.
- **Forbidden words helper**: `check_forbidden_words()` đã implement nhưng
  chưa integrate vào `create_comment` / `create_topic` / chat handlers — sẽ
  integrate ở v0.9.42+ (cần kiểm tra perf vì mỗi comment submit sẽ thêm 1
  query SELECT tất cả forbidden_words).

---

## [0.9.40] — 2026-08-17 — Giai đoạn 44: Chợ Đạo Hữu + Admin Thương Thành Hoàn Thiện + Payment K/Bank 🪷

### 🎯 Mục tiêu giai đoạn

Bản phát hành này thực hiện **3 thay đổi lớn** theo yêu cầu user:

1. **Xóa hoàn toàn phần "Đăng Bán Vật Phẩm PvP"** — vì game Siêu Độ đã bị xóa
   khỏi dự án từ v0.9.35 (Giai đoạn 40), nên PvP (người-vs-người) không còn ý
   nghĩa. Thay bằng một loại đăng bán mới linh hoạt hơn.
2. **Đăng bán linh hoạt theo danh mục** — người đăng có thể chọn danh mục có
   sẵn trong hệ thống HOẶC tạo mới danh mục (cần admin duyệt trước khi public).
3. **Chọn phương thức thanh toán** — khi đăng vật phẩm, người đăng có thể chọn
   nhận tiền bằng **K** (tiền tệ trong app, 10% phí hệ thống) HOẶC **chuyển
   khoản ngân hàng** (tự điền thông tin bank_name, account_number,
   account_holder, QR URL).
4. **Hoàn thiện bảng quản trị Thương Thành** — trước v0.9.40, admin không có
   UI quản lý Thương Thành. Không thể duyệt, xóa, hoặc feature sản phẩm do
   user đăng. v0.9.40 thêm 2 trang admin hoàn chỉnh.

### 🤝 Chợ Đạo Hữu — Rename từ "Chợ PvP"

Theo tài liệu `HieuLouis/Hệ Thống Và Chức Năng Chi Tiết.docx` mục V.3, "Cửa Hàng
PvP" được định nghĩa là nơi thành viên đăng đạo cụ trong game Siêu Độ. Vì game
đã bị xóa, giữ tên "PvP" gây nhầm lẫn cho người dùng mới. v0.9.40 rename thành
"Chợ Đạo Hữu" — phản ánh đúng bản chất: nơi đạo hữu trao đổi vật phẩm Phật giáo,
sách, đồ thờ, dịch vụ thiện lành (không liên quan đến game).

- **[DH-1]** `src/models/thuong_thanh.rs` — Rename `ShopStore::Pvp` → `ShopStore::DaoHuu`.
  Method `from_str()` giờ accept cả `"pvp"` (data cũ) và `"dao_huu"` (mới) — cùng
  map về `DaoHuu`. Label `"Chợ Đạo Hữu"`, icon `🤝`, color `#C62828`.
- **[DH-2]** `src/handlers/thuong_thanh.rs` — Rename `store_pvp` → `store_dao_huu`.
  Thêm `store_pvp_redirect` — `GET /thuong-thanh/pvp` giờ redirect 301 permanent
  → `/thuong-thanh/cho-dao-huu` (back-compat cho bookmark cũ).
- **[DH-3]** `src/main.rs` — Route mới `/thuong-thanh/cho-dao-huu`. Route cũ
  `/thuong-thanh/pvp` giữ lại (gọi `store_pvp_redirect`).
- **[DH-4]** `templates/thuong-thanh/pvp.html` — Hero đổi "⚔️ Chợ PvP" → "🤝 Chợ Đạo
  Hữu". Mô tả đổi "Đạo hữu tự đăng bán vật phẩm, thiết lập giá. Giao dịch thu 20%
  phí" → "Đạo hữu tự đăng bán vật phẩm Phật giáo, sách, đồ thờ, dịch vụ thiện lành.
  Chọn nhận tiền K (10% phí) hoặc chuyển khoản ngân hàng".
- **[DH-5]** `templates/thuong-thanh/index.html` — Section "Chợ PvP" → "Chợ Đạo Hữu".
  Stats label "PvP đang bán" → "Đạo Hữu đang bán". Link "Xem tất cả" trỏ tới
  `/thuong-thanh/cho-dao-huu` (thay `/thuong-thanh/pvp`). Hiển thị giá K HOẶC
  VNĐ + badge "🏦 Ngân hàng" / "🪷 Tiền K" cho mỗi item.
- **[DH-6]** `templates/thuong-thanh/create.html` — Title "Đăng Bán PvP" →
  "Đăng Bán Vật Phẩm — Chợ Đạo Hữu". Nút submit "⚔️ Đăng bán" → "🤝 Đăng bán"
  (color #C62828 → #0F766E).
- **[DH-7]** `templates/thuong-thanh/item.html` — Breadcrumb "Chợ PvP" → "Chợ Đạo Hữu".
  Back link cũng đổi theo.
- **[DH-8]** `templates/thuong-thanh/cart.html` — Label "PvP (20% phí)" →
  "Chợ Đạo Hữu (10% phí)".

### 📂 Danh Mục Linh Hoạt — Chọn Có Sẵn Hoặc Tạo Mới

User khi đăng bán có thể chọn 1 trong 2 cách:
- **Chọn danh mục có sẵn** từ dropdown (12 danh mục hệ thống: Thẻ Tu Học, Thẻ
  Đổi Tên, Vật Phẩm, Cao Cấp, Sách Phật Giáo, Đồ Thờ, Dịch Vụ, Thực Phẩm Chay,
  Thẻ Hỗ Trợ, Thẻ Nhóm, Thẻ Bầu Chọn, Khác).
- **Tạo danh mục mới** — user nhập tên + icon emoji. Danh mục do user tạo có
  `is_system = false, is_approved = false` → cần admin duyệt trước khi xuất
  hiện công khai. Trong khi chờ duyệt, item vẫn đăng được (dùng tạm category
  text).

- **[CAT-1]** `migrations/028_cho_dao_huu_marketplace.sql` — Tạo bảng
  `shop_categories` (id, slug, name_vi, description, icon, color, parent_id,
  sort_order, is_system, is_approved, is_active, created_by, created_at,
  updated_at). Index `idx_shop_categories_parent` + `idx_shop_categories_active`.
- **[CAT-2]** Migration 028 — Seed 12 danh mục hệ thống (`is_system = true`):
  the-tu-hoc, the-doi-ten, the-ho-tro, the-nhom, the-bau-chon, vat-pham,
  cao-cap, sach-phat-giao, do-tho, dich-vu, thuc-pham-chay, khac.
- **[CAT-3]** Migration 028 — Backfill `category_id` cho shop_items cũ (map
  `category` TEXT → `shop_categories.slug` qua `REPLACE(LOWER(category), '_', '-')`).
- **[CAT-4]** `src/db/mod.rs::ensure_schema_safety()` — `CREATE TABLE IF NOT
  EXISTS shop_categories (... 14 cột ...)` (idempotent, chạy trước sqlx
  migrations). Đồng bộ với migration 028. Index + seed 12 categories.
- **[CAT-5]** `src/handlers/thuong_thanh.rs::slugify_vi()` — Hàm tạo slug từ
  tên tiếng Việt có dấu (VD: "Đồ Gốm Phật Giáo" → "do-go-phat-giao"). Bỏ dấu
  → lowercase → thay khoảng trắng bằng `-`.
- **[CAT-6]** `src/handlers/thuong_thanh.rs::fetch_categories()` — Fetch all
  active, approved categories (ORDER BY sort_order, name_vi).
- **[CAT-7]** `src/handlers/thuong_thanh.rs::create_item_form` — Truyền
  `categories: Vec<ShopCategory>` vào template cho dropdown.
- **[CAT-8]** `src/handlers/thuong_thanh.rs::create_item` — Nếu user nhập
  `new_category_name` không rỗng → INSERT vào `shop_categories` với
  `is_system = false, is_approved = false, created_by = user_id`. Lấy `id` mới
  → gán vào `shop_items.category_id`. Nếu trùng slug (đã có user khác tạo)
  → `ON CONFLICT (slug) DO UPDATE` lấy id hiện có.
- **[CAT-9]** `templates/thuong-thanh/create.html` — Dropdown "Chọn có sẵn"
  + toggle "Tạo danh mục mới" (Alpine.js `createNewCat`). Khi toggle = tạo
  mới, hiện input `new_category_name` + `new_category_icon`.

### 💰 Payment Method — K Hoặc Ngân Hàng

Khi đăng vật phẩm, user chọn 1 trong 2 phương thức thanh toán:

- **K (tiền tệ trong app)** — buyer thêm vào giỏ hàng, thanh toán = trừ K từ
  ví buyer + cộng K cho seller (sau phí 10%). Phí giảm từ 20% → 10% cho Chợ
  Đạo Hữu (PvP cũ vẫn 20% cho back-compat).
- **Chuyển khoản ngân hàng** — seller tự điền: `bank_name` (Vietcombank,
  Techcombank, MB Bank...), `account_number`, `account_holder`, `branch`
  (optional), `qr_image_url` (URL ảnh QR VietQR hoặc QR tự tạo). Buyer xem
  thông tin ngân hàng trên trang chi tiết vật phẩm, tự liên hệ seller để
  chuyển khoản. KHÔNG qua giỏ hàng (vì không thể verify chuyển khoản tự động).

- **[PAY-1]** Migration 028 — `ALTER TABLE shop_items ADD COLUMN IF NOT EXISTS
  payment_method TEXT NOT NULL DEFAULT 'k' CHECK (payment_method IN ('k', 'bank'))`.
- **[PAY-2]** Migration 028 — `ALTER TABLE shop_items ADD COLUMN IF NOT EXISTS
  price_vnd BIGINT` (giá VNĐ khi payment_method = 'bank').
- **[PAY-3]** Migration 028 — `ALTER TABLE shop_items ADD COLUMN IF NOT EXISTS
  bank_info JSONB DEFAULT '{}'` (lưu {bank_name, account_number,
  account_holder, branch, qr_image_url}).
- **[PAY-4]** Migration 028 — `ALTER TABLE transactions ADD COLUMN IF NOT
  EXISTS payment_method TEXT NOT NULL DEFAULT 'k'` (snapshot lúc giao dịch).
- **[PAY-5]** Migration 028 — `ALTER TABLE transactions ADD COLUMN IF NOT
  EXISTS price_vnd BIGINT` + `bank_info JSONB` + `buyer_contact TEXT` (snapshot
  buyer contact info khi bank transfer).
- **[PAY-6]** `src/models/thuong_thanh.rs::BankInfo` — Struct mới với 5 field
  (bank_name, account_number, account_holder, branch, qr_image_url). Method
  `to_json()` build `serde_json::Value` cho sqlx bind. Method `validate()`
  kiểm tra 3 field bắt buộc + max length.
- **[PAY-7]** `src/models/thuong_thanh.rs::format_vnd()` — Helper format số
  VNĐ: `1500000` → `"1.500.000 ₫"`.
- **[PAY-8]** `src/models/thuong_thanh.rs::ShopItem` + `ShopItemWithSeller` —
  Thêm field `category_id, payment_method, price_vnd, bank_info, is_featured,
  moderation_status` (with `#[sqlx(default)]`). Method `price_display()` trả
  K hoặc VNĐ tuỳ payment_method. Method `bank_info_struct()` parse JSONB.
- **[PAY-9]** `src/models/thuong_thanh.rs::ItemCreateForm` — Form mới với
  tất cả field: name, description, price_k, price_vnd, category_id,
  new_category_name, new_category_icon, payment_method, bank_name,
  account_number, account_holder, branch, qr_image_url, buyer_contact.
  Method `validate()` trả `ValidatedItem` struct (category_id, payment_method,
  price_vnd, bank_info).
- **[PAY-10]** `src/handlers/thuong_thanh.rs::create_item` — INSERT shop_items
  với tất cả field mới. `payment_method = 'bank'` → bind `price_vnd` +
  `bank_info` JSONB. `moderation_status = 'approved'` (auto-approve — admin có
  thể review sau qua `/admin/thuong-thanh`).
- **[PAY-11]** `templates/thuong-thanh/create.html` — Toggle "Nhận tiền K" /
  "Chuyển khoản ngân hàng" (Alpine.js `payment`). Khi `payment = 'k'` → hiện
  input `price_k`. Khi `payment = 'bank'` → hiện 5 input (bank_name,
  account_number, account_holder, branch, qr_image_url) + giá VNĐ + warning
  "Vật phẩm thanh toán ngân hàng KHÔNG qua giỏ hàng".
- **[PAY-12]** `templates/thuong-thanh/item.html` — Khi `item.payment_method
  == 'bank'` → render box "🏦 Thông tin chuyển khoản" với 4 field (bank_name,
  account_number, account_holder, branch) + ảnh QR (nếu có `qr_image_url`) +
  warning "Đây là giao dịch giữa hai cá nhân. Hệ thống không chịu trách nhiệm".
  Nút "🛒 Thêm vào giỏ hàng" → "💬 Liên hệ người bán" (link `/ban-be/tin-nhan`).
- **[PAY-13]** `src/handlers/thuong_thanh.rs::cart_add` — Check
  `payment_method` của item trước khi thêm vào giỏ. Nếu `bank` → redirect tới
  `/thuong-thanh/vat-pham/{id}?bank=1` (trang chi tiết có bank info). Nếu `k`
  → thêm vào giỏ bình thường.
- **[PAY-14]** `src/handlers/thuong_thanh.rs::cart_checkout` — Thêm
  `payment_method = 'k'` khi INSERT transaction. Fee cho Đạo Hữu giảm từ 20%
  → 10% (PvP cũ vẫn 20% cho back-compat).
- **[PAY-15]** `templates/thuong-thanh/pvp.html` — Card hiển thị giá K HOẶC
  VNĐ + badge "🏦 Ngân hàng" / "🪷 Tiền K" cho mỗi item. Nút "🛒 Mua" →
  "🏦 Xem bank info" nếu bank payment.

### 🛡️ Admin Thương Thành Hoàn Thiện

Trước v0.9.40, admin không có UI quản lý Thương Thành. Module chỉ có ở mặt user.
User report không có cách kiểm duyệt sản phẩm đăng bán (đặc biệt khi user chọn
bank payment với thông tin ngân hàng của họ). v0.9.40 thêm 2 trang admin hoàn chỉnh.

- **[ADM-1]** `src/handlers/admin.rs::admin_thuong_thanh_list` — `GET
  /admin/thuong-thanh` — List 100 shop_items mới nhất (kèm seller_name +
  category_name JOIN). Stats: tổng vật phẩm, đang hoạt động, nổi bật, chờ
  duyệt. Permission: tất cả admin role.
- **[ADM-2]** `src/handlers/admin.rs::admin_thuong_thanh_delete` — `POST
  /admin/thuong-thanh/{id}/xoa` — Soft delete (set `is_active = false,
  moderation_status = 'removed'`). Audit log.
- **[ADM-3]** `src/handlers/admin.rs::admin_thuong_thanh_toggle_featured` —
  `POST /admin/thuong-thanh/{id}/noi-bat` — Toggle `is_featured` (đặt/bỏ
  nổi bật).
- **[ADM-4]** `src/handlers/admin.rs::admin_thuong_thanh_approve` — `POST
  /admin/thuong-thanh/{id}/duyet` — Set `moderation_status = 'approved',
  is_active = true`.
- **[ADM-5]** `src/handlers/admin.rs::admin_thuong_thanh_reject` — `POST
  /admin/thuong-thanh/{id}/tu-choi` — Set `moderation_status = 'rejected',
  is_active = false` (ẩn khỏi công khai).
- **[ADM-6]** `src/handlers/admin.rs::admin_thuong_thanh_categories` — `GET
  /admin/thuong-thanh/danh-muc` — List all shop_categories (kèm item_count +
  creator_name JOIN). Hiển thị badge "Hệ thống" / "User tạo" / "Chờ duyệt" /
  "Ẩn".
- **[ADM-7]** `src/handlers/admin.rs::admin_category_create` — `POST
  /admin/thuong-thanh/danh-muc/tao` — Tạo category mới (`is_system = true,
  is_approved = true`). Tự tạo slug từ tên nếu user không cung cấp.
- **[ADM-8]** `src/handlers/admin.rs::admin_category_approve` — `POST
  /admin/thuong-thanh/danh-muc/{id}/duyet` — Duyệt category do user tạo
  (`is_approved = true, is_active = true`).
- **[ADM-9]** `src/handlers/admin.rs::admin_category_delete` — `POST
  /admin/thuong-thanh/danh-muc/{id}/xoa` — Ẩn category (`is_active = false`).
  Vật phẩm thuộc category sẽ không bị xóa.
- **[ADM-10]** `templates/admin/thuong-thanh/index.html` — Trang list items
  với 4 stats cards + table list + actions (Duyệt / Từ chối / Nổi bật / Xóa).
  Filter chips theo moderation_status. Color-coded badges.
- **[ADM-11]** `templates/admin/thuong-thanh/danh-muc.html` — Trang quản lý
  categories với form tạo mới (name_vi, slug, icon, color, description) +
  list categories + actions (Duyệt / Ẩn).
- **[ADM-12]** `src/main.rs` — 8 routes mới cho admin Thương Thành.
- **[ADM-13]** `templates/admin/quan-li/index.html` — Thêm nav tab "🏪 Thương
  Thành" trỏ tới `/admin/thuong-thanh`.

### 🗄️ Migration 028 + Safety Schema

- **[DB-1]** `migrations/028_cho_dao_huu_marketplace.sql` — Toàn bộ schema
  mới (shop_categories table + 6 cột mới shop_items + 4 cột mới transactions
  + 12 system categories seed + backfill category_id + 3 index + 2 CHECK
  constraint mới).
- **[DB-2]** `src/db/mod.rs::ensure_schema_safety()` — `CREATE TABLE IF NOT
  EXISTS shop_categories (...)`, `ALTER TABLE shop_items ADD COLUMN IF NOT
  EXISTS ...` (6 cột), `ALTER TABLE transactions ADD COLUMN IF NOT EXISTS
  ...` (4 cột). Index + seed 12 categories. Backfill category_id. Idempotent
  — chạy trước sqlx migrations để schema luôn nhất quán (cùng cơ chế đã fix
  v0.9.25, v0.9.38, v0.9.39).

### 📦 Version Sync v0.9.40

- Bump version `0.9.39` → `0.9.40` ở: `Cargo.toml`, `src/main.rs` (startup
  log + health check public + health check inner + phase 43 → 44), `templates/
  layout.html` (footer), `templates/admin/placeholder.html` (title + footer),
  `templates/admin/ky-thuat/index.html` (title + footer), `templates/admin/
  quan-li/index.html` (footer), `templates/admin/cong-dong/index.html`
  (footer), `templates/admin/phat-trien/index.html` (badge + roadmap + footer),
  `templates/khong-gian/index.html` (footer), `src/middleware/rate_limit.rs`
  (429 page footer), `Dockerfile.coolify` (comment).
- Update phase 43 → 44 trong health check + main log.
- Thêm 43 feature flags v0.9.40 vào `HEALTH_FEATURES` array.
- Cập nhật roadmap `/admin/phat-trien`: Giai đoạn 43 → "Hoàn thành" (green),
  Giai đoạn 44 → "Đang triển khai" (indigo).
- Thêm `v0_9_40_note` vào health check JSON response.

### 🔧 Tech Notes

- **Rust 1.97.1** — Đảm bảo `Cargo.toml` có `rust-version = "1.97.1"` và
  `Dockerfile` dùng `rust:1.97.1-slim-bookworm`. Build pass `cargo check
  --release` và `cargo build --release` thành công (17 warnings, 0 errors —
  tất cả warnings là dead-code fields/methods reserved cho future use).
- **Askama template syntax** — Không hỗ trợ `.iter().filter().count()` trong
  template expressions. Workaround: precompute counts trong handler, truyền
  vào template struct (`total_active, total_featured, total_pending`).
- **Alpine.js** — Form đăng bán dùng Alpine.js cho 2 toggle: `createNewCat`
  (chọn category có sẵn vs tạo mới) + `payment` (K vs bank). CSS `x-cloak`
  ẩn các section chưa active để tránh FOUC.
- **Slug generation** — `slugify_vi()` tự tạo slug từ tên tiếng Việt có dấu
  (Đồ Gốm → do-go). Mirror function `slugify_vi_admin()` trong admin handler
  (tránh circular dependency giữa handlers::thuong_thanh và handlers::admin).

---

## [0.9.39] — 2026-08-17 — Giai đoạn 43: Active User Sync + Settings Fix + Stats Timezone Fix + Mobile Menu Accordion 🪷

### 🎯 Mục tiêu giai đoạn

Bản phát hành này giải quyết **5 vấn đề user báo cáo nhiều nhất** về đồng bộ,
sai số thống kê và UI tràn màn hình:

1. **Bug "5 user đang hoạt động nhưng vào quản lý thành viên không thấy ai online"** —
   admin dashboard hiển thị "5 active users" nhưng khi vào `/admin/thanh-vien`
   không thấy user nào có dấu chấm xanh "Đang hoạt động". Bản thân user đang
   login cũng bị hiển thị "hoạt động 1 ngày trước" dù vừa mở app. **Nguyên nhân
   gốc:** admin stats `active_users` đếm `WHERE is_active` (tức là "tài khoản
   KHÔNG bị ban") chứ không phải "đang online". User list hiển thị
   `MAX(sessions.created_at)` (lúc LOGIN, không phải lúc ACTIVE). Heartbeat
   handler `/api/heartbeat` không làm gì cả, chỉ trả về `{"status":"ok"}`.
2. **Bug "Lỗi database: error returned from database: relation 'user_settings'
   does not exist"** — khi user lưu cài đặt trên trang `/cai-dat`, server trả
   lỗi 500 vì bảng `user_settings` (migration 017) chưa được apply trên production
   (checksum mismatch, partial deploy, manual rollback). Tương tự bug v0.9.25
   (users.i_balance) và v0.9.38 (groups.logo_upload_id).
3. **Bug "Tính sai tổng lời niệm" + "Tính sai số ngày tu liên tiếp"** — user niệm
   phật nhưng "Tổng niệm" và "Ngày tu liên tiếp" hiển thị sai. **Nguyên nhân
   gốc:** Docker container TZ=UTC mặc định, trong khi user ở Asia/Saigon (UTC+7).
   `CURRENT_DATE` trong PostgreSQL trả về ngày UTC, nhưng `today_utc_naive()`
   trong Rust cũng trả về UTC → user niệm phật lúc 01:00 Saigon Aug 17 (= 18:00
   UTC Aug 16) bị ghi `log_date = Aug 16` (sai! user nghĩ là Aug 17). Streak bị
   lệch, today_niem bị 0 dù user đã niệm "hôm nay".
4. **Bug "Nút ba gạch tràn màn hình"** — mobile drawer 27+ items liệt kê dọc
   từ trên xuống dưới, quá dài, gây tràn màn hình và khó tìm. User muốn rút gọn
   hoặc xếp gọn lại.
5. **Nhiều lỗi đồng bộ khác** — admin dashboard vs user list không khớp số
   user active, user tự thấy mình "hoạt động 1 ngày trước", last_seen không
   phản ánh đúng lúc user online.

### 🔧 Fix Root Cause — Active User Sync (Heartbeat + last_seen_at)

Trước v0.9.39, `/api/heartbeat` handler KHÔNG làm gì cả — chỉ trả về
`{"status":"ok"}`. Client app.js gọi heartbeat mỗi 10 phút nhưng server
không track thời điểm user active.

- **[SYNC-1]** `migrations/027_user_last_seen_at.sql` — Thêm cột
  `last_seen_at TIMESTAMPTZ` vào users. Index `idx_users_last_seen_at` cho
  query "active trong 5 phút" nhanh. Seed `last_seen_at` cho user hiện có
  từ `MAX(sessions.created_at)` (lấy session gần nhất làm baseline).
- **[SYNC-2]** `src/db/mod.rs::ensure_schema_safety()` — `ALTER TABLE users
  ADD COLUMN IF NOT EXISTS last_seen_at TIMESTAMPTZ` + index (idempotent,
  chạy trước sqlx migrations → đảm bảo cột tồn tại ngay cả khi migration 027
  chưa được apply).
- **[SYNC-3]** `src/handlers/mod.rs::heartbeat()` — Handler giờ update
  `users.last_seen_at = NOW()` cho user đã login. Best-effort (không block
  response nếu DB error).
- **[SYNC-4]** `src/handlers/admin.rs::fetch_admin_stats()` — `active_users`
  giờ đếm `WHERE last_seen_at IS NOT NULL AND last_seen_at > NOW() - INTERVAL
  '5 minutes'` (tức là user đã heartbeat trong 5 phút gần nhất = đang online
  thật). Fallback: nếu cột last_seen_at không tồn tại, dùng `is_active` (cũ)
  để không crash server.
- **[SYNC-5]** `src/main.rs::fetch_admin_stats_summary()` — Tương tự SYNC-4
  cho health check JSON response.
- **[SYNC-6]** `src/handlers/admin.rs::fetch_users_list()` — SELECT dùng
  `COALESCE(u.last_seen_at, (SELECT MAX(s.created_at) FROM sessions s WHERE
  s.user_id = u.id)) AS last_session_at`. Ưu tiên `last_seen_at` (update qua
  heartbeat), fallback về `MAX(sessions.created_at)` nếu user chưa heartbeat
  từ v0.9.39.
- **[SYNC-7]** `src/handlers/admin.rs::AdminUserRow::last_seen_text()` — Giữ
  nguyên logic "Đang hoạt động" (< 5 phút), "X phút trước" (5-60 phút), "X
  giờ trước" (1-24 giờ), "X ngày trước" (> 24 giờ). Nhưng giờ dùng `last_seen_at`
  thay vì `MAX(sessions.created_at)` → phản ánh đúng thời điểm user active gần
  nhất, không phải lúc login. Fix bug "tôi đang hoạt động nhưng nó báo 1 ngày
  trước".

### 🗄️ Fix Root Cause — user_settings Table Safety Schema

Tương tự bug v0.9.25 (users.i_balance) và v0.9.38 (groups.logo_upload_id),
migration 017 (user_settings) có thể không được apply trên production do
checksum mismatch, partial deploy, hoặc manual rollback. Khi đó, INSERT/SELECT
trên `user_settings` fail với "relation user_settings does not exist".

- **[DB-1]** `src/db/mod.rs::ensure_schema_safety()` — `CREATE TABLE IF NOT
  EXISTS user_settings (... 17 cột ...)` (idempotent). Đồng bộ với migration
  017. Index `idx_user_settings_theme` + `idx_user_settings_visibility`.
- **[DB-2]** `src/db/mod.rs::ensure_schema_safety()` — Seed default settings
  cho user hiện có (chưa có row trong `user_settings`): `INSERT INTO user_settings
  (user_id) SELECT id FROM users WHERE NOT EXISTS (...)`. Idempotent.
- **[DB-3]** `src/handlers/cai_dat.rs` — Không cần sửa code vì `fetch_user_settings`
  đã có fallback `INSERT INTO user_settings (user_id) VALUES ($1) ON CONFLICT
  DO NOTHING` nếu user chưa có row. Nhưng trước v0.9.39, fallback này fail nếu
  bảng không tồn tại → giờ bảng luôn tồn tại nhờ safety schema.

### 🕐 Fix Root Cause — Timezone Streak + Today Niem

Trước v0.9.39, Docker container không set TZ → mặc định UTC. PostgreSQL
`CURRENT_DATE` trả về ngày UTC. User ở Asia/Saigon (UTC+7) → lệch 7 giờ:
- User niệm phật lúc 23:00 Saigon Aug 16 (= 16:00 UTC Aug 16) → log_date = Aug 16 ✓
- User niệm phật lúc 08:00 Saigon Aug 17 (= 01:00 UTC Aug 17) → log_date = Aug 17 ✓
- User niệm phật lúc 01:00 Saigon Aug 17 (= 18:00 UTC Aug 16) → log_date = Aug 16 ✗ (sai!)

- **[TZ-1]** `Dockerfile` — `ENV TZ=Asia/Ho_Chi_Minh` cho runtime stage.
  Cài `tzdata` package để hỗ trợ timezone conversion.
- **[TZ-2]** `src/handlers/khong_gian.rs` — `today_utc_naive()` →
  `today_local_naive()`. Dùng `chrono::Local::now().date_naive()` thay vì
  `Utc::now().date_naive()`. Local::now() đọc TZ env var → trả về giờ Saigon.
- **[TZ-3]** `src/handlers/khong_gian.rs::compute_streak()` — Dùng
  `today_local_naive()` thay vì `today_utc_naive()`. Streak giờ tính theo ngày
  Saigon, đồng bộ với `CURRENT_DATE` trong PostgreSQL (cũng đọc TZ env var).
- **[TZ-4]** `docker-compose.yml` — Không cần sửa (dev environment, có thể
  override TZ qua env var nếu muốn).

Nhờ fix này:
- User niệm phật lúc 01:00 Saigon Aug 17 → DB ghi log_date = Aug 17 (đúng).
- `today_niem` hiển thị đúng số lần niệm hôm nay (theo giờ Saigon).
- `streak_days` tính đúng số ngày liên tiếp (theo giờ Saigon).
- `total_niem` (SUM của mọi niem_count) không bị ảnh hưởng bởi timezone —
  vẫn đúng.

### 🎨 Fix UI — Mobile Menu Accordion (Rút Gọn)

Trước v0.9.39, mobile drawer 27+ items liệt kê dọc từ trên xuống dưới → quá
dài, tràn màn hình, khó tìm. v0.9.39 refactor thành 6 section accordion:

- **[UI-1]** `templates/layout.html` — Thêm Alpine.js Collapse plugin
  (`@alpinejs/collapse@3.14.9`). Plugin này cho phép `x-collapse` directive
  smooth height transition.
- **[UI-2]** `templates/layout.html` — Body `x-data` thêm `mobileMenuSection: 'main'`
  state để track section nào đang mở. Default: không section nào mở.
- **[UI-3]** `templates/layout.html` — Top of mobile drawer: 4 nút chính
  grid 4 cột (Trang Chủ + Không Gian + Cộng Đồng + Kinh Sách) — quick access,
  không cần mở section.
- **[UI-4]** `templates/layout.html` — 6 section accordion (mỗi section có
  header + chevron icon rotate 180° khi mở):
  1. 🌍 Không Gian (Niệm Phật, Nhà Nhạc)
  2. 👥 Cộng Đồng (Tất Cả Nhóm, Tạo Nhóm Mới)
  3. 👤 Bạn Bè (Danh Sách, Tin Nhắn, Hộp Thư, Thông Báo, Tìm Bạn)
  4. 📚 Kinh Sách (Thư Viện, Phật Gia, Đạo Gia, Tìm Sách)
  5. 🧭 Khám Phá (Giới Thiệu, Tổng Quan, Quỹ Từ Bi, BXH, Thành Tích,
     Thương Thành, Đội Ngũ, Tìm Kiếm)
  6. ⚙️ Tài Khoản (Hồ Sơ, Cài Đặt, Quản Trị, Theme toggle, Thoát) — chỉ
     khi đã login
- **[UI-5]** Mỗi section header là `<button>` với `@click="mobileMenuSection =
  mobileMenuSection === '<id>' ? '' : '<id>'"`. Click lại section đang mở sẽ
  đóng (toggle behavior). Mở section mới KHÔNG tự đóng section cũ (cho phép
  nhiều section mở cùng lúc — user có thể so sánh).
- **[UI-6]** Mỗi section content dùng `x-show="mobileMenuSection === '<id>'"`
  + `x-collapse` (smooth height animation) + `x-cloak` (chống FOUC).
- **[UI-7]** Chưa login: ẩn section Tài Khoản, hiển thị Theme toggle + Đăng
  Nhập Google ở cuối drawer.

### 📦 Version Sync v0.9.39

- Bump version `0.9.38` → `0.9.39` ở: `Cargo.toml`, `src/main.rs` (startup
  log + health check public + health check inner + phase 42 → 43),
  `templates/layout.html` (footer), `templates/khong-gian/index.html`
  (footer), `templates/admin/cong-dong/index.html` (footer),
  `templates/admin/quan-li/index.html` (footer), `templates/admin/ky-thuat/index.html`
  (title + footer), `templates/admin/placeholder.html` (title + footer),
  `templates/admin/phat-trien/index.html` (phase badge + roadmap + footer),
  `src/middleware/rate_limit.rs` (429 page footer), `Dockerfile.coolify` (comment).
- Update phase 42 → 43 trong health check + main log + admin phat-trien dashboard.
- Thêm 16 feature flags v0.9.39 vào `HEALTH_FEATURES` array (active-user-sync
  + user-settings-safety-schema + timezone-streak-fix + mobile-menu-accordion
  + heartbeat-update-last_seen_at + dockerfile-tz-asia-ho_chi_minh + migration-027
  × 16 flags).
- Thêm `v0_9_39_note` vào health check `notes` object.
- Cập nhật roadmap `/admin/phat-trien`: Giai đoạn 42 (Logo PNG + Group Logo Fix
  + Music Submit Fix + Team Update) → "Hoàn thành" (green). Giai đoạn 43
  (Active User Sync + Settings Fix + Stats Timezone Fix + Mobile Menu Accordion)
  → "Đang triển khai" (indigo, badge 43).

### 📋 Ghi chú vận hành

- **Database**: Safety schema check chạy idempotent DDL trước sqlx migrations →
  đảm bảo `user_settings` table + `users.last_seen_at` column luôn tồn tại ngay
  cả khi migration 017 / 027 chưa được apply. KHÔNG xóa dữ liệu user, chỉ CREATE
  TABLE IF NOT EXISTS / ADD COLUMN IF NOT EXISTS.
- **Timezone**: Dockerfile set `TZ=Asia/Ho_Chi_Minh`. Cần cài `tzdata` package
  trong Docker runtime stage (debian:bookworm-slim) để timezone conversion hoạt
  động. Postgres đọc TZ env var của process → `CURRENT_DATE` trả về ngày Saigon.
- **Heartbeat**: Client app.js gọi `/api/heartbeat` mỗi 10 phút. Server update
  `users.last_seen_at = NOW()`. Admin stats `active_users` đếm user có
  `last_seen_at > NOW() - 5 min` = đang online thật.
- **Streak**: Trước v0.9.39, user niệm phật lúc 01:00 Saigon Aug 17 bị ghi
  log_date = Aug 16 (UTC). Sau v0.9.39, log_date = Aug 17 (Saigon). Streak tính
  đúng số ngày liên tiếp theo giờ Saigon.
- **Rollback**: Nếu TZ fix gây vấn đề (vd. Postgres cluster có timezone config
  riêng), có thể revert Dockerfile TZ env var. Streak sẽ tiếp tục dùng UTC nhưng
  vẫn nhất quán (đã hoạt động từ v0.9.15).

---

## [0.9.38] — 2026-08-17 — Giai đoạn 42: Logo PNG + Group Logo Bug Fix + Music Submit Bug Fix + About Page Team Update 🪷

### 🎯 Mục tiêu giai đoạn

Bản phát hành này giải quyết **4 vấn đề user báo cáo** và **1 yêu cầu thương hiệu**:

1. **Bug "Lỗi cập nhật logo nhóm."** — khi user (owner/admin) upload logo mới cho nhóm cộng đồng,
   handler `change_group_logo` trả lỗi 500. Nguyên nhân gốc: migration 026 (v0.9.36) thêm cột
   `groups.logo_upload_id` nhưng trên production có thể chưa được apply đầy đủ (checksum mismatch,
   partial deploy, DB rollback manual). Khi `UPDATE groups SET logo_upload_id = ...` chạy mà cột
   không tồn tại → SQL error → user thấy "Lỗi cập nhật logo nhóm." trên trang trắng.
2. **Bug "lỗi gửi bài" khi đăng nhạc trong Nhà Nhạc** — khi user submit file âm thanh (MP3/M4A/OGG/WAV/FLAC)
   hoặc YouTube link, INSERT vào `user_music_submissions` fail vì các cột `source_type`,
   `audio_file_upload_id`, `audio_duration_seconds` (migration 026) chưa tồn tại. User thấy
   "⚠️ Lỗi gửi bài — vui lòng thử lại." mà không rõ nguyên nhân.
3. **Yêu cầu thay toàn bộ logo web sang PNG thật** — trước v0.9.38, logo toàn web dùng emoji 🪷
   (SVG data URI cho favicon, `<span>🪷</span>` cho header/footer/bottom-nav/home/login). Emoji
   render khác nhau trên mỗi platform (Apple Color Emoji vs Segoe UI Emoji vs Noto Color Emoji)
   → thương hiệu không nhất quán. User cung cấp `tubi.png` (1254×1254 PNG thật) yêu cầu thay toàn bộ.
4. **Cập nhật thông tin đội ngũ trên trang /gioi-thieu** — Đỗ Văn Cường rút lui về làm hỗ trợ,
   Nguyễn Đình Minh Hiếu từ Admin Cộng Đồng chuyển sang Admin Kỹ Thuật để phụ trách chính.

### 🔧 Fix Root Cause — Safety Schema Check cho migration 026

Cùng cơ chế đã fix v0.9.25 (cho `users.i_balance` + `permissions` table), giờ áp dụng cho
migration 026 (v0.9.36 — Community Group Logo + Audio File Uploads). Trước khi sqlx migrations
chạy, `ensure_schema_safety()` chạy idempotent DDL trực tiếp (`ADD COLUMN IF NOT EXISTS` /
`CREATE TABLE IF NOT EXISTS`) để đảm bảo schema luôn nhất quán:

- **[SAFETY-1]** `src/db/mod.rs` — `ALTER TABLE groups ADD COLUMN IF NOT EXISTS logo_upload_id
  UUID REFERENCES images(id) ON DELETE SET NULL`. Fix bug "Lỗi cập nhật logo nhóm."
- **[SAFETY-2]** `src/db/mod.rs` — `CREATE TABLE IF NOT EXISTS audio_files (... 11 columns ...)`
  + 3 index. Fix bug "lỗi gửi bài" (audio file upload path).
- **[SAFETY-3]** `src/db/mod.rs` — `ALTER TABLE user_music_submissions ADD COLUMN IF NOT EXISTS
  source_type TEXT NOT NULL DEFAULT 'youtube'` + CHECK constraint (idempotent qua `DO $$ ... END $$`).
- **[SAFETY-4]** `src/db/mod.rs` — `ALTER TABLE user_music_submissions ADD COLUMN IF NOT EXISTS
  audio_file_upload_id UUID REFERENCES audio_files(id) ON DELETE SET NULL`.
- **[SAFETY-5]** `src/db/mod.rs` — `ALTER TABLE user_music_submissions ADD COLUMN IF NOT EXISTS
  audio_duration_seconds INT`.
- **[SAFETY-6]** `src/db/mod.rs` — 3 index phụ: `idx_music_submissions_source_type`,
  `idx_music_submissions_audio_file`, `idx_groups_logo_upload`.

### 🐛 Fix UX — Handler Error Handling

- **[UX-1]** `src/handlers/community.rs::change_group_logo` — Khi `UPDATE groups SET logo_upload_id`
  fail, thay vì trả plain-text 500 "Lỗi cập nhật logo nhóm." (UX tệ — trang trắng), giờ redirect
  về `/cong-dong/nhom/{slug}?err=...` (group page đã có error banner từ v0.9.37). Cleanup file
  PNG + image row đã tạo để tránh orphan rows. Log error chi tiết (group slug + image_id + error).
- **[UX-2]** `src/handlers/nha_nhac.rs::nha_nhac_submit_music_file` — Cải thiện error message
  từ "⚠️ Lỗi gửi bài — vui lòng thử lại." → "⚠️ Lỗi gửi bài — không thể lưu bài hát vào cơ sở dữ liệu.
  Vui lòng thử lại sau ít phút. Nếu lỗi vẫn tiếp diễn, hãy liên hệ admin kỹ thuật." Log error chi tiết.
- **[UX-3]** `src/handlers/nha_nhac.rs::nha_nhac_submit_music` (YouTube path) — Cải thiện error
  message tương tự [UX-2]. Log error với prefix "(YouTube)" để phân biệt.

### 🎨 Replace All Web Logos — tubi.png (NEW MAJOR)

User cung cấp `tubi.png` (PNG 1254×1254, RGBA, 1.25 MB). Thay toàn bộ logo web từ emoji 🪷
sang PNG thật:

- **[LOGO-1]** `src/static/tubi.png` — Copy file từ upload user vào static assets (Docker image
  sẽ copy vào `/app/static/tubi.png`).
- **[LOGO-2]** `src/static/og-image.png` — Overwrite social preview image với tubi.png (Facebook/
  Twitter/Telegram/Discord preview khi share URL).
- **[LOGO-3]** `templates/layout.html` — Favicon (tab logo): thay SVG data URI emoji 🪷 bằng
  `<link rel="icon" type="image/png" sizes="32x32" href="/static/tubi.png">` + 5 sizes (16/32/180/192/512)
  cho mọi platform (browser tab, taskbar, PWA, apple-touch-icon, splash screen). Giữ `favicon.svg`
  làm fallback cho trình duyệt cũ.
- **[LOGO-4]** `templates/layout.html` — Header logo: thay `<span class="lotus-logo-header">🪷</span>`
  trong green gradient box bằng `<img src="/static/tubi.png" class="w-9 h-9 rounded-xl object-cover">` (36×36).
- **[LOGO-5]** `templates/layout.html` — Bottom nav center button (mobile): thay `<span>🪷</span>`
  bằng `<img src="/static/tubi.png" class="w-12 h-12 -mt-4 rounded-full">` (48×48, nút nổi).
- **[LOGO-6]** `templates/layout.html` — Footer logo (desktop): thay `<span class="text-xl">🪷</span>`
  bằng `<img src="/static/tubi.png" class="w-6 h-6 rounded">` (24×24).
- **[LOGO-7]** `templates/layout.html` — OG image meta: `og:image` + `twitter:image` đổi từ
  `/static/og-image.png` (1200×630 cũ) sang `/static/tubi.png` (1254×1254 mới). Cập nhật
  `og:image:width` + `og:image:height` + `og:image:alt`.
- **[LOGO-8]** `templates/home.html` — Hero logo: thay `<span class="lotus-emoji text-5xl">🪷</span>`
  bằng `<img src="/static/tubi.png" class="w-16 h-16 rounded-full">` (64×64) trong white circle container.
- **[LOGO-9]** `templates/auth/login.html` — Top logo: thay `<span class="text-4xl">🪷</span>`
  bằng `<img src="/static/tubi.png" class="w-16 h-16 mx-auto rounded-2xl">` (64×64).
- **[LOGO-10]** `templates/gioi-thieu.html` — Hero logo (section 1): thay emoji span bằng
  `<img src="/static/tubi.png" class="w-16 h-16 rounded-full">` (64×64).
- **[LOGO-11]** `templates/tong-quan/index.html` — Tile "Giới Thiệu" logo: thay emoji div
  bằng `<img src="/static/tubi.png" class="w-12 h-12 rounded-full">` (48×48).
- **[LOGO-12]** `src/handlers/mod.rs` (placeholder/error pages) — Favicon (5 sizes PNG),
  header logo, footer logo, bottom nav center button — tất cả đổi sang `<img src="/static/tubi.png">`.
- **[LOGO-13]** `src/handlers/auth.rs` — Error page (login callback error) top logo: thay
  `<div class="text-5xl">🪷</div>` bằng `<img src="/static/tubi.png" class="w-16 h-16">`.
- **[LOGO-14]** `src/middleware/rate_limit.rs` — 429 Too Many Requests page: thay
  `<div class="emoji">🪷</div>` bằng `<img src="/static/tubi.png" style="width:64px;height:64px">`.

Lưu ý: emoji 🪷 vẫn được giữ trong text decorations (menu items, button text, footer copyright
text "🪷 Ứng Dụng Từ Bi v0.9.38 · Nguyện công đức vô lượng..."). Chỉ thay **logo positions**
(brand icon) bằng PNG, không thay emoji trang trí.

### 👥 About Page Team Update — /gioi-thieu

Cập nhật thông tin đội ngũ trên trang `/gioi-thieu` (section 4 "Founder Story") theo yêu cầu user:

- **[TEAM-1]** `templates/gioi-thieu.html` — Đổi card "Admin Kỹ Thuật (Cường)" → "Admin Kỹ Thuật (Hiếu)".
  Mô tả: "Phụ trách hệ thống, server, cơ sở dữ liệu, mã nguồn, bảo mật và phát triển ứng dụng.
  Nguyễn Đình Minh Hiếu từ Admin Cộng Đồng chuyển sang Admin Kỹ Thuật để phụ trách mảng kỹ thuật
  của dự án."
- **[TEAM-2]** `templates/gioi-thieu.html` — Thêm amber banner "Đỗ Văn Cường — đã rút lui về làm
  hỗ trợ" ngay dưới 2 card founder. Nội dung: "Đỗ Văn Cường, vốn là Admin Kỹ Thuật nòng cốt từ
  những ngày đầu, nay đã rút lui về làm hỗ trợ kỹ thuật. Nguyễn Đình Minh Hiếu — trước đây đảm
  nhận vai trò Admin Cộng Đồng — được điều chuyển sang vị trí Admin Kỹ Thuật để phụ trách chính
  mảng kỹ thuật của Ứng Dụng Từ Bi. Sự chuyển đổi này giúp dự án có người chịu trách nhiệm toàn
  thời gian cho hạ tầng, server và phát triển ứng dụng."
- **[TEAM-3]** `templates/gioi-thieu.html` — Section 7 (Tuyển thành viên): Đổi "Admin Cường
  thiết kế và xây dựng Thư Viện" → "Admin Hiếu thiết kế và xây dựng Thư Viện" (Thư Viện Kinh Sách).
- Lưu ý: Trang `/doi-ngu-quan-li` (đội ngũ quản lí chi tiết) đã có sẵn thông tin đúng từ v0.9.30
  (Đỗ Văn Cường — "Hiện tại đã lui về hỗ trợ", Nguyễn Đình Minh Hiếu — "Hiện tại đang làm chính").
  Không cần sửa.

### 📦 Version Sync v0.9.38

- Bump version `0.9.37` → `0.9.38` ở: `Cargo.toml`, `src/main.rs` (startup log + health check
  public + health check inner + phase 41 → 42), `templates/layout.html` (footer), `templates/khong-gian/index.html`
  (footer), `templates/admin/cong-dong/index.html` (footer), `templates/admin/quan-li/index.html`
  (footer), `templates/admin/ky-thuat/index.html` (title + footer), `templates/admin/placeholder.html`
  (title + footer), `templates/admin/phat-trien/index.html` (phase badge + roadmap + footer),
  `src/middleware/rate_limit.rs` (429 page footer), `Dockerfile.coolify` (comment).
- Update phase 41 → 42 trong health check + main log + admin phat-trien dashboard.
- Thêm 16 feature flags v0.9.38 vào `HEALTH_FEATURES` array (logo-png-replace-emoji-* × 8 +
  group-logo-safety-schema-fix + music-submit-safety-schema-fix + audio-files-table-safety-schema +
  music-submissions-source-type-safety-schema + group-logo-error-redirect-with-err +
  music-submit-error-message-improved + about-page-team-update-cuong-hieu).
- Thêm `v0_9_38_note` vào health check `notes` object.
- Cập nhật roadmap `/admin/phat-trien`: Giai đoạn 41 (About Page + Orphan Fix + Post-Fix + Notif + 429)
  → "Hoàn thành" (green). Giai đoạn 42 (Logo PNG + Group Logo Fix + Music Submit Fix + Team Update)
  → "Đang triển khai" (indigo, badge 43).

### 📋 Ghi chú vận hành

- **Database**: Safety schema check chạy idempotent DDL trước sqlx migrations → đảm bảo schema
  luôn nhất quán ngay cả khi migration 026 chưa được apply (checksum mismatch, partial deploy,
  manual rollback). KHÔNG xóa dữ liệu user, chỉ ADD COLUMN IF NOT EXISTS / CREATE TABLE IF NOT EXISTS.
- **Logo file**: `tubi.png` (1254×1254 PNG, 1.25 MB) được copy vào `src/static/tubi.png`. Docker
  image copy vào `/app/static/tubi.png`. Browser cache: file tĩnh có ETag qua tower-http ServeDir,
  nhưng không có content hash trong filename → user có thể cần hard-refresh (Ctrl+Shift+R) lần đầu
  để thấy logo mới. Sau đó browser cache sẽ hoạt động bình thường.
- **Emoji 🪷 trong text**: Vẫn được giữ trong menu items, button text, copyright text — chỉ thay
  logo positions (brand icon) bằng PNG. Nếu user muốn thay toàn bộ emoji 🪷 (kể cả text decorations),
  cần yêu cầu riêng.
- **Rollback**: Nếu logo PNG có vấn đề, có thể rollback bằng cách revert commit này. Fallback
  SVG (`favicon.svg`) vẫn còn → trình duyệt cũ không bị broken icon.

---

## [0.9.37] — 2026-08-16 — Giai đoạn 41 (phần 2): About Page + Orphan-Link Fix + Post-Submit Fix + Notification Mark-All + 429 Hardening 🪷

### 🎯 Mục tiêu giai đoạn

Bản phát hành này là phần **bổ sung và sửa lỗi** cho Giai đoạn 41 (v0.9.36), tập trung
vào **5 vấn đề user báo cáo nhiều nhất**:

1. **"Trang mồ côi" / thiếu liên kết nội bộ** — nhiều route hợp lệ (như `/cai-dat`,
   `/khong-gian/nha-nhac`, `/thuong-thanh`, `/admin/nha-nhac/dang-cho-duyet`...) không
   có nút bấm / menu / link nào dẫn đến trên giao diện, đặc biệt là mobile drawer.
2. **Thiếu trang giới thiệu chi tiết** — không có trang `/gioi-thieu` tổng hợp sứ mệnh,
   tầm nhìn, triết lý, tính năng, tuyển thành viên.
3. **Lỗi "gửi bài" không rõ nguyên nhân** — pending member hoặc khi DB error, user bị
   redirect về trang nhóm mà KHÔNG có thông báo gì → tưởng form hỏng.
4. **Không có nút "đánh dấu đã đọc" trong thông báo** — chỉ có auto-mark-on-visit,
   không có per-item button, không có mark-all-as-read.
5. **Lỗi 429 Too Many Requests xuất hiện nhiều khi đổi tab** — limit quá thấp (60/phút
   cho API, 120/phút cho general), classification sai (`/api/ban-be/*` rơi vào `api`
   thay vì `social`), response trả plain-text không có nút quay lại.

### 📖 Trang Giới Thiệu — `/gioi-thieu` (NEW MAJOR)

- **[ABOUT-1] `src/handlers/mod.rs`** — Thêm `GioiThieuTemplate` struct + `gioi_thieu`
  handler. Trang tĩnh, công khai (không yêu cầu login), nội dung dài.

- **[ABOUT-2] `templates/gioi-thieu.html`** — Trang `/gioi-thieu` với **9 section**:
  1. Hero — "ỨNG DỤNG TỪ BI · CỘNG TU · KẾT NỐI · PHỤNG SỰ" + quote "Trong cuộc sống
     hiện đại, con người có thể kết nối..."
  2. Mục lục (TOC) — quick-jump tới 9 section.
  3. Ứng Dụng Từ Bi là gì? — định vị, đối tượng, vấn đề giải quyết.
  4. Tầm nhìn tối thượng — "Hãy mở Ứng Dụng Từ Bi."
  5. Triết lý & khẩu hiệu — 4 triết lý (cốt lõi / khẩu hiệu / phát triển / game /
     cuối cùng).
  6. Câu chuyện khởi sáng — founder story (Admin Định Hướng Ti + Admin Kỹ Thuật Cường).
  7. Tính năng chính — 4 chuyên mục cards + 10 bullet points + link tới các trang.
  8. Hệ sinh thái tương lai — Game Siêu Độ, AI Từ Bi, Học Viện, Hỗ Trợ Người Tu,
     Thiện Nguyện, Toàn Cầu Hoá.
  9. Thông báo tuyển thành viên — 5 vị trí (Kinh Sách, Nhạc, Website, Fanpage,
     Tặng Sách) + 8 cách đồng hành + CTA.

- **[ABOUT-3] `src/main.rs`** — Đăng ký route `.route("/gioi-thieu", get(handlers::gioi_thieu))`.

- **[ABOUT-4]** Nội dung trích dẫn trực tiếp từ 6 file `.docx` trong thư mục
  `HieuLouis/` để giữ tinh thần dự án (xem `HieuLouis/Giới thiệu về ứng dụng từ bi.docx`,
  `Kế Hoạch Và Mục Tiêu Phát Triển Dự Án Từ Bi.docx`, `THÔNG BÁO TUYỂN THÀNH VIÊN.docx`).

### 🔗 Orphan-Link Fix — Bổ sung menu/link cho các trang mồ côi (MAJOR)

Phân tích 72 GET routes + 50+ template files phát hiện:

- **1 orphan thực sự:** `/admin/nha-nhac/dang-cho-duyet` — admin dashboard không có
  tile/link nào dẫn đến trang duyệt nhạc cộng đồng. Admin phải biết URL bằng lòng.
- **Mobile drawer quá sparse:** Chỉ có 5 link chính (Không Gian, Cộng Đồng, Bạn Bè,
  Kinh Sách, Hồ Sơ). Các route `/cai-dat`, `/thuong-thanh`, `/quy-tu-bi`, `/bang-xep-hang`,
  `/thanh-tich`, `/doi-ngu-quan-li`, `/tim-kiem`, `/khong-gian/nha-nhac`, `/ban-be/tin-nhan`,
  `/ban-be/thu`, `/ban-be/thong-bao`, `/ban-be/tim-kiem`, `/cong-dong/tao-nhom`,
  `/kinh-sach/thu-vien/*`, `/kinh-sach/tim-kiem`, `/gioi-thieu` đều KHÔNG có link
  trên mobile drawer → user mobile không thể click đến.

**Fix:**

- **[ORPHAN-1] `templates/layout.html`** — Mở rộng mobile drawer từ 5 items → **27 items**,
  chia 3 nhóm: "Chuyên Mục" (Không Gian + Nhà Nhạc, Cộng Đồng + Tạo Nhóm, Bạn Bè +
  Tin Nhắn/Hộp Thư/Thông Báo/Tìm Bạn, Kinh Sách + Phật Gia/Đạo Gia/Tìm Sách),
  "Khám Phá" (Giới Thiệu MỚI, Tổng Quan, Quỹ Từ Bi, BXH, Thành Tích, Thương Thành,
  Đội Ngũ, Tìm Kiếm), "Tài Khoản" (Hồ Sơ, Cài Đặt, Quản Trị, Theme toggle, Đăng xuất).

- **[ORPHAN-2] `templates/layout.html`** — Desktop mega-menu Col 1 "Hệ Thống" thêm:
  `🪷 Giới Thiệu` (đầu tiên), `🎵 Nhà Nhạc` (sau Tổng Quan).

- **[ORPHAN-3] `templates/layout.html`** — Footer "Hệ Thống" column thêm `Giới Thiệu`
  + `Nhà Nhạc`.

- **[ORPHAN-4] `templates/home.html`** — Thêm `/gioi-thieu` card ở "Khám Phá Thêm"
  section (badge MỚI) + `/khong-gian/nha-nhac` card. Thêm link "📖 Đọc giới thiệu
  chi tiết →" ở section "Bốn Chuyên Mục Chính".

- **[ORPHAN-5] `templates/tong-quan/index.html`** — Thêm bannerfeatured `/gioi-thieu`
  card giữa "4 Chuyên Mục" và "Hệ Thống".

- **[ORPHAN-6] `templates/admin/ky-thuat/index.html`** — Thêm tile `🎵 Duyệt Nhạc
  Cộng Đồng` dẫn tới `/admin/nha-nhac/dang-cho-duyet` (trước đây là orphan thực sự).

### ✍️ Fix "gửi bài không được" (MAJOR)

Phân tích `src/handlers/community.rs::create_topic` + `create_comment`:

**BUG 1 (P0): Silent rejection của pending/banned member.**
- Trước đây: `create_topic_form` và `create_topic` check `membership.status != "active"`
  → `Redirect::to("/cong-dong/nhom/{slug}")` không có query param → user thấy trang
  nhóm như bình thường, không hiểu vì sao nút "Tạo Chủ Đề" không hoạt động.
- Fix: redirect với `?err=ban-cho-duyet` / `ban-bi-khoa` / `ban-da-roi-nhom` /
  `ban-chua-tham-gia-nhom`. `view_group` parse query param và render banner error
  rõ ràng (vd: "Đơn tham gia nhóm của bạn đang chờ Trưởng Nhóm / Admin duyệt.

**BUG 2 (P1): `group_name` rỗng khi validation error.**
- Trước đây: khi title empty / quá 200 ký tự, handler re-render `CreateTopicTemplate`
  với `group_name: String::new()` → breadcrumb "Đăng trong — " blank.
- Fix: fetch `group_name` từ DB cùng lúc với `group_id`, dùng cho cả success và
  error path. Validation error path cũng render form với ĐÚNG `group_name`.

**BUG 3 (P1): DB error trả plain-text 500.**
- Trước đây: `INSERT INTO topics ... RETURNING id` fail → trả
  `(StatusCode::INTERNAL_SERVER_ERROR, "Lỗi tạo chủ đề")` → user thấy trang trắng,
  mất toàn bộ nội dung đã gõ.
- Fix: render lại `CreateTopicTemplate` với error message chi tiết + `group_name`
  đúng + title đã nhập (để user copy + retry).

**BUG 4 (P1): Stale counter `groups.topic_count` và `topics.comment_count`.**
- Trước đây: handler INSERT topic/comment nhưng KHÔNG update counter → badge
  "📝 N chủ đề" trên `group.html` stale cho đến khi có trigger/background job.
- Fix: thêm `UPDATE groups SET topic_count = topic_count + 1 WHERE id = $1` sau
  INSERT topic, và `UPDATE topics SET comment_count = comment_count + 1 WHERE id = $1`
  sau INSERT comment.

**BUG 5 (P1): `create_comment` trả plain-text cho mọi error path.**
- Trước đây: validation error → `(400, "Bình luận không hợp lệ.")`, locked topic →
  `(403, "Chủ đề đã bị khoá.")`, DB error → `(500, "Lỗi đăng bình luận.")`. Tất cả
  đều plain-text, không có nav / nút quay lại.
- Fix: tất cả error path redirect về `/cong-dong/chu-de/{id}?err=...` để `view_topic`
  render lại topic page + banner error rõ ràng.

- **[POST-1]** `src/handlers/community.rs::create_topic_form` — redirect với err param.
- **[POST-2]** `src/handlers/community.rs::create_topic` — fetch `group_name` từ DB,
  validation path render form với `group_name` đúng, DB error path render form với
  error message + title đã nhập.
- **[POST-3]** `src/handlers/community.rs::create_topic` — update `topic_count` sau INSERT.
- **[POST-4]** `src/handlers/community.rs::create_comment` — redirect với err param
  thay vì plain-text status code. Update `comment_count` sau INSERT.
- **[POST-5]** `GroupTemplate` + `TopicTemplate` thêm field `error: Option<String>`.
- **[POST-6]** `view_group` + `view_topic` thêm `Query<HashMap<String, String>>`
  extractor, parse `?err=...` thành user-friendly message.
- **[POST-7]** `templates/community/group.html` + `topic.html` — render error banner
  (red alert box) khi `error.is_some()`.

### 🔔 Notification Mark-All-As-Read + Per-Item Button (MAJOR)

Trước v0.9.37, `/ban-be/thong-bao` chỉ có **auto-mark-on-visit** (server UPDATE tất
cả unread khi user mở trang). Endpoint `/api/ban-be/thong-bao/{id}/da-doc` đã tồn
tại nhưng là **dead code** — không có UI nào gọi.

**Fix:**

- **[NOTIF-1] `src/handlers/friends.rs::mark_all_notifications_read`** — Endpoint mới
  `POST /api/ban-be/thong-bao/da-doc-tat-ca`. Bulk UPDATE tất cả unread của user,
  trả JSON `{status: "ok", marked_count: N}`. Không có giới hạn số lượng.

- **[NOTIF-2] `src/main.rs`** — Đăng ký route
  `.route("/api/ban-be/thong-bao/da-doc-tat-ca", post(handlers::friends::mark_all_notifications_read))`.

- **[NOTIF-3] `templates/ban-be/notifications.html`** — UI overhaul:
  - Nút "✓ Đánh dấu tất cả đã đọc" ở header (disabled khi đang xử lý).
  - Per-item button "✓ Đánh dấu đã đọc" trên mỗi notification chưa đọc.
  - Toast thông báo "✅ Đã đánh dấu N thông báo là đã đọc." sau khi thành công.
  - Alpine.js `notificationsManager()` component quản lý state `readState[uuid]`.
  - Cập nhật header badge ngay lập tức qua `window.__tubiSetNotificationBadge(0)`.
  - Handle 429: toast "⏳ Quá nhiều request. Vui lòng thử lại sau vài giây."

- **[NOTIF-4] `src/static/js/chat.js::notificationBadge()`** — Expose 2 global helpers:
  - `window.__tubiSetNotificationBadge(count)` — set badge to specific value.
  - `window.__tubiDecrementNotificationBadge()` — decrement by 1 (per-item mark-as-read).
  - Lắng nghe custom event `tubi-notifications-changed` để refresh badge ngay lập tức
    (trước đây phải chờ 60s poll).

### 🚫 Fix 429 Too Many Requests (MAJOR)

Phân tích `src/middleware/rate_limit.rs` + `src/static/js/chat.js` + `app.js`:

**Tăng limit (giảm 429 false positive):**
- `api` group: 60 → **180** req/phút (stats, history, preferences, music tracks API).
- `social` group: 60 → **180** req/phút (DM, notifications, friend ops).
- `general` group: 120 → **300** req/phút (HTML pages, static files).
- `post` group: 30 → **60** req/phút (create topic, comment, cam-ngo, tang-hoa).
- Block penalty: 60s → **30s** (cân bằng anti-spam vs UX).
- `auth` / `upload` / `profile_update` giữ nguyên (security limits, không nới lỏng).

**Fix classification bugs (3 bugs):**
- **BUG A:** `/api/ban-be/*` trước đây rơi vào `api` (60/min). Sửa → `social` (180/min).
  Lý do: DM + notification poll + friend search là social features, không nên share
  budget với stats API.
- **BUG B:** `/api/nha-nhac/dang-nhac` + `/api/nha-nhac/dang-nhac-file` trước đây rơi
  vào `api`. Sửa → `upload` (10/min). Lý do: đây là upload operation (YouTube URL
  submit hoặc audio file upload), nên vào stricter bucket.
- **BUG C:** `/kinh-sach/{slug}/cam-ngo` + `/kinh-sach/{slug}/tang-hoa` trước đây rơi
  vào `general` (120/min) — quá dễ spam. Sửa → `post` (60/min).

**429 response overhaul:**
- Trước đây: trả plain-text `"429 — Quá nhiều request. Vui lòng thử lại sau 60 giây. 🪷"`
  → user thấy trang trắng, không có nav / nút quay lại.
- Fix:
  - **HTML page** cho browser navigation: layout tối giản, countdown timer 30s,
    hiển thị "📝 Nhóm bị giới hạn", "⚡ Giới hạn: 180 request/phút", "🔗 Đường dẫn:
    /api/...", nút "← Quay lại" (disabled cho đến khi countdown = 0).
  - **JSON response** cho `fetch()` calls (Accept: application/json hoặc
    X-Requested-With: XMLHttpRequest): `{error: "rate_limited", message: "...",
    retry_after: 30, group: "api"}` → client-side JS có thể handle.

**Client-side 429 hardening:**
- **[429-1] `src/static/js/app.js`** — `window.tubiFetch(url, opts)` wrapper: tự catch
  HTTP 429, đọc `Retry-After` header, hiển thị toast, reject với error có `.retryAfter`
  + `.group` + `.isRateLimited` properties.
- **[429-2] `src/static/js/app.js`** — `window.tubiToast(message, type)` global toast
  system (info/success/warning/error). Auto-dismiss 4s.
- **[429-3] `src/static/js/app.js`** — Pause polling khi `document.hidden = true`
  (tab không visible). Resume khi tab visible lại — dispatch custom event
  `tubi-tab-visible` để `notificationBadge()` fetch ngay lập tức.
- **[429-4] `src/static/js/chat.js::notificationBadge()`** — `scheduledFetch()` check
  `window.__tubiPollingPaused` trước khi poll. Nếu 429 → pause poll 30s rồi resume.
  Lắng nghe `tubi-tab-visible` event để fetch ngay khi user quay lại tab.

### 🛠️ Misc

- **Bump version** `0.9.36` → `0.9.37` trong `Cargo.toml`, `src/main.rs` (log lines,
  health check JSON, phase_name), `templates/layout.html` (footer),
  `templates/admin/phat-trien/index.html` (badge, footer), `templates/admin/ky-thuat/index.html`,
  `templates/admin/quan-li/index.html`, `templates/admin/cong-dong/index.html`,
  `templates/admin/placeholder.html`, `templates/admin/cong-dong/cam-ngo.html`,
  `templates/khong-gian/index.html`, `templates/khong-gian/nha-nhac.html`.
- **Rust 1.97.1** — `Cargo.toml` đã specify `rust-version = "1.97.1"`. Verified build
  pass với `rustc 1.97.1 (8bab26f4f 2026-07-14)`.
- **Health check** `/api/health` thêm 27 feature flags mới cho v0.9.37 + `v0_9_37_note`.

---

## [0.9.36] — 2026-08-16 — Giai đoạn 41: Community Group Logo + Audio File Uploads 🪷

### 🎯 Mục tiêu giai đoạn

Triển khai **2 tính năng mới** cho Giai đoạn 41:

1. **Đổi logo cộng đồng** — Cho phép chủ nhóm/admin upload logo riêng (icon vuông nhỏ,
   khác với ảnh bìa banner). Trước v0.9.36, nhóm cộng đồng chỉ có thể đổi ảnh bìa
   (`cover_upload_id`); icon đại diện nhóm là emoji cố định theo category. Giờ v0.9.36
   thêm `logo_upload_id` riêng để nhóm có thể chọn logo tuỳ ý (PNG/JPG/WebP/GIF).

2. **Tải file âm thanh (MP3/M4A/OGG/WAV/FLAC) trong Nhà Nhạc** — Bổ sung cho nguồn
   YouTube hiện có (v0.9.35). Theo tài liệu "Hệ Thống Và Chức Năng Chi Tiết.docx" mục 3
   (Nhà Nhạc): *"Cá nhân là danh sách nhạc do thành viên tải lên từ điện thoại hoặc thêm
   từ kho nhạc miễn phí của hệ thống."* Trước v0.9.36, user chỉ có thể đăng nhạc qua link
   YouTube; giờ v0.9.36 user có thể upload file âm thanh trực tiếp (tối đa 20 MB/file).

### 🎨 Community Group Logo (MAJOR)

- **[LOGO-1] `migrations/026_community_logo_and_audio_files.sql`** — Thêm cột
  `logo_upload_id UUID REFERENCES images(id) ON DELETE SET NULL` vào bảng `groups`.
  Index `idx_groups_logo_upload` (partial WHERE NOT NULL).

- **[LOGO-2] `src/models/community.rs`** — Thêm field `logo_upload_id: Option<Uuid>`
  vào struct `Group` với `#[sqlx(default)]` (backward compat với DB chưa migrate).

- **[LOGO-3] `src/handlers/community.rs`** — Thêm field `logo_image_url: Option<String>`
  vào `GroupTemplate` struct. Trong `view_group`, fetch logo URL qua
  `SELECT i.stored_filename FROM images i JOIN groups g ON g.logo_upload_id = i.id`.

- **[LOGO-4] `src/handlers/community.rs`** — Thêm handler `change_group_logo`:
  - Route: `POST /cong-dong/nhom/{slug}/doi-logo`
  - Permission: chỉ owner hoặc admin của nhóm
  - Accept: multipart form `file` (image/jpeg, image/png, image/webp, image/gif)
  - Lưu file vào `upload_dir/<uuid>.<ext>`, insert metadata vào `images` table,
    update `groups.logo_upload_id`.
  - Validate MIME, compute SHA-256, parse dimensions (giống `change_group_cover`).

- **[LOGO-5] `templates/community/group.html`** — Hiển thị logo:
  - Nếu `logo_image_url` có giá trị → render `<img src=logo_url>` trong khung 16×16
    rounded-2xl với border + shadow.
  - Nếu không → fallback về `group.category_icon_or_lotus()` (emoji theo category).
  - Thêm nút "🎨 Đổi logo" (violet, distinct từ "📷 Đổi ảnh bìa" green) cho owner/admin.

- **[LOGO-6] `src/main.rs`** — Route mới:
  ```
  POST /cong-dong/nhom/{slug}/doi-logo → handlers::community::change_group_logo
  ```

### 🎵 Audio File Upload trong Nhà Nhạc (MAJOR)

- **[AUDIO-1] `migrations/026_community_logo_and_audio_files.sql`** — Tạo bảng
  `audio_files` mới (metadata file âm thanh do user upload):
  - `id UUID PRIMARY KEY DEFAULT gen_random_uuid()`
  - `uploader_id UUID REFERENCES users(id) ON DELETE SET NULL`
  - `original_name VARCHAR(255) NOT NULL`
  - `stored_filename VARCHAR(255) NOT NULL UNIQUE` (= `<uuid>.<ext>`)
  - `mime_type VARCHAR(100) NOT NULL`
  - `size_bytes BIGINT NOT NULL`
  - `sha256 VARCHAR(64) NOT NULL` (checksum chống trùng)
  - `duration_seconds INT` (thời lượng, ước lượng từ bitrate)
  - `purpose VARCHAR(50) NOT NULL DEFAULT 'other'`
    (`music_submission` | `personal_track` | `other`)
  - `is_public BOOLEAN NOT NULL DEFAULT true`
  - Indexes: `idx_audio_files_uploader`, `idx_audio_files_purpose`, `idx_audio_files_sha256`.
  - Bảng `migration_log` cũng được tạo `IF NOT EXISTS` (safety cho migration 025).

- **[AUDIO-2] `migrations/026_community_logo_and_audio_files.sql`** — Thêm 3 cột vào
  `user_music_submissions`:
  - `source_type TEXT NOT NULL DEFAULT 'youtube' CHECK (source_type IN ('youtube', 'audio_file'))`
  - `audio_file_upload_id UUID REFERENCES audio_files(id) ON DELETE SET NULL`
  - `audio_duration_seconds INT` (thời lượng ước lượng)
  - Indexes: `idx_music_submissions_source_type`, `idx_music_submissions_audio_file`.

- **[AUDIO-3] `src/handlers/uploads.rs`** — Thêm helpers cho audio:
  - `ALLOWED_AUDIO_MIME` const — 12 MIME types: audio/mpeg, audio/mp3, audio/mp4,
    audio/x-m4a, audio/m4a, audio/ogg, audio/vorbis, audio/wav, audio/x-wav,
    audio/wave, audio/flac, audio/x-flac.
  - `MAX_AUDIO_BYTES` const = 20 × 1024 × 1024 (20 MB).
  - `audio_mime_to_ext(mime)` — trả về extension (mp3, m4a, ogg, wav, flac).
  - `read_multipart_audio_file(multipart, max_bytes)` — read multipart fields
    (file binary + text fields title/artist/category/description). Trả về
    `(Bytes, Option<String>, Option<String>, HashMap<String, String>)`.
  - `insert_audio_metadata(pool, file_id, uploader_id, original_name, stored_filename,
    mime, file_bytes, sha256, duration_seconds, purpose)` — insert vào `audio_files`.
  - `estimate_audio_duration_seconds(file_bytes_len, mime)` — ước lượng thời lượng
    từ byte size + bitrate trung bình theo format (MP3 128k, AAC 128k, Vorbis 112k,
    WAV PCM 1411k, FLAC 800k). Sai số có thể lớn — trong tương lai thay bằng
    crate `symphonia` để parse chính xác.

- **[AUDIO-4] `src/models/nha_nhac.rs`** — Cập nhật `UserMusicSubmission` struct:
  - Thêm field `source_type: String` (default `'youtube'`)
  - Thêm field `audio_file_upload_id: Option<Uuid>`
  - Thêm field `audio_duration_seconds: Option<i32>`
  - Method mới: `is_audio_file()`, `duration_display()`.
  - Tất cả đều có `#[sqlx(default)]` để backward compat với DB chưa migrate.

- **[AUDIO-5] `src/models/nha_nhac.rs`** — Cập nhật `SubmissionWithUser` struct
  (cho admin view):
  - Thêm `source_type`, `audio_file_upload_id`, `audio_duration_seconds`,
    `audio_stored_filename` (JOIN từ audio_files).
  - Method mới: `is_audio_file()`, `youtube_embed_url()`, `duration_display()`,
    `source_icon()`, `source_label()`.

- **[AUDIO-6] `src/handlers/nha_nhac.rs`** — Thêm handler `nha_nhac_submit_music_file`:
  - Route: `POST /api/nha-nhac/dang-nhac-file`
  - Accept: multipart form (file + title + artist + category + description)
  - Validate: file không rỗng, MIME audio hợp lệ, title ≤ 200 chars, artist ≤ 100 chars,
    category ∈ {niem, thien, dao, khong_loi}, description ≤ 500 chars.
  - Rate limit: max 5 submissions per user per day (same as YouTube).
  - Dedup: check SHA-256 trùng với audio_files user đã upload.
  - Lưu file vào `upload_dir/<uuid>.<ext>`, insert vào `audio_files`, insert vào
    `user_music_submissions` với `source_type='audio_file'`.
  - Trả về HTMX partial (success/error message).
  - Cleanup: nếu insert DB fail, xóa file đã ghi + xóa row audio_files.

- **[AUDIO-7] `src/handlers/nha_nhac.rs`** — Cập nhật `admin_music_pending`:
  - JOIN `audio_files` để lấy `stored_filename` cho audio preview.
  - Trả về `SubmissionWithUser` với `audio_stored_filename` field.

- **[AUDIO-8] `src/handlers/nha_nhac.rs`** — Cập nhật `admin_music_review`:
  - Khi approve: nếu `source_type='audio_file'`, build `audio_url` từ local file
    (`upload_url_prefix + stored_filename`) thay vì YouTube URL.
  - Insert vào `music_tracks` với `audio_url` local + `duration_seconds` từ submission.

- **[AUDIO-9] `templates/khong-gian/nha-nhac.html`** — Cập nhật modal "Đăng Nhạc":
  - Đổi nút "🎵 Đăng Nhạc (YouTube)" → "🎵 Đăng Nhạc Cộng Đồng".
  - Thêm tab switcher (Alpine.js `x-data="{ submitMode: 'youtube' }"`):
    - Tab "▶️ Link YouTube" (amber accent) — form YouTube hiện có (unchanged).
    - Tab "🎵 Tải file MP3" (violet accent) — form upload file mới.
  - Form upload file: title, artist, category, file (accept audio/*), description.
    File input có Tailwind `file:` classes (violet button).
  - Submit button: "🎵 Tải lên file nhạc" (violet).
  - HTMX `hx-encoding="multipart/form-data"` cho form upload.

- **[AUDIO-10] `templates/admin/nha-nhac-pending.html`** — Cập nhật admin review UI:
  - Header text: "...YouTube hoặc tải file âm thanh (MP3/M4A/OGG/WAV/FLAC)".
  - Preview area: nếu `sub.is_audio_file()` → render `<audio controls>` với source
    là `/uploads/<stored_filename>`. Nếu không → render YouTube iframe như cũ.
  - Source badge: "▶️ YouTube" (red) hoặc "🎵 File âm thanh" (violet).
  - Footer info: hiển thị duration cho audio file.

### 📦 Version Sync v0.9.36

- **[VER-1] `Cargo.toml`** — `version = "0.9.36"`.
- **[VER-2] `src/main.rs`** — Startup log v0.9.36 + phase 41 + health check version/phase.
- **[VER-3] `templates/layout.html`** — Footer: v0.9.36.
- **[VER-4] `templates/khong-gian/index.html`** — Footer: v0.9.36.
- **[VER-5] `templates/khong-gian/nha-nhac.html`** — Footer note: v0.9.36 · Giai đoạn 41.
- **[VER-6] `Dockerfile.coolify`** — Comment: v0.9.36 — Giai đoạn 41.
- **[VER-7] `templates/admin/phat-trien/index.html`** — Phase badge: GIAI ĐOẠN 41.
  Roadmap: 40 → "Hoàn thành" (green), 41 → "Đang triển khai" (indigo,
  "Community Logo + Audio File Uploads"). Footer: v0.9.36.
- **[VER-8] 5 admin templates** — Footer version sync: `templates/admin/{ky-thuat,
  quan-li, cong-dong/index, cong-dong/cam-ngo, placeholder}.html` — version → v0.9.36.

### 📋 Health Check Updates

- `HEALTH_FEATURES` thêm 13 features mới:
  - `community-group-logo-upload-v0.9.36`
  - `group-logo-change-endpoint-v0.9.36`
  - `audio-files-table-v0.9.36`
  - `music-audio-file-upload-mp3-v0.9.36`
  - `music-audio-file-upload-m4a-v0.9.36`
  - `music-audio-file-upload-ogg-v0.9.36`
  - `music-audio-file-upload-wav-v0.9.36`
  - `music-audio-file-upload-flac-v0.9.36`
  - `music-audio-20mb-limit-v0.9.36`
  - `music-audio-duration-estimate-v0.9.36`
  - `music-audio-sha256-dedup-v0.9.36`
  - `music-source-type-youtube-or-audio-v0.9.36`
  - `admin-music-pending-shows-source-type-v0.9.36`
- `khong_gian.features` thêm `nha-nhac-audio-file-upload`.
- `khong_gian.nha_nhac` thêm `submission_sources: ["youtube", "audio_file"]`,
  `audio_formats: ["mp3", "m4a", "ogg", "wav", "flac"]`, `audio_max_bytes: 20971520`.
- `cong_dong` object mới (status, features, group_logo_route).
- `v0_9_36_note` — Mô tả thay đổi giai đoạn 41.
- Phase 40 → 41 trong health check + main log.

### 🛠️ Yêu cầu kỹ thuật

- **Rust 1.97.1** — đã pin trong `Cargo.toml` (`rust-version = "1.97.1"`) và
  `Dockerfile` (`FROM rust:1.97.1-slim-bookworm`).
- **PostgreSQL 17** — migration 026 sử dụng `ALTER TABLE ADD COLUMN IF NOT EXISTS`,
  `CREATE TABLE IF NOT EXISTS`, `CHECK` constraint, `REFERENCES` FK.
- **axum 0.8** — `Multipart` extractor cho upload file. Route chain `get().post()`.
- **askama 0.14** — template `khong-gian/nha-nhac.html` và `community/group.html`
  extends `layout.html`.
- **HTMX** — `hx-post`, `hx-target`, `hx-swap`, `hx-encoding="multipart/form-data"`
  cho form upload không reload page.
- **Alpine.js** — `x-data="{ submitMode: 'youtube' }"` cho tab switcher trong modal.

### 🧪 Test scenarios

- Owner/admin nhóm vào `/cong-dong/nhom/{slug}` → thấy nút "🎨 Đổi logo" → chọn ảnh
  PNG/JPG → submit → logo hiện ở header nhóm (thay emoji category).
- User vào `/khong-gian/nha-nhac` → bấm "🎵 Đăng Nhạc Cộng Đồng" → modal hiện 2 tab
  (YouTube / Tải file MP3) → chọn tab "Tải file MP3" → điền title/artist/category →
  chọn file MP3 → submit → thông báo "Đã tải lên file âm thanh! Chờ admin duyệt."
- Admin vào `/admin/nha-nhac/dang-cho-duyet` → thấy submission mới với badge
  "🎵 File âm thanh" (violet) → preview audio player → bấm "✅ Duyệt" → submission
  approved → xuất hiện trong `music_tracks` với audio_url local → user có thể play
  trong Nhà Nhạc.

---

## [0.9.33] — 2026-08-16 — Giai đoạn 38: Nhà Nhạc (Music House — KG-03) + Logo Emoji Sharpened 🪷

### 🎯 Mục tiêu giai đoạn

Triển khai **Nhà Nhạc** — phòng KG-03 trong Không Gian (theo tài liệu "Hệ Thống Và Chức Năng Chi Tiết.docx" mục 3). Đây là giai đoạn tiếp theo theo roadmap đã công bố ở v0.9.32: `37 (đang triển khai) · 38 (Nhà Nhạc) · 39 (Thương Thành) · 40 (Game Siêu Độ)`.

**Nhà Nhạc** là nơi mở nhạc + cài đặt chế độ nghe nhạc, gồm:
- 5 thư mục: Nhạc Niệm · Nhạc Thiền · Nhạc Đạo · Không Lời · Cá Nhân
- 4 chế độ phát: Một bài liên tục · Ngẫu nhiên liên tục · Lặp lại liên tục · Lặp lại một vòng
- Hẹn thời gian tắt (sleep timer 15/30/60 phút)
- Playlist Cá Nhân — user add track hệ thống vào danh sách riêng

Đồng thời **làm nét logo emoji 🪷** theo yêu cầu user: giữ nguyên emoji (không đổi thành hình khác) nhưng render sắc nét hơn — bump viewBox 100→256, thêm font-family fallback (Apple/Segoe/Noto/Twemoji), `text-rendering="geometricPrecision"`, `shape-rendering="geometricPrecision"`, `-webkit-font-smoothing: antialiased`.

### 🎵 Nhà Nhạc (Music House) — `/khong-gian/nha-nhac` (MAJOR)

- **[MUSIC-1] `migrations/023_nha_nhac.sql`** — Tạo schema cho Nhà Nhạc:
  - `music_tracks` — kho nhạc hệ thống (id, title, category, description, artist, audio_url, duration_seconds, cover_url, is_public, upload_user_id, sort_order, is_active, play_count, created_at, updated_at). CHECK constraint cho `category` ∈ {niem, thien, dao, khong_loi}.
  - `user_music_prefs` — preferences per-user (user_id PK, playback_mode CHECK ∈ {single_repeat, shuffle, repeat_all, loop}, volume CHECK 0–100, sleep_timer_minutes nullable, last_track_id nullable, updated_at).
  - `user_personal_tracks` — playlist Cá Nhân (id, user_id, track_id, sort_order, added_at) với UNIQUE(user_id, track_id) chống duplicate.
  - Indexes: `idx_music_tracks_category_sort` (category, sort_order, id) WHERE is_active AND is_public — query nhanh cho category browse. `idx_music_tracks_upload_user` + `idx_user_personal_tracks_user`.
  - Seed 12 track mẫu (3 per category × 4 category): Nam Mô A Di Đà Phật, Lục Tự Đại Minh Chú, Thiền Chuông Tây Tạng, Mưa Nhẹ Rơi Trên Lá Sen, Hymn To The Lotus, Đường Về Tịnh Độ, Mộc Tần, Trúc Điếu, Cổ Cầm...
  - Idempotent: tất cả CREATE TABLE IF NOT EXISTS + INSERT ... ON CONFLICT DO NOTHING.

- **[MUSIC-2] `src/models/nha_nhac.rs`** — Models:
  - `MusicCategory` enum: Niem, Thien, Dao, KhongLoi, CaNhan — methods `from_str`, `db_value`, `display`, `icon`, `color`, `description`, `all_system()`.
  - `PlaybackMode` enum: SingleRepeat, Shuffle, RepeatAll, Loop — methods `from_str`, `db_value`, `display`, `icon`, `all()`. Default: RepeatAll.
  - `MusicTrack` struct (FromRow) — methods `category_enum()`, `duration_display()` (MM:SS / HH:MM:SS), `can_play()` (check audio_url non-empty), `cover_emoji()` (fallback emoji per category).
  - `UserMusicPrefs` struct (FromRow) — `playback_mode_enum()`, Default impl.
  - `MusicPrefsForm` — form payload cho POST preferences. `validate()` trả về tuple (mode, volume, sleep, last_track) đã sanitize — volume clamped 0–100, sleep_timer > 0 hoặc None.
  - `AddPersonalTrackForm` — form payload cho add track → Cá Nhân.
  - `PersonalPlaylistItem` struct (FromRow).
  - `NhaNhacStats` struct: total_tracks, tracks_by_category, personal_tracks, total_plays.

- **[MUSIC-3] `src/handlers/nha_nhac.rs`** — 9 handlers:
  - `nha_nhac_index` — GET /khong-gian/nha-nhac (default category: niem).
  - `nha_nhac_category` — GET /khong-gian/nha-nhac/{category} — render template với category đã chọn; invalid category → redirect về index.
  - `nha_nhac_tracks_api` — GET /api/nha-nhac/tracks — JSON tất cả track (auth required).
  - `nha_nhac_tracks_by_category_api` — GET /api/nha-nhac/tracks/{category} — JSON track theo category (ca_nhan → empty, frontend gọi API khác).
  - `nha_nhac_prefs_api` — GET /api/nha-nhac/preferences — JSON preferences của user (lazy-create default nếu chưa có).
  - `nha_nhac_prefs_update` — POST /api/nha-nhac/preferences — HTMX partial response (success message với mode label + volume + sleep timer status).
  - `nha_nhac_ca_nhan_add` — POST /api/nha-nhac/ca-nhan/them — Add track → Cá Nhân (UNIQUE constraint → idempotent, trả "đã có" nếu duplicate).
  - `nha_nhac_ca_nhan_remove` — POST /api/nha-nhac/ca-nhan/xoa/{track_id} — Remove track khỏi Cá Nhân.
  - `nha_nhac_track_play` — POST /api/nha-nhac/track/{track_id}/play — Increment play_count (analytics).
  - `nha_nhac_stats_api` — GET /api/nha-nhac/stats — JSON stats cho dashboard.
  - Internal helpers: `render_nha_nhac()` (chia sẻ logic render cho index + category), `fetch_all_tracks()`, `fetch_tracks_by_category()`, `fetch_personal_tracks()` (JOIN user_personal_tracks + music_tracks), `fetch_prefs()` (lazy insert default nếu chưa có), `upsert_prefs()` (dynamic SET với COALESCE — chỉ update fields được cung cấp), `fetch_stats()`.

- **[MUSIC-4] `templates/khong-gian/nha-nhac.html`** — Player UI:
  - Hero banner gradient indigo→violet với total tracks stats.
  - Category tabs (5 pills): Niem · Thien · Dao · KhongLoi · CaNhan — active state với gradient.
  - Main grid 2 cột (lg): track list (left, 2/3) + player card (right, 1/3).
  - Track list: cover emoji (large), title, artist, duration, play_count, ⭐ Add to Cá Nhân button (hoặc 🗑️ Remove khi ở tab CaNhan).
  - Player card (gradient indigo-900): now playing display (cover emoji 96×96, title, artist), HTML5 `<audio>` element, controls (⏮ ▶/⏸ ⏭), playback mode selector (4 buttons), volume slider (range input 0–100), sleep timer (15/30/60/Tắt) với amber pulse animation khi active, "empty audio URL" notice cho track chưa có file.
  - "Về Nhà Nhạc" info card với stats (total tracks · personal count).
  - Alpine.js component `musicPlayer({tracks, currentTrackId, playbackMode, volume, sleepTimerMinutes})`:
    - `playTrack(id)` — set current, play audio, increment play count via fetch.
    - `togglePlay()` — play/pause.
    - `onEnded()` — apply playback mode (single_repeat → replay, else nextTrack).
    - `nextTrack()` / `prevTrack()` — navigation theo playback mode (shuffle → random).
    - `setPlaybackMode(mode)` + `setSleepTimer(minutes)` — update + savePrefs().
    - `savePrefs()` — debounced (600ms) POST /api/nha-nhac/preferences với URLSearchParams body.
    - Sleep timer: setTimeout (minutes × 60000) → pause audio + clear state.
  - Auth gate: không đăng nhập → show "Vui lòng đăng nhập" + Google OAuth button.

- **[MUSIC-5] `src/main.rs`** — 10 routes mới (xem README cho danh sách đầy đủ). Lưu ý: `GET|POST /api/nha-nhac/preferences` chain trong 1 route() call (axum 0.8 không cho phép 2 route() trùng path).

- **[MUSIC-6] `templates/khong-gian/index.html`** — Thêm card "Nhà Nhạc" (gradient indigo→violet) ngay dưới hero Không Gian, với badge "MỚI · v0.9.33" và CTA "🎵 Mở Nhà Nhạc →" link tới /khong-gian/nha-nhac.

- **[MUSIC-7] `src/static/css/app.css`** — CSS cho Nhà Nhạc:
  - `.nha-nhac-player` — gradient indigo-900 → indigo-700, color #e0e7ff.
  - `.nha-nhac-btn` — transition + hover (translateY -1px + shadow) + active (scale 0.96).
  - `.nha-nhac-btn-active` — gradient indigo-500 → indigo-700, white text, shadow.
  - `.nha-nhac-track` — hover (bg indigo/12% + translateX 2px).
  - `.nha-nhac-track-playing` — bg indigo/18% + border-left 3px indigo.
  - `.nha-nhac-playing-indicator` — pulse animation 1.2s.
  - `.nha-nhac-equalizer-bar` — animated equalizer bars (4 bars, staggered delay 0/0.15/0.30/0.45s).

### 🪷 Logo Emoji Sharpened (giữ nguyên emoji 🪷)

- **[LOGO-1] `src/static/favicon.svg`** — Bump viewBox 100→256, font-size 90→240. Thêm `shape-rendering="geometricPrecision"` trên `<svg>`. Thêm `text-anchor="middle"` + `dominant-baseline="central"` để center chính xác. Thêm `text-rendering="geometricPrecision"` trên `<text>`. Thêm `font-family` fallback: `"Apple Color Emoji", "Segoe UI Emoji", "Noto Color Emoji", "Twemoji Mozilla", "EmojiOne Color", system-ui, sans-serif`.

- **[LOGO-2] `src/static/logo.svg`** — Tương tự favicon (256×256, geometricPrecision, font-family fallback). Dùng cho home hero + og:image.

- **[LOGO-3] `src/static/logo-inline.svg`** — Tương tự (256×256, geometricPrecision, font-family fallback). Dùng cho header navbar.

- **[LOGO-4] `templates/layout.html`** — Favicon data URI:
  - Bump viewBox 100→256, font-size 90→240.
  - Thêm `shape-rendering='geometricPrecision'` trên `<svg>`.
  - Thêm `text-rendering='geometricPrecision'` + `text-anchor='middle'` + `dominant-baseline='central'` trên `<text>`.
  - Thêm `font-family='Apple Color Emoji, Segoe UI Emoji, Noto Color Emoji, Twemoji Mozilla, system-ui, sans-serif'`.
  - URL-encode `<`→`%3C`, `>`→`%3E` cho data URI SVG hợp lệ (trước đây dùng raw `<` `>` hoạt động trên Chrome nhưng fail trên một số browser cũ).
  - Thêm `<link rel="alternate icon" href="/static/favicon.svg">` fallback cho trình duyệt cũ không hỗ trợ SVG data URI.
  - Thêm `type="image/svg+xml"` cho `<link rel="icon">` để browser biết content type.

- **[LOGO-5] `src/static/css/app.css`** — Thêm CSS class mới:
  - `.lotus-emoji` — class helper cho mọi chỗ dùng emoji 🪷 as logo/hero. `font-family` emoji fallback, `text-rendering: geometricPrecision`, `-webkit-font-smoothing: antialiased`, `-moz-osx-font-smoothing: grayscale`, `font-feature-settings: "liga" 1, "calt" 1`, `font-variant-emoji: emoji`, `transform: translateZ(0)` (GPU acceleration), `backface-visibility: hidden`.
  - `.lotus-logo-header` — variant cho header navbar logo (36px displayed). Tương tự `.lotus-emoji` + `line-height: 1`.
  - Override `.niem-btn` + `.buddha-statue` — thêm `font-family` emoji fallback để emoji render as color glyph (không phải text monochrome).

- **[LOGO-6] Layout.html + home.html** — Apply class:
  - `templates/layout.html` header logo: `<span class="text-2xl leading-none">🪷</span>` → `<span class="lotus-logo-header text-2xl">🪷</span>`.
  - `templates/layout.html` bottom nav center button: tương tự.
  - `templates/layout.html` chat bubble: wrap `🪷` trong `<span class="lotus-emoji">🪷</span>`.
  - `templates/home.html` hero: `<span class="text-5xl leading-none">🪷</span>` → `<span class="lotus-emoji text-5xl leading-none">🪷</span>`.

### 📦 Version Sync v0.9.33

- **[VER-1] `Cargo.toml`** — `version = "0.9.33"`.
- **[VER-2] `src/main.rs`** — Startup log v0.9.33 + phase 38 + health check version/phase (public + inner).
- **[VER-3] `templates/layout.html`** — Footer: v0.9.33.
- **[VER-4] `templates/khong-gian/index.html`** — Footer: v0.9.33.
- **[VER-5] `Dockerfile.coolify`** — Comment: v0.9.33 — Giai đoạn 38.
- **[VER-6] `templates/admin/phat-trien/index.html`** — Phase badge: GIAI ĐOẠN 38. Roadmap: 37 → "Hoàn thành" (green), 38 → "Đang triển khai" (indigo, "Nhà Nhạc (Music House)"). Footer: v0.9.33.
- **[VER-7] 6 admin templates** — Footer version sync: `templates/admin/{ky-thuat,quan-li,cong-dong,cong-dong/cam-ngo,users,placeholder}.html` — display version v0.9.32 → v0.9.33.

### 📋 Health Check Updates

- `HEALTH_FEATURES` thêm 9 features mới:
  - `nha-nhac-music-house-v0.9.33`
  - `music-player-5-categories-v0.9.33`
  - `music-playback-modes-4-v0.9.33`
  - `music-sleep-timer-v0.9.33`
  - `music-personal-playlist-v0.9.33`
  - `music-preferences-persisted-v0.9.33`
  - `music-stats-play-count-v0.9.33`
  - `logo-emoji-sharpened-geometric-precision-v0.9.33`
  - `favicon-svg-256-viewbox-v0.9.33`
  - `emoji-font-family-fallback-v0.9.33`
- `khong_gian.features` thêm `nha-nhac-music-house`.
- `khong_gian.nha_nhac` object mới (status, route, categories, playback_modes, sleep_timer, personal_playlist).
- `v0_9_33_note` — Mô tả thay đổi giai đoạn 38.
- Phase 37 → 38 trong health check.

### 🛠️ Yêu cầu kỹ thuật

- **Rust 1.97.1** — đã pin trong `Cargo.toml` (`rust-version = "1.97.1"`) và `Dockerfile` (`FROM rust:1.97.1-slim-bookworm`).
- **PostgreSQL 17** — migration 023 sử dụng `BIGSERIAL`, `TIMESTAMPTZ`, `UUID`, `CHECK` constraint, `ON CONFLICT` (PostgreSQL 9.5+).
- **axum 0.8** — route chain `get().post()` cho `/api/nha-nhac/preferences`.
- **askama 0.14** — template `khong-gian/nha-nhac.html` extends `layout.html`.

---

## [0.9.32] — 2026-08-16 — Giai đoạn 37: Admin Phát Triển Dashboard + Logo Emoji 🪷 + Version Sync

### 🎯 Mục tiêu giai đoạn

Tạo **dashboard riêng** cho Admin Phát Triển (`/admin/phat-trien`) — thay thế việc tạm dùng `/admin/ky-thuat`. Đổi hoàn toàn logo sang **emoji hoa sen 🪷** (thay SVG lotus phức tạp). Đồng bộ version v0.9.32 trên tất cả file — fix version drift từ v0.9.19/v0.9.29/v0.9.30.

### 🧭 Admin Phát Triển Dashboard (MAJOR)

- **[DASH-1] `templates/admin/phat-trien/index.html`** — Tạo dashboard riêng cho admin_phat_trien:
  - Phong cách: **indigo/vision/roadmap** — màu chủ đạo `#312E81` (indigo-900)
  - 4 stats card: Thành viên · Nhóm · Sách · Bình luận
  - **Roadmap Phát Triển**: Giai đoạn 37 (đang triển khai) · 38 (Nhà Nhạc) · 39 (Thương Thành) · 40 (Game Siêu Độ)
  - **CI/CD & Triển khai**: GitHub Actions · Coolify · Docker Image · Domain · Migrations
  - 3 Quick actions: Nhật ký · Thành viên · Kinh Sách
  - Permission summary: 39/150 quyền (system/security/analytics/navigation/api + media)
  - **Định hướng Phát triển**: Nguyên tắc cốt lõi · 3 giai đoạn (I/II/III)
  - Tab navigation: Tổng quan · Nhật ký · Thành viên · Kinh Sách · Quỹ
  - Phase banner: Giai đoạn 37 · v0.9.32

- **[DASH-2] `src/handlers/admin.rs`** — Thêm handler `admin_phat_trien_dashboard`:
  - Permission check: chỉ `admin_phat_trien` vào được
  - Template struct `AdminPhatTrienTemplate` với Askama
  - Cập nhật module doc: "4 giao diện admin riêng biệt" (thêm /admin/phat-trien)

- **[DASH-3] `src/main.rs`** — Thêm route `GET /admin/phat-trien`

- **[DASH-4] `src/models/user.rs`** — Cập nhật `admin_dashboard_path()`:
  - `admin_phat_trien` giờ trả `/admin/phat-trien` (trước đây trả `/admin/ky-thuat`)
  - Xóa TODO comment "Dashboard riêng sẽ thêm ở giai đoạn sau"

- **[DASH-5] `src/main.rs` health check** — `admin_phat_trien_dashboard` giờ là `/admin/phat-trien`

### 🪷 Logo Emoji Hoa Sen (MAJOR)

- **[LOGO-1] `templates/layout.html`** — Favicon đổi từ SVG sang inline emoji SVG data URI:
  - `<link rel="icon" href="data:image/svg+xml,...🪷...">`
  - Header logo đổi từ `<img src="/static/logo-inline.svg">` sang `<span>🪷</span>`
  - Footer version cập nhật v0.9.32

- **[LOGO-2] `templates/home.html`** — Hero logo đổi từ `<img src="/static/logo.svg">` sang `<span>🪷</span>`

- **[LOGO-3] `src/static/favicon.svg`** — Thay bằng emoji-based SVG: `<text>🪷</text>`

- **[LOGO-4] `src/static/logo.svg`** — Thay bằng emoji-based SVG: `<text>🪷</text>`

- **[LOGO-5] `src/static/logo-inline.svg`** — Thay bằng emoji-based SVG: `<text>🪷</text>`

- **[LOGO-6] Tất cả admin templates** — Favicon đồng nhất sang emoji 🪷 (6 file: cong-dong/index, cong-dong/cam-ngo, quan-li/index, placeholder, phat-trien/index, ky-thuat/index)

### 📦 Version Sync v0.9.32

- **[VER-1] `Cargo.toml`** — `version = "0.9.32"`
- **[VER-2] `src/main.rs`** — Startup log v0.9.32 + phase 37 + health check version/phase
- **[VER-3] `templates/layout.html`** — Footer: v0.9.32
- **[VER-4] `Dockerfile.coolify`** — Comment: v0.9.32 — Giai đoạn 37
- **[VER-5] Tất cả admin templates** — Fix version drift: v0.9.19/v0.9.29/v0.9.30 → v0.9.32 (31 replacements across 9 files)
- **[VER-6] `templates/khong-gian/index.html`** — Footer: v0.9.32

### 📋 Health Check Updates

- `HEALTH_FEATURES` thêm 6 features mới:
  - `admin-phat-trien-dashboard-v0.9.32`
  - `admin-phat-trien-indigo-vision-theme-v0.9.32`
  - `admin-4-dashboards-separate-v0.9.32`
  - `logo-emoji-lotus-v0.9.32`
  - `favicon-emoji-lotus-v0.9.32`
  - `version-sync-v0.9.32`
- `roles.admin_phat_trien_dashboard` → `/admin/phat-trien`
- `v0_9_32_note` — Mô tả thay đổi giai đoạn 37

---

## [0.9.30] — 2026-08-16 — Giai đoạn 35: Admin Phát Triển Role + DM REST Fallback + Bug Fix Sweep

### 🎯 Mục tiêu giai đoạn

Thêm chính thức chức vụ **"Admin Phát Triển"** (`admin_phat_trien`) vào hệ thống — vai trò mà v0.9.29 đã phải tạm đổi sang "Admin Cộng Đồng" vì role chưa tồn tại. Cập nhật thông tin Võ Đăng Trọng Nghĩa (đổi sang Admin Phát Triển, không tôn giáo). Fix triệt để lỗi "không thể gửi tin nhắn cho bạn bè" bằng REST fallback endpoint. Quét và fix các lỗi logic/UI còn sót.

### 🧭 Thêm role `admin_phat_trien` — Admin Phát Triển (MAJOR)

- **[ROLE-1] Migration 022** — Thêm role `admin_phat_trien` vào hệ thống:
  - Drop old CHECK constraint (5 giá trị) → add new CHECK constraint cho phép 6 giá trị (thêm `admin_phat_trien`).
  - Seed `role_permissions` cho `admin_phat_trien` — 39 quyền: system (10) + users (7) + security (5) + media (5) + analytics (6) + navigation (5) + api (1).
  - Scope: định hướng phát triển sản phẩm, CI/CD, roadmap, kỹ thuật xây dựng. Giao thoa với admin_ky_thuat nhưng tập trung vào "phát triển sản phẩm" thay vì "vận hành hệ thống".
  - Update view `v_user_permissions` + comment trên `role_permissions`.
  - File: `migrations/022_admin_phat_trien_role.sql`

- **[ROLE-2] `db/mod.rs` — Cập nhật `ensure_schema_safety`** — CHECK constraint trong safety check cũng được cập nhật để cho phép `admin_phat_trien` (idempotent, chạy trên fresh deploy).

- **[ROLE-3] `src/models/user.rs` — Full support admin_phat_trien**:
  - `role_display()` → "Admin Phát Triển"
  - `role_icon()` → 🧭 (la bàn — định hướng phát triển)
  - `role_color()` → `#312E81` (indigo-900 — phát triển/sáng tạo)
  - `role_level()` → 3 (NGANG HÀNH với 3 admin kia)
  - `is_admin()` → include `admin_phat_trien` (4 admin ngang hàng)
  - `is_admin_phat_trien()` — method mới, true chỉ cho role này
  - `has_permission_code()` — arm mới với 39 quyền theo scope
  - `permission_count()` → 39
  - `admin_dashboard_path()` → `/admin/ky-thuat` (scope giao thoa, dashboard riêng sẽ thêm giai đoạn sau)

- **[ROLE-4] `src/handlers/admin.rs` — Validate + badge + color**:
  - `admin_change_role` chấp nhận `admin_phat_trien` trong danh sách role hợp lệ.
  - `role_badge_html` hiển thị 🧭 Admin Phát Triển.
  - `role_color_hint` trả `#312E81` (indigo-900).
  - Comment header cập nhật — phản ánh 4 admin ngang hàng.

- **[ROLE-5] `templates/admin/users.html` — Thêm option role dropdown**:
  - Thêm form "🧭 Admin Phát Triển" với indigo styling, sau "⚙️ Admin Kỹ Thuật".
  - Active state: `bg-indigo-50 text-indigo-700 font-semibold`.

### 👥 Cập nhật trang Đội Ngũ Quản Lí — Võ Đăng Trọng Nghĩa

- **[TEAM-1] `src/handlers/doi_ngu.rs`** — Cập nhật `TEAM_MEMBERS`:
  - Võ Đăng Trọng Nghĩa: `role_title` đổi từ "Admin Cộng Đồng" → "Admin Phát Triển".
  - `religion` đổi từ "Phật giáo" → "Không" (theo yêu cầu cập nhật thông tin).
  - `role_detail` đổi thành "Định hướng phát triển sản phẩm, roadmap và kỹ thuật xây dựng".
  - `icon` đổi từ 🛡️ → 🧭. `accent_color` đổi từ `#1565C0` (blue) → `#312E81` (indigo).

- **[TEAM-2] `templates/doi-ngu-quan-li/index.html`** — Cập nhật card + section "Hệ Thống Vai Trò":
  - Card Võ Đăng Trọng Nghĩa: gradient indigo, icon 🧭, badge "Admin Phát Triển", tôn giáo "Không".
  - Section "Hệ Thống Vai Trò Quản Lí": grid 4 cột (lg:grid-cols-4) hiển thị 4 admin ngang hàng (Kỹ Thuật · Quản Lí · Cộng Đồng · Phát Triển) + Mod + Thành Viên.
  - Cập nhật ghi chú nguyên tắc: "4 admin đều ngang hàng ở cấp 3".

### 🔧 Fix lỗi không thể gửi tin nhắn cho bạn bè (CRITICAL — DM REST Fallback)

- **[DM-1] Thêm REST fallback endpoint** `POST /api/ban-be/tin-nhan/{conversation_id}/gui`:
  - **Nguyên nhân gốc rễ (còn sót từ v0.9.29)**: v0.9.29 đã fix nút Gửi bị disable khi WS chưa connect, nhưng nếu WS fail vĩnh viễn (mạng chập, proxy timeout, exhausted 10 reconnect attempts), tin nhắn vẫn bị kẹt trong queue `_queue` không bao giờ gửi → user vẫn "không gửi được tin nhắn".
  - **Fix v0.9.30**: Thêm endpoint REST dự phòng. Frontend `dmChat.send()` thử WS trước (fast path), nếu WS không OPEN thì fallback sang HTTP POST (reliable path). Server lưu message vào DB + broadcast qua DmChatHub → user khác online vẫn nhận realtime; user gửi nhận lại message qua HTTP response.
  - Handler `dm_send_message` in `src/handlers/friends.rs`: auth bắt buộc, verify participant, validate body (không rỗng, tối đa 1000 ký tự), save DB, broadcast hub, trả JSON message.
  - Route đăng ký trong `src/main.rs`.

- **[DM-2] `src/static/js/chat.js` — Cập nhật `dmChat.send()`**:
  - Nếu WS connected + OPEN → gửi qua WS (fast path, realtime).
  - Nếu WS không OPEN → gọi `_sendViaRest(body)` (HTTP POST fallback).
  - `_sendViaRest`: optimistic clear draft, fetch POST, thêm message vào danh sách nếu chưa duplicate, xử lý error (401/403/400/network), restore draft nếu fail.
  - Network error → reset `reconnectAttempts = 0` + `scheduleReconnect()` (cho phép reconnect mới sau khi đã exhausted).

- **[DM-3] Thêm role `admin_phat_trien` vào chat helpers**:
  - `msgBubbleClass`, `msgNameClass`, `avatarClass` — thêm class `chat-msg-admin-phat-trien` / `chat-avatar-admin-phat-trien`.
  - `roleBadgeHtml` — thêm badge "🧭 DEV" cho admin_phat_trien.

### 📦 Version Sync & Health Check

- Bump version `0.9.29` → `0.9.30` ở: `Cargo.toml`, `src/main.rs` (log + health check public + health check inner + phase 34 → 35), `templates/layout.html` (footer), `src/handlers/mod.rs` (placeholder footer — fix version drift v0.9.28 → v0.9.30).
- Thêm 10 feature flags v0.9.30 vào `HEALTH_FEATURES` array.
- Cập nhật `roles` object trong health check: hierarchy thêm `admin_phat_trien`, permission_counts thêm 39, admin_panel_access thêm role, thêm `admin_phat_trien_dashboard` + `v0_9_30_note`.

### 🐛 Bug fixes (UI/Logic sweep)

- **[BUG-1] Version drift trong placeholder footer** — `handlers/mod.rs:624` hiển thị "v0.9.28" (sai từ v0.9.29 chưa fix). Fix: đồng bộ v0.9.30.
- **[BUG-2] admin_phat_trien không có trong chat role badges** — Fix: thêm role badge 🧭 DEV + CSS classes cho admin_phat_trien trong chat.js.
- **[BUG-3] admin_phat_trien không có trong admin users dropdown** — Fix: thêm option 🧭 Admin Phát Triển trong role dropdown (templates/admin/users.html).

---

## [0.9.29] — 2026-08-16 — Giai đoạn 34: Admin Equal Rebalance + Live Chat Optimize + DM Fix + Performance

### 🎯 Mục tiêu giai đoạn

Đồng bộ hóa toàn bộ hệ thống admin với nguyên tắc "mọi admin đều bằng nhau ngang hàng", tối ưu hiệu năng chat và fix lỗi gửi tin nhắn cho bạn bè. Đây là giai đoạn chuyển tiếp từ Alpha (Giai đoạn I — 6 tháng) chuẩn bị bước sang Giai đoạn II — 100 ngày phát triển hệ sinh thái.

### 👥 Sửa hệ thống admin — Tất cả admin đều bằng nhau, ngang hàng, không phân cấp

- **[ADMIN-1] Đồng bộ code với migration 021** — Migration 021 từ v0.9.24 đã redesign admin ngang hàng (3 admin cùng cấp 3, scope quyền riêng), nhưng code và template vẫn còn nhiều chỗ lệch:
  - Trước v0.9.29: trang `/admin/quan-li` hiển thị "100/150 quyền", `/admin/cong-dong` hiển thị "75/150 quyền" (sai — số cũ từ v0.9.19 khi admin_ky_thuat có 150/150).
  - Trước v0.9.29: trang `/admin/users` ghi "KT (150/150) > QL (100/100) > CD (75/75) > Mod (15) > TV (0)" (sai — phân cấp cũ).
  - Trước v0.9.29: trang `/admin/ky-thuat` ghi "Admin Kỹ Thuật có toàn bộ 150 quyền (cao nhất), Admin Quản Lý 100, Admin Cộng Đồng 75" (sai).

  **Fix v0.9.29**:
  - Cập nhật comment header trong `src/handlers/admin.rs` — phản ánh đúng nguyên tắc ngang hàng.
  - Cập nhật `templates/admin/quan-li/index.html`: "100/150 quyền" → "40/150 quyền (admin ngang hàng)".
  - Cập nhật `templates/admin/cong-dong/index.html`: "75/150 quyền (UI/hệ thống)" → "45/150 quyền (admin ngang hàng)".
  - Cập nhật `templates/admin/ky-thuat/index.html`: "Admin Kỹ Thuật có toàn bộ 150 quyền (cao nhất)" → "3 admin NGANG HÀNH (cấp 3) — Kỹ Thuật 41 quyền · Quản Lí 40 quyền · Cộng Đồng 45 quyền".
  - Cập nhật `templates/admin/users.html`: note footer "v0.9.19: KT > QL > CD > Mod > TV" → "v0.9.29: 3 admin NGANG HÀNH (cấp 3) — KT (41) · QL (40) · CD (45) — không phân cấp cao/thấp".

- **[ADMIN-2] Sửa trang "Đội Ngũ Quản Lí"** — Trang `/doi-ngu-quan-li` hiển thị sai:
  - **Võ Đăng Trọng Nghĩa** được dán nhãn "Admin Phát Triển" — nhưng role `admin_phat_trien` KHÔNG TỒN TẠI trong code (chỉ có `member`, `mod`, `admin_ky_thuat`, `admin_cong_dong`, `admin_quan_li`).
  - Section "Hệ Thống Phân Cấp Quản Lí" hiển thị 5 cấp (5/4/3/2/1) — sai với code thực tế (3 admin ngang hàng cấp 3 + Mod cấp 2 + Member cấp 1).
  - Hiển thị "Admin Kỹ Thuật 150/150 quyền" — sai (code trả về 41 quyền).

  **Fix v0.9.29**:
  - Đổi vai trò Võ Đăng Trọng Nghĩa từ "Admin Phát Triển" → "Admin Cộng Đồng" (vai trò phù hợp với phụ trách: định hướng nội dung, cộng đồng, truyền thông, sự kiện).
  - Đổi icon từ 🧭 (violet) → 🛡️ (blue) — đồng bộ với CSS role_color của `admin_cong_dong`.
  - Đổi section "Hệ Thống Phân Cấp Quản Lí" → "Hệ Thống Vai Trò Quản Lí" — hiển thị 3 admin ngang hàng (cấp 3) + Mod (cấp 2) + Thành Viên (cấp 1), kèm số quyền đúng (41/40/45/15/0).
  - Thêm ghi chú nguyên tắc: "Tất cả admin đều bằng nhau ngang hàng, cùng cấp, không ai hơn ai."
  - Cập nhật `src/handlers/doi_ngu.rs` — đổi `role_title` của Võ Đăng Trọng Nghĩa trong `TEAM_MEMBERS` const.

### 💬 Live Chat Chung — Kéo dài + xóa che mờ + xóa hiệu ứng

- **[CHAT-1] Kéo dài màn hình Live Chat Chung** — User yêu cầu chat popup "dài hơn":
  - Desktop: `height: 65dvh` → `85dvh`, `max-height: 580px` → `880px`, `min-height: 360px` → `420px`, `width: 380px` → `400px`.
  - Mobile: `height: 45dvh` → `78dvh`, `max-height: 50dvh` → `82dvh`, `min-height: 240px` → `360px`.
  - Áp dụng cho cả `src/static/css/app.css` (selector `.chat-chung-popup`) và `src/static/css/chat.css` (override).

- **[CHAT-2] Xóa backdrop che mờ khi mở Live Chat Chung** — User yêu cầu "xóa che mờ khi mở live chat chung":
  - Xóa `<div class="chat-chung-backdrop md:hidden">` khỏi `templates/layout.html`.
  - Vô hiệu hóa CSS `.chat-chung-backdrop` (`display: none !important; background: transparent !important;`).
  - Xóa `body.chat-popup-open` lock scroll (CSS override `overflow: auto; position: static`).
  - Xóa `document.body.classList.add('chat-popup-open')` trong `toggleChat()` của `src/static/js/chat.js`.
  - Chat popup giờ mở trong suốt — user vẫn scroll trang được khi đang chat.

- **[CHAT-3] Xóa hiệu ứng tin nhắn admin/mod** — User yêu cầu "xóa hiệu ứng nhắn tin của các admin hay mod để tránh lag":
  - Vô hiệu hóa hoàn toàn CSS hiệu ứng trong `src/static/css/app.css`:
    - `chat-msg-admin-ky-thuat` (Matrix Terminal: scanline, glow, monospace) → bubble thường.
    - `chat-msg-admin-quan-li` (Premium Gold Frame: gradient, glow, 👑) → bubble thường.
    - `chat-msg-admin-cong-dong` (Shield Blue Frame: gradient, glow, 🛡️) → bubble thường.
    - `chat-msg-mod` (Teal Frame: gradient, glow, 📜) → bubble thường.
    - Xóa `::before` pseudo-element (đã từng là icon 👑/🛡️/📜).
    - Xóa animation `scanline`, `chat-msg-glow-green`, `chat-avatar-pulse-green`.
    - Avatar admin/mod: viền thường, không glow, không pulse.
  - Xóa prefix `[SYS]` trước tên admin_ky_thuat trong `authorLabel()` của `src/static/js/chat.js`.
  - Chỉ giữ role badge mini cạnh tên (để user vẫn biết đây là admin/mod) — không animation, không glow.

### 🔧 Fix lỗi không thể gửi tin nhắn cho bạn bè

- **[DM-1] Cho phép gửi tin nhắn ngay cả khi WebSocket chưa kết nối** — User report "không thể gửi tin nhắn cho bạn bè":
  - **Nguyên nhân gốc rễ**: Nút Gửi trong `templates/ban-be/conversation.html` có `:disabled="!connected || !draft.trim()"` → nếu WS chưa connect (vd. mạng chập, server restart), nút bị disable → user không thể gửi.
  - Cũng áp dụng cho chat chung trong `templates/layout.html` (`:disabled="!connected || !draft.trim()"`).

  **Fix v0.9.29**:
  - Đổi `:disabled="!connected || !draft.trim()"` → `:disabled="!draft.trim()"` — chỉ disable khi draft rỗng.
  - Cập nhật hàm `send()` trong cả `globalChat()` và `dmChat()` của `src/static/js/chat.js`:
    - Nếu WS chưa open, push tin nhắn vào `_queue`, clear draft (optimistic UX), auto-reconnect ngay lập tức.
    - Khi WS mở lại (onopen), flush queue tự động (đã có từ v0.9.20).
    - Nếu socket đang ở trạng thái CLOSING/CLOSED, chủ động `close(1000)` rồi `scheduleReconnect()`.
  - Thêm indicator "đang kết nối lại..." trong conversation.html khi `!connected`.

- **[DM-2] Tăng tốc auto-reconnect WebSocket**:
  - Trước v0.9.29: delay = `min(1000 * 2^(attempts-1), 30000)` → attempt 1 = 1s, attempt 5 = 16s, max 30s.
  - v0.9.29: delay = `min(500 * 1.8^(attempts-1), 8000)` → attempt 1 = 500ms (gần như ngay lập tức), attempt 5 = 2.9s, max 8s.
  - User experience: khi WS rớt, chỉ cần nửa giây để thử lại, tối đa 8s giữa các lần thử.

### ⚡ Tăng độ mượt và tốc độ load web

- **[PERF-1] Xóa CSS animation thừa gây lag**:
  - Xóa `msg-slide-in` animation (0.25s ease-out) cho tất cả chat message containers — thay bằng `animation: none`.
  - Xóa `send-btn-pulse` animation — thay bằng `transform: scale(0.96)` đơn giản.
  - Xóa `conn-pulse` animation cho connection indicator — `animation: none`.
  - Lý do: với 50+ tin nhắn trong chat, mỗi tin nhắn đều chạy animation riêng → CPU/GPU overload trên thiết bị yếu → lag/jank khi scroll.

- **[PERF-2] Giảm polling frequency**:
  - Notification badge poll: 30s → 60s (`src/static/js/chat.js`).
  - Session heartbeat: 5 phút → 10 phút (`src/static/js/app.js`).
  - Lý do: giảm số request không cần thiết tới server, tiết kiệm bandwidth và CPU.

- **[PERF-3] Tăng transition speed cho chat popup**:
  - Enter transition: 200ms → 150ms.
  - Leave transition: 150ms → 100ms.
  - Cảm giác mở/đóng popup nhanh và responsive hơn.

### 📚 Đồng bộ version & tài liệu

- **[DOC-1] Cập nhật version lên v0.9.29**:
  - `Cargo.toml`: `version = "0.9.29"`.
  - `src/main.rs`: log startup, `HEALTH_FEATURES`, `health_check_secure`, `health_check_inner` đều cập nhật `0.9.29` + Giai đoạn 34.
  - `templates/layout.html`: footer version.
  - Thêm 10 feature flags v0.9.29 vào `HEALTH_FEATURES` array.
  - Cập nhật `phase_name` trong health_check_inner: "Giai đoạn 34 — Admin Equal Rebalance + Live Chat Optimize + DM Fix + Performance".

- **[DOC-2] Cập nhật comment trong source code**:
  - `src/handlers/admin.rs`: header comment phản ánh đúng phân quyền ngang hàng v0.9.29.
  - `src/handlers/doi_ngu.rs`: comment giải thích lý do đổi vai trò Võ Đăng Trọng Nghĩa.
  - `src/static/css/app.css`: comment chi tiết về lý do xóa hiệu ứng admin/mod.
  - `src/static/css/chat.css`: comment về lý do xóa animation message slide-in.
  - `src/static/js/chat.js`: comment về fix lỗi gửi tin nhắn khi WS disconnected.

---

## [0.9.28] — 2026-08-16 — Giai đoạn 33: CSP Fix (Alpine.js) + XSS Hardening + Memory Leak Fix

### 🚨 Sửa lỗi CRITICAL — CSP thiếu `'unsafe-eval'` làm Alpine.js hoàn toàn không hoạt động

- **[CSP-1] Alpine.js fail silently trên production** — Nguyên nhân gốc rễ của TẤT CẢ lỗi UI mà user report ở v0.9.27:
  - **Hamburger menu (3 gạch) bị liệt** — click không có tác dụng
  - **Cả 2 icon (☰ hamburger + ✕ close) cùng hiện** — x-show directive không evaluate được
  - **Chat bubble biến mất** (khi logged in) — x-data="globalChat()" không init được, x-cloak vẫn còn → CSS `[x-cloak] { display: none !important; }` ẩn element
  - **Mega menu desktop không mở** được
  - **Theme toggle không hoạt động**
  - **Notification badge không update**
  - **Mọi tính năng dựa trên Alpine.js đều hỏng**

  **Nguyên nhân**: CSP header `script-src 'self' 'unsafe-inline' https://cdn.tailwindcss.com https://unpkg.com https://www.googletagmanager.com` thiếu `'unsafe-eval'`. Alpine.js 3.x dùng `new Function()` để evaluate các expression trong `x-data`, `x-show`, `x-text`, `@click`, v.v. Browser block eval theo CSP → Alpine throw warning nhưng fail silently.

  **Fix**: Thêm `'unsafe-eval'` vào `script-src` trong `src/middleware/headers.rs`. Trong tương lai có thể migrate sang Alpine CSP build (`alpine.csp.js`) + `Alpine.data()` registrations để bỏ `'unsafe-eval'`, nhưng đó là refactor lớn chạm toàn bộ templates.

  **Xác minh**: Test trên production bằng headless browser — trước fix, browser console có 50+ warnings "Alpine Expression Error: Evaluating a string as JavaScript violates the following Content Security Policy directive because 'unsafe-eval' is not an allowed source of script". Sau fix, không còn warning nào, Alpine.js hoạt động bình thường.

### 🔒 Sửa lỗi HIGH — Reflected XSS trong OAuth error page

- **[XSS-1] `auth.rs::error_page` không escape HTML** — Hàm `error_page(title, msg)` dùng `format!()` để inject `title` và `msg` trực tiếp vào HTML. Tại `google_callback`, `&query.error` (từ URL query string, hoàn toàn user-controlled) được chèn vào `msg`:
  ```rust
  error_page("Chưa đăng nhập được bằng Google",
      &format!("Google báo lỗi: {err}. Vui lòng thử lại."));
  ```
  Attacker craft URL `https://tubi.../auth/google/callback?error=<script>fetch('/admin/thanh-vien/VICTIM_ID/ban',{method:'POST'})</script>` rồi lừa victim click → script execute trong session của victim. CSP có `'unsafe-inline'` nên script chạy được. Attacker có thể thực hiện actions thay victim (ban user, gửi mail, đổi avatar...).

  **Fix**: Thêm utility `html_escape()` trong `src/handlers/mod.rs`. Escape `title` và `msg` trước khi `format!()`.

### 🔒 Sửa lỗi HIGH — Stored XSS trong friends.rs HTMX responses

- **[XSS-2] `send_friend_request` và `accept_friend_request` không escape `display_name` + `avatar_url`** — Các handler trả về HTMX partial HTML xây bằng `format!()`:
  ```rust
  format!(r#"...<div class="font-semibold ...">{display_name}</div>..."#)
  format!(r#"<img src="{url}" alt="avatar" ...>"#)
  ```
  `display_name` là user-controlled qua `POST /ca-nhan/cap-nhat` (chỉ trim + check length ≤ 100, không sanitize HTML). User ác ý đặt `display_name = "<img src=x onerror=fetch('/admin/thanh-vien/VICTIM_ID/ban',{method:'POST'})>"` → khi victim xem danh sách bạn bè / gửi lời mời kết bạn / chấp nhận lời mời, script execute trong session của victim. Có thể dùng để tự động ban user khác (nếu victim là admin), gửi mail spam, v.v.

  **Fix**: HTML-escape `display_name` và `avatar_url` bằng `html_escape()` trước khi `format!()`.

### 🔧 Sửa lỗi HIGH — Memory leak trong DmChatHub

- **[LEAK-1] `DmChatHub::channels` HashMap grow unbounded** — `DmChatHub` giữ `Arc<Mutex<HashMap<Uuid, broadcast::Sender<BroadcastPayload>>>>`. Mỗi conversation_id mới tạo entry `or_insert_with(|| broadcast::channel(128))` nhưng **không bao giờ remove** entry khi conversation hết active receivers. Sau hàng ngàn conversation, HashMap leak RAM (mỗi entry giữ broadcast buffer 128 slots). Server phải restart mới giải phóng.

  **Fix**: Thêm method `cleanup_if_empty(conversation_id)` — gọi sau khi DM WebSocket disconnect. Nếu `sender.receiver_count() == 0` thì `map.remove(&conversation_id)`. So sánh: `GlobalChatHub` chỉ 1 channel nên không cần cleanup.

### 📦 Version Sync

- Bump version `0.9.27` → `0.9.28` ở: `Cargo.toml`, `src/main.rs` (log + health check response 2 nơi + comment), `templates/layout.html` (footer), `src/handlers/mod.rs` (placeholder_page footer — fix drift từ v0.9.26).
- Update phase 32 → 33 trong health check.
- Update `HEALTH_FEATURES` (+8 features v0.9.28).

### 📋 Ghi chú

- **Báo cáo lỗi của user**: User report "không thấy bong bóng live chat đâu, nút 3 gạch bị liệt, bên dưới có dấu x". Đây là các triệu chứng của cùng 1 root cause (CSP thiếu 'unsafe-eval' → Alpine.js fail). Fix 1 lỗi CSP → tất cả triệu chứng biến mất.
- **Security tradeoff**: `'unsafe-eval'` giảm security posture, nhưng đã có `'unsafe-inline'` từ trước nên impact thêm là nhỏ. Long-term plan: migrate sang Alpine CSP build.
- **Không có DB migration** trong release này — chỉ thay đổi code + CSP header.

---

## [0.9.27] — 2026-08-15 — Giai đoạn 32: Critical UI Fix (FOUC + Chat + Menu) + Chat History Robustness + Security

### Sửa lỗi (CRITICAL — FOUC / Flash of Unstyled Content)

- **[FOUC-1] Chat Chung popup flash visible trước khi Alpine.js init** — Nguyên nhân: `[x-cloak]` CSS specificity thấp hơn `.chat-chung-popup` (display:flex) → trên mobile, chat popup flash visible trước khi Alpine xử lý `x-show`. Fix: thêm `style="display:none"` trực tiếp vào HTML element + class-specific x-cloak selectors `[x-cloak].chat-chung-popup` (specificity 0,2,0 thắng 0,1,0). Alpine `x-show` override khi init xong. Nếu Alpine fail → popup vẫn ẩn.

- **[FOUC-2] Mobile hamburger menu (3 gạch) flash visible trước khi Alpine init** — Cùng nguyên nhân FOUC-1. Fix: thêm `style="display:none"` + class `mobile-menu-drawer` + class-specific x-cloak selector.

### Sửa lỗi (HIGH — Chat popup tự mở + không đóng được)

- **[CHAT-1] Chat popup tự mở khi Alpine component re-init** — `isOpen` có thể bị `undefined` → `!undefined === true` → popup tự mở. Fix: thêm guard `typeof isOpen !== 'boolean'` → reset false; thêm `this.isOpen = false` đầu tiên trong `init()`.

- **[CHAT-2] Nút đóng chat (×) quá nhỏ trên mobile** — w-8 h-8 (32px) dễ miss tap. Fix: tăng lên w-10 h-10 (40px) + border + active feedback.

- **[CHAT-3] Chat popup vẫn che nhiều trên điện thoại nhỏ** — 50dvh + min 280px che >50% màn hình 568px. Fix: giảm từ 50dvh → 45dvh, min 280 → 240, max 55dvh → 50dvh.

### Sửa lỗi (HIGH — Chat history bị mất)

- **[CHAT-4] loadHistory() fail silently → "mất lịch sử"** — Nếu API fail, messages = [] → user tưởng mất data. Fix: retry tối đa 2 lần với exponential backoff, log error bằng console.warn, validate response là Array.

### Sửa lỗi (MEDIUM — ILIKE wildcard injection)

- **[SEARCH-1] User search "%" match tất cả rows** — `format!("%{q}%")` không escape `%` và `_`. Fix: escape `\` → `\\`, `%` → `\%`, `_` → `\_` + thêm `ESCAPE '\\'` clause trong SQL. Áp dụng cho `tim_kiem.rs`, `kinh_sach.rs`, `friends.rs`.

### Sửa lỗi (LOW — Missing x-cloak)

- **[FOUC-3] Missing x-cloak trên DM chat + global chat** — 6 element thiếu `x-cloak` → flash visible. Fix: thêm `x-cloak` + `style="display:none"`.

---

## [0.9.26] — 2026-08-15 — Giai đoạn 31: UI Fix (Live Chat + Hamburger Menu) + Deploy Pipeline Fix

### Sửa lỗi (CRITICAL — Deploy pipeline)

- **[DEPLOY-1] Workflow GitHub Actions không tự update Dockerfile.coolify** — Trước v0.9.26, deploy pipeline dùng 2-commit pattern:
  1. Code commit X → workflow build image `sha-X` → trigger Coolify deploy
  2. Coolify clones repo → đọc Dockerfile.coolify từ commit X (vẫn ghi `FROM sha-PREVIOUS`) → pull `sha-PREVIOUS` → deploy OLD code
  3. Developer phải push commit X+1 manual để update Dockerfile.coolify → `FROM sha-X`
  4. Coolify deploy lại → lần này mới chạy code mới
  - **Tác động**: User báo "deploy thành công nhưng production vẫn y như cũ" — vì production chạy code cũ, chỉ khi nào developer nhớ push commit thứ 2 thì code mới thực sự lên.
  - **Fix v0.9.26**: Workflow GitHub Actions tự update Dockerfile.coolify với SHA tag mới NGAY SAU khi build image xong, trước khi trigger Coolify deploy. Commit message chứa `[skip ci]` để tránh workflow loop. Developer chỉ cần push 1 commit code, workflow tự lo phần còn lại.
  - **File**: `.github/workflows/docker.yml` — thêm job `update-coolify-dockerfile` chạy sau `build-and-push`, trước `trigger-coolify`.

### Sửa lỗi (HIGH — UI)

- **[UI-1] Chat Chung popup che 60% màn hình mobile, không có backdrop, không thể thao tác** — Trước v0.9.26, `.chat-chung-popup` trên mobile có `width: 100%, height: 60dvh, bottom: 72px`, không có backdrop overlay. Khi user mở chat, popup che gần hết màn hình, không có cách nào rõ ràng để đóng (nút × quá nhỏ, không tap outside được).
  - **Fix v0.9.26**:
    - Giảm popup height trên mobile từ `60dvh` → `50dvh` (còn không gian trên để tương tác với page)
    - Giảm min-height từ `340px` → `280px` (fit điện thoại nhỏ)
    - Thêm backdrop overlay semi-transparent (`.chat-chung-backdrop`) — tap outside để đóng popup
    - Lock body scroll khi popup mở (class `body.chat-popup-open`) — tránh scroll page bên dưới
    - ESC key đóng popup (`@keydown.escape.window`)
    - Nút × to hơn (text-lg → text-2xl, padding + hover bg) — dễ bấm hơn
  - **File**: `templates/layout.html`, `src/static/css/app.css`, `src/static/css/chat.css`, `src/static/js/chat.js`.

- **[UI-2] Mobile menu (3 gạch) bị bật vĩnh viễn, không tự đóng** — Trước v0.9.26, `<div x-show="mobileMenu">` không có `@click.outside` directive. Khi user tap nút 3 gạch, `mobileMenu = true` → menu mở. Nhưng menu chỉ đóng khi tap lại nút 3 gạch, tap vào 1 link, hoặc resize sang desktop. Không có cách nào đóng menu khi tap outside hoặc nhấn ESC.
  - **Fix v0.9.26**:
    - Thêm `@click.outside="mobileMenu = false"` → đóng menu khi tap ra ngoài
    - Thêm `@keydown.escape.window="mobileMenu = false"` → đóng menu khi nhấn ESC
    - Icon toggle: 3 gạch ⇄ X (đổi icon khi mở/đóng)
    - Tất cả link trong menu thêm `@click="mobileMenu = false"` → đóng menu khi click link
    - `aria-expanded` + `aria-label="Menu"` cho accessibility
  - **File**: `templates/layout.html`.

### Sửa lỗi (MEDIUM — UI)

- **[UI-3] Chat bubble trên mobile đè 32px lên bottom nav** — Trước v0.9.26, chat bubble có `top: y = innerHeight - 88` trên mobile. Bubble height = 56px → bottom edge ở `innerHeight - 32`. Bottom nav top ở `innerHeight - 64`. → Bubble bottom (innerHeight - 32) nằm BELOW nav top (innerHeight - 64) → đè lên nav 32px. User tap "🙏 Niệm Phật" (rightmost nav item) vô tình tap chat bubble.
  - **Fix v0.9.26**: Đổi bubble `y = innerHeight - 128` trên mobile → bubble bottom ở `innerHeight - 72` = ngay trên bottom nav top. Ẩn chat bubble khi chat popup đang mở (`x-show="!isOpen"`) → tránh bubble che popup input area.
  - **File**: `src/static/js/chat.js`, `templates/layout.html`.

### Thay đổi

- Cập nhật version v0.9.25 → v0.9.26, Giai đoạn 30 → 31
- `Cargo.toml`: bump version 0.9.25 → 0.9.26 (rust-version = "1.97.1" giữ nguyên)
- `src/main.rs`: cập nhật log info, health check response (version, phase, phase_name), `HEALTH_FEATURES` (+12 features v0.9.26)
- `templates/layout.html`: update footer version v0.9.25 → v0.9.26, thêm backdrop overlay + ESC handler cho chat popup, thêm `@click.outside` + `@keydown.escape.window` cho mobile menu, icon toggle 3 gạch ⇄ X, link click tự đóng menu, `aria-expanded` + `aria-label` cho accessibility, ẩn chat bubble khi popup mở
- `src/handlers/mod.rs`: update placeholder footer version v0.9.25 → v0.9.26
- `src/static/css/app.css`: thêm `.chat-chung-backdrop` class, `body.chat-popup-open` scroll lock, giảm `.chat-chung-popup` mobile height từ 55dvh → 50dvh
- `src/static/css/chat.css`: giảm `.chat-chung-popup` mobile height từ 60dvh → 50dvh, min-height từ 340px → 280px
- `src/static/js/chat.js`: `toggleChat()` thêm `document.body.classList` toggle cho scroll lock; `chatBubble.init()` đổi mobile y từ `innerHeight - 88` → `innerHeight - 128` để tránh đè bottom nav
- `.github/workflows/docker.yml`: thêm job `update-coolify-dockerfile` chạy sau `build-and-push`, trước `trigger-coolify`. Job này tự update Dockerfile.coolify với SHA tag mới, commit + push với `[skip ci]` để tránh workflow loop.
- `Dockerfile.coolify`: update comment giải thích cơ chế auto-update SHA tag v0.9.26
- `README.md`: thêm section v0.9.26 — Giai đoạn 31, đẩy v0.9.25 xuống "Phiên bản trước"

### Ghi chú vận hành

- **Database persistence**: Database `tubi-postgres` (PostgreSQL 17-alpine) trên Coolify có persistent volume. Migration 021 (TRUNCATE role_permissions) chỉ xoá bảng permissions (re-seed), KHÔNG chạm vào user data, chat messages, topics, comments.
- **Mất dữ liệu lịch sử**: Nếu user thấy "mất hết dữ liệu kể từ v0.9.25", nguyên nhân thực tế là DB container bị recreate ngày 2026-08-13 (không phải do code v0.9.25). Database hiện tại có persistent volume, sẽ không bị mất khi deploy lại.
- **Env vars duplicate**: Coolify app hiện có 36 env vars (18 keys × 2 entries). Đây là artifact từ lần migrate hạ tầng trước, không ảnh hưởng functionality (Coolify dùng giá trị mới nhất).

---

## [0.9.25] — 2026-08-15 — Giai đoạn 30: Stability Fix + Critical Bug Fixes (Login + Migration + Schema)

### Sửa lỗi (CRITICAL — Production-down)

- **[B1] Mọi login mới fail sau v0.9.24** — Migration 021 set `csrf_token VARCHAR(64) NOT NULL` trên bảng `sessions`, nhưng `auth.rs::google_callback` INSERT session mới không set `csrf_token` → fail với `null value in column "csrf_token" violates not-null constraint` → **100% new login bị fail**.
  - **Fix**: Sinh `csrf_token` random (64 hex chars = 32 bytes) + INSERT cùng session trong `auth.rs:240-260`.
  - **Tác động**: Đây là nguyên nhân chính khiến user báo "hỏng hết rồi" — app responsive nhưng không user nào login được.
- **[B2] Migration 021 fail vì thiếu pgcrypto** — `gen_random_bytes()` thuộc extension `pgcrypto`, nhưng không migration nào `CREATE EXTENSION pgcrypto`. Migration 021 fail tại `UPDATE sessions SET csrf_token = encode(gen_random_bytes(32), 'hex')` → cascade failure: các phần sau (rate_limit_log, login_attempts tables, NOT NULL constraint) không được tạo.
  - **Fix**: Thêm `CREATE EXTENSION IF NOT EXISTS pgcrypto;` ở đầu migration 021 (idempotent — safe để chạy nhiều lần).

### Sửa lỗi (HIGH — Functional)

- **[B3] `ensure_schema_safety` tạo bảng `permissions`/`role_permissions` với SAI column names** — Dùng `name` (thay vì `name_vi`), `role_code` (thay vì `role`), `permission_id` (thay vì `permission_code`), `assigned_at` (thay vì `granted_at`). Trên fresh deploy, safety_schema tạo bảng sai trước → migration 014 `CREATE TABLE IF NOT EXISTS` bị skip → INSERT fail vì column không tồn tại → cascading migration failure.
  - **Fix**: Đồng bộ column names với migration 014 trong `src/db/mod.rs:114-152`.
- **[B4] Rate limit cleanup task chạy trên instance throwaway (memory leak)** — `main.rs:140` gọi `spawn_cleanup_task(RateLimitState::new())` (instance MỚI với empty map), trong khi middleware thực tế dùng `RateLimitState::get_global()` (OnceLock singleton — instance KHÁC). Cleanup task làm trống map rỗng, không bao giờ dọn global map → memory leak theo thời gian (hàng chục nghìn entry tích lũy).
  - **Fix**: `spawn_cleanup_task(RateLimitState::get_global().clone())` trong `src/main.rs:140-142`.
- **[B5] `tim_kiem.rs` query dùng cột không tồn tại `cover_image_url`** — Bảng `books` có cột `cover_url` (không phải `cover_image_url`); bảng `groups` có `cover_upload_id` (không có `cover_image_url`). Cả 2 query SELECT fail → search books + groups luôn trả empty (bị nuốt bởi `unwrap_or_else(|e| vec![])`).
  - **Fix**: Đổi `cover_image_url` → `cover_url` cho books; dùng subquery join `images` cho groups trong `src/handlers/tim_kiem.rs:170, 208`.
- **[B6] admin_ky_thuat không đổi role được (permission inconsistency)** — Comment trong `admin.rs:411` nói "admin_ky_thuat và admin_quan_li có quyền [users_change_role]", nhưng `user.rs::has_permission_code("users_change_role")` cho admin_ky_thuat trả về false (chỉ admin_quan_li có). → admin_ky_thuat gọi `/admin/thanh-vien/{id}/role` sẽ bị 403 với message "Vai trò của bạn không có quyền đổi role user".
  - **Fix**: Thêm `users_change_role` vào match arm của admin_ky_thuat trong `user.rs::has_permission_code` (41 quyền thay vì 40). Thêm `'users_change_role'` vào migration 021 cho admin_ky_thuat. Update `permission_count()` 40 → 41.

### Sửa lỗi (MEDIUM — UI/UX)

- **[C1] Version drift trong footer** — `handlers/mod.rs:594` (placeholder_page) hiển thị "v0.9.21", `layout.html:422` hiển thị "v0.9.23". Cả 2 nên là "v0.9.25".
  - **Fix**: Cập nhật cả 2 thành "v0.9.25".
- **[C2] `BuddhaVowForm::validate` dùng byte length thay vì char count** — `content.len() < 10` đếm byte, không phải ký tự. Tiếng Việt có dấu là multi-byte UTF-8 (2-3 bytes/char) → validation sai (user viết 4 ký tự Việt có thể pass validation).
  - **Fix**: Dùng `chars().count()` trong `src/models/khong_gian.rs:163`.
- **[C3] `notifications_list` TOCTOU — đánh dấu all-read sau fetch** — Handler SELECT 50 notifications, rồi UPDATE mark all unread → read. Nếu notification mới đến giữa SELECT và UPDATE, nó bị mark read mà chưa hiển thị cho user.
  - **Fix**: UPDATE chỉ mark những id đã fetch (dùng `WHERE id = ANY($1)`) trong `src/handlers/friends.rs:1285-1301`.

### Thay đổi

- Cập nhật version v0.9.24 → v0.9.25, Giai đoạn 29 → 30
- `Cargo.toml`: bump version 0.9.24 → 0.9.25 (rust-version = "1.97.1" giữ nguyên)
- `Dockerfile.coolify`: tag `:sha-5367aaa` → SHA tag mới sau commit này
- `src/main.rs`: cập nhật log info, health check response (version, phase, phase_name, permission_counts 40→41 cho admin_ky_thuat), `HEALTH_FEATURES` (+9 features v0.9.25)
- `src/models/user.rs`: thêm `users_change_role` vào admin_ky_thuat permission list, update `permission_count()` 40 → 41
- `src/handlers/auth.rs`: thêm sinh `csrf_token` + bind vào INSERT session
- `src/handlers/tim_kiem.rs`: đổi `BookResult.cover_image_url` → `cover_url`; sửa SQL books + groups
- `src/handlers/friends.rs`: sửa `notifications_list` TOCTOU
- `src/handlers/mod.rs`: update footer version
- `src/db/mod.rs`: đồng bộ column names với migration 014 trong `ensure_schema_safety`
- `src/models/khong_gian.rs`: sửa `BuddhaVowForm::validate` dùng `chars().count()`
- `templates/layout.html`: update footer version v0.9.23 → v0.9.25
- `migrations/021_admin_equal_permissions.sql`: thêm `CREATE EXTENSION IF NOT EXISTS pgcrypto` ở đầu, thêm `'users_change_role'` cho admin_ky_thuat
- `README.md`: thêm section v0.9.25 — Giai đoạn 30, đẩy v0.9.24 xuống "Phiên bản trước"

---

## [0.9.24] — 2026-08-15 — Giai đoạn 29: Permission Redesign + SVG Redesign + Security Hardening + Deploy Fix

### Sửa lỗi (CRITICAL — Deploy)

- **[DEPLOY-1] v0.9.23 không được deploy thực sự** — Production vẫn chạy v0.9.22 mặc dù GitHub Actions build thành công và Coolify nhận trigger (HTTP 200, deployment_uuid trả về). Nguyên nhân: Docker daemon trên VPS cache stale digest của image `:0.9.23`, hoặc Coolify pull image nhưng container cũ chưa được stop hẳn. **Fix v0.9.24**: 
  - Bump tag image lên `:0.9.24` (tag mới chưa tồn tại trong cache → Docker chắc chắn pull image mới).
  - Thêm `CACHEBUSTER` env mới trên Coolify để invalidate cache.
  - Verify production `/api/health` trả về `version: 0.9.24` sau deploy.
  - Nếu vẫn không update, sẽ restart Coolify app thủ công qua API.

### Thay đổi (MAJOR — Permission Redesign)

- **[PERM-1] Bỏ hierarchy admin cũ** — Trước đây: admin_ky_thuat(5) > admin_quan_li(4) > admin_cong_dong(3) > mod(2) > member(1). **Giờ**: tất cả admin NGANG HÀNH (level 3), mỗi admin có scope quyền riêng theo phần phụ trách.
  - `admin_ky_thuat`: 40 quyền — system, security, technical infrastructure, media storage, analytics
  - `admin_quan_li`: 40 quyền — users (bao gồm change_role), content, community, fund, mail/notif
  - `admin_cong_dong`: 45 quyền — content, community, friends, mail, events, achievements, media mod
  - `mod`: 15 quyền — content moderation, chat moderation, basic community
  - `member`: 0 quyền admin
- **[PERM-2] Migration 021** — Re-seed `role_permissions` theo phân quyền mới. TRUNCATE cũ + INSERT theo scope. Thêm cột `csrf_token` vào `sessions`, `last_login_ip` + `last_login_at` vào `users`, `ip_address` vào `audit_log`. Tạo bảng `rate_limit_log` + `login_attempts`.
- **[PERM-3] `can_manage_*()` dùng permission check** — Thay vì `role_level() >= N`, giờ dùng `has_permission_code(code)`:
  - `can_manage_technical()` → check `system_view_status` (chỉ admin_ky_thuat)
  - `can_manage_admin()` → check `users_change_role` (admin_ky_thuat + admin_quan_li)
  - `can_manage_community()` → check `content_mod_reviews` (tất cả admin + mod)
  - `can_ban_user()` → NEW, check `users_ban` (admin_ky_thuat + admin_quan_li)
- **[PERM-4] Bỏ hierarchical role check** — Trong `admin_change_role`, bỏ check `new_role_level >= actor.role_level()` vì tất cả admin ngang hàng. Mọi admin có quyền `users_change_role` đều có thể đặt role bất kỳ cho user khác (chỉ trừ tự đổi role chính mình).
- **[PERM-5] Template users.html** — Cập nhật permission notice + actions dropdown để dùng `can_manage_admin()` và `can_ban_user()` thay vì `is_admin_ky_thuat() || is_admin_quan_li()`.

### Thêm mới (SVG Redesign)

- **[SVG-1] Redraw `favicon.svg`** — Hoa sen 3 lớp cánh (8 outer + 8 middle + 6 inner) + 2 lá sen base + tim sen vàng-xanh. Gradient hồng-đỏ-vàng-xanh, highlight ánh sáng, background circle cream. Đẹp hơn, chi tiết hơn, recognizable ở size 64x64.
- **[SVG-2] Tạo `logo.svg`** — Logo đầy đủ 128x128 cho landing page, có background circle + glow filter + nhụy sen. Dùng trên home hero.
- **[SVG-3] Tạo `logo-inline.svg`** — Logo inline 48x48 không background, fit trong header navbar button 36-40px.
- **[SVG-4] Layout.html + home.html** — Thay emoji 🪷 bằng `<img src="/static/logo-inline.svg">` trong header, `<img src="/static/logo.svg">` trong home hero. Giữ emoji 🪷 làm fallback alt text.

### Thêm mới (Security Hardening)

- **[SEC-1] Security Headers middleware** — `src/middleware/headers.rs` inject các headers vào mọi response:
  - `Content-Security-Policy`: default-src 'self', script-src 'self' + Tailwind/Alpine CDN + Google, style-src 'self' + inline + Google Fonts, img-src 'self' + data + blob + https, connect-src 'self' + wss + https, object-src 'none', frame-ancestors 'none', upgrade-insecure-requests
  - `X-Frame-Options: DENY` — chống clickjacking
  - `X-Content-Type-Options: nosniff` — chống MIME sniffing
  - `Referrer-Policy: strict-origin-when-cross-origin`
  - `Permissions-Policy`: camera=(), microphone=(), geolocation=(), payment=(), usb=(), etc.
  - `Strict-Transport-Security: max-age=63072000; includeSubDomains; preload` (HSTS 2 năm)
  - `Cross-Origin-Opener-Policy: same-origin`, `Cross-Origin-Resource-Policy: same-site`
  - `X-XSS-Protection: 1; mode=block` (legacy)
  - `X-DNS-Prefetch-Control: off`
- **[SEC-2] Rate Limiting middleware** — `src/middleware/rate_limit.rs` in-memory token bucket per IP + endpoint:
  - Auth endpoints (/dang-nhap, /auth/*): 10 req/phút — chống brute-force OAuth
  - Upload endpoints (/api/upload*): 10 req/phút — chống upload spam
  - API endpoints (/api/*): 60 req/phút — chống scraping
  - Profile update: 10 req/phút
  - POST endpoints: 30 req/phút — chống spam form
  - Social endpoints: 60 req/phút
  - General: 120 req/phút
  - Khi exceed: 429 Too Many Requests + Retry-After header
  - Background cleanup task mỗi 5 phút (xoá entries cũ)
- **[SEC-3] CSRF Protection middleware** — `src/middleware/csrf.rs` log-only mode (v0.9.24). Ghi log mọi POST/PUT/DELETE request để monitor. Sẽ chuyển sang block mode ở v0.9.25 sau khi all forms đã có CSRF hidden input. Whitelist: OAuth callback, /api/theme, /api/heartbeat, /ws/* (WebSocket).
- **[SEC-4] Audit log IP tracking** — Migration 021 thêm cột `ip_address` vào `audit_log`, `last_login_ip` + `last_login_at` vào `users`. Track IP mọi admin action + login.
- **[SEC-5] Login attempts table** — Migration 021 tạo bảng `login_attempts` (ip, email, success, attempted_at, user_agent). Track mọi login attempt để detect brute-force. (Sẽ integrate vào auth handler ở v0.9.25.)
- **[SEC-6] Rate limit log table** — Migration 021 tạo bảng `rate_limit_log` (ip, endpoint, hit_count, blocked_until). Persist rate limit state (tương lai có thể chuyển từ in-memory sang DB).
- **[SEC-7] Layout meta tags** — Thêm `<meta name="referrer" content="strict-origin-when-cross-origin">` + `<meta name="robots" content="index, follow, max-image-preview:large">` vào layout.html `<head>`.

### Thay đổi

- Cập nhật version v0.9.24, Giai đoạn 29
- `Cargo.toml`: bump version 0.9.23 → 0.9.24 (rust-version = "1.97.1" giữ nguyên)
- `Dockerfile.coolify`: tag `:0.9.23` → `:0.9.24`
- `src/main.rs`: thêm `mod middleware`, inject `RateLimitState`, thêm 3 security layers (`map_response` + 2× `from_fn`), cập nhật `HEALTH_FEATURES` (+12 features v0.9.24), cập nhật health check response (version, phase, permission_counts)
- `src/models/user.rs`: redesign `role_level()` (tất cả admin = 3), `can_manage_*()` dùng permission check, `has_permission_code()` update permission lists theo migration 021, `permission_count()` update (40/40/45/15/0)
- `src/handlers/admin.rs`: `admin_change_role` bỏ hierarchical check, `admin_ban_user`/`admin_activate_user` dùng `can_ban_user()`, `render_forbidden` update message, `fetch_users_list` update comment
- `templates/admin/users.html`: permission notice + actions dropdown dùng `can_manage_admin()` + `can_ban_user()`
- `templates/layout.html`: logo dùng SVG inline thay emoji 🪷, thêm meta tags bảo mật
- `templates/home.html`: hero logo dùng SVG
- Tạo `src/middleware/mod.rs`, `src/middleware/headers.rs`, `src/middleware/csrf.rs`, `src/middleware/rate_limit.rs`
- Tạo `src/static/logo.svg`, `src/static/logo-inline.svg`
- Redraw `src/static/favicon.svg`
- Tạo `migrations/021_admin_equal_permissions.sql`

---

## [0.9.23] — 2026-08-15 — Giai đoạn 28: Security Fix + Member Mgmt + Thuong Thanh + UI Fix

### Sửa lỗi (CRITICAL — Security)

- **[SEC-1] Health check lộ thông tin nhạy cảm** — `GET /api/health` công khai cho TẤT CẢ user, lộ DB version, features list, role hierarchy, permission counts, user counts. **Fix**: endpoint giờ yêu cầu auth + staff role (admin/mod). User thường nhận 401/403.
- **[SEC-2] Health Check link trên trang Tổng Quan** — User thường có thể click vào 💓 Health Check trên trang `/tong-quan` để xem thông tin hệ thống nhạy cảm. **Fix**: thay thế bằng link 👥 Đội Ngũ (công khai, an toàn).
- **[SEC-3] Chống leo thang đặc quyền** — Admin Quản Lý (level 4) có thể nâng user lên Admin Kỹ Thuật (level 5). **Fix**: thêm check `new_role_level >= actor.role_level()` — không cho nâng user lên role cao hơn hoặc bằng role của actor.
- **[SEC-4] DmCtrlMessage::Error dùng sai cho pong** — DM WebSocket handler gửi pong response qua `DmCtrlMessage::Error` variant (sai ngữ nghĩa), trong khi global chat dùng `CtrlMessage::Text`. **Fix**: đổi `DmCtrlMessage::Error` → `DmCtrlMessage::Text`, đồng bộ với chat.rs.

### Sửa lỗi (UI)

- **[UI-1] Danh sách bạn bè tràn trên mobile** — Nút "💬 Nhắn tin" và "✉️ Gửi thư" chiếm quá nhiều không gian, đè lên tên user, gây tràn layout trên điện thoại. **Fix**: chuyển sang icon-only trên mobile (chỉ hiện 💬, ✉️, ✕), responsive flex với gap/padding thích ứng. Avatar và text cũng thu nhỏ trên mobile.
- **[UI-2] Lời mời kết bạn tràn trên mobile** — Cùng vấn đề với danh sách bạn bè. **Fix**: responsive treatment đồng nhất.

### Thêm mới

- **Trang Thương Thành** (`GET /thuong-thanh`) — Marketplace của Ứng Dụng Từ Bi
  - 6 danh mục vật phẩm: Phật Tử, Kinh Sách, Đồ Cúng Tụ, Trang Phục, Dịch Vụ, Khác
  - Nguyên tắc Thương Thành (cúng dường đúng pháp, giá cả hợp lý, trao đổi thiện lành, an toàn minh bạch)
  - Thống kê, liên kết hữu ích
  - Handler riêng `handlers::thuong_thanh` + template `thuong-thanh/index.html`
  - Thay thế placeholder page cũ

- **Quản lý thành viên nhóm** (Group Member Management)
  - Route `POST /cong-dong/nhom/{slug}/duyet-thanh-vien/{member_id}` — duyệt thành viên đang chờ
  - Route `POST /cong-dong/nhom/{slug}/xoa-thanh-vien/{member_id}` — xóa thành viên
  - Section "Quản Lý Thành Viên" hiển thị trên trang nhóm khi user là owner/admin/staff
  - `GroupMemberWithUser` model mới với `role_display()`, `role_icon()`, `initial()` helpers
  - Không thể xóa chủ nhóm (bảo vệ)

- **Nút Đội Ngũ Quản Lí**
  - Thêm vào "Khám Phá Thêm" trên trang chủ (home.html)
  - Thêm vào section "Hệ Thống" trên trang Tổng Quan (thay Health Check)

### Thay đổi

- Cập nhật version v0.9.23, Giai đoạn 28
- `DmCtrlMessage` enum: `Error` → `Text` (đồng bộ với `CtrlMessage` trong chat.rs)
- `health_check()` → `health_check_secure()` + `health_check_inner()` (tách logic)
- Thêm `CookieJar` import trong main.rs
- Thêm route member management trong main.rs
- Thêm module `handlers::thuong_thanh`
- Thêm template `templates/thuong-thanh/index.html`
- Thêm `members: Vec<GroupMemberWithUser>` vào `GroupTemplate`
- Cập nhật Dockerfile.coolify tag → `:0.9.23`
- HEALTH_FEATURES thêm 5 features mới v0.9.23

---

## [0.9.22] — 2026-08-15 — Giai đoạn 27: Đội Ngũ Quản Lí + SQL Injection Fix + UI Fix

### Thêm mới

- **Trang Đội Ngũ Quản Lí** (`GET /doi-ngu-quan-li`) — công khai, không yêu cầu đăng nhập
  - Hiển thị 4 thành viên đội ngũ quản trị với thông tin chi tiết: họ tên, pháp danh, năm sinh, quê quán, tôn giáo, chức vụ, Facebook
  - Card grid responsive (1 cột mobile, 2 cột desktop), gradient accent theo role
  - Hệ thống phân cấp quản lí (5 cấp) hiển thị phía dưới
  - Link Facebook cho từng thành viên
  - Thêm vào mega menu (Hệ Thống), footer (Hệ Thống)

### Sửa lỗi

- **[SEC-1] SQL Injection Fix** — `bang_xep_hang.rs` và `quy_tu_bi.rs` dùng `format!()` để interpolate `limit` vào SQL
  - Fix: Bind `limit` as `$1` parameter trong tất cả queries
  - Bị ảnh hưởng: `fetch_leaderboard()` (5 tabs), `fetch_streak_leaderboard()`, `fetch_recent_donations()`, `fetch_top_donors()`, `fetch_recent_expenses()`

### Thay đổi

- Cập nhật version v0.9.22, Giai đoạn 27
- Thêm route `/doi-ngu-quan-li` vào main.rs
- Thêm module `handlers::doi_ngu`
- Thêm template `templates/doi-ngu-quan-li/index.html`
- Thêm link "👥 Đội Ngũ" vào navigation (mega menu + footer)
- Cập nhật Dockerfile.coolify tag → `:0.9.22`

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
