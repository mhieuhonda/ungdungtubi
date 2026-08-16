-- =====================================================================
-- Migration 024 — Giai đoạn 39: Thương Thành MVP
-- v0.9.35 — 2026-08-16 (updated: removed Game store)
--
-- Mục tiêu:
--   Tạo schema cho Thương Thành (Marketplace) — theo tài liệu
--   "Hệ Thống Và Chức Năng Chi Tiết.docx" mục V:
--     * 2 cửa hàng: Cửa Hàng Ứng Dụng (app), PvP
--     * CRUD vật phẩm: tạo/xem/sửa/xoá items
--     * Giỏ hàng: thêm/xoá/thanh toán
--     * Giao dịch K: mua bán bằng tiền tệ K
--
-- Thiết kế:
--   1. shop_items — danh mục vật phẩm (cả 2 store)
--   2. cart_items — giỏ hàng tạm (chưa thanh toán)
--   3. transactions — lịch sử giao dịch K (mua/bán/chuyển)
--
-- Idempotent: tất cả CREATE TABLE IF NOT EXISTS — chạy lại không lỗi.
-- =====================================================================

-- 1. Bảng shop_items — vật phẩm trong Thương Thành
--    store = 'app'  → Cửa Hàng Ứng Dụng (system items, price cố định)
--    store = 'pvp'  → PvP (người dùng tự đăng bán, 20% fee)
CREATE TABLE IF NOT EXISTS shop_items (
    id              BIGSERIAL PRIMARY KEY,
    -- Loại cửa hàng: app / pvp
    store           TEXT        NOT NULL CHECK (store IN ('app', 'pvp')),
    -- Danh mục vật phẩm
    category        TEXT        NOT NULL,
    -- Tên vật phẩm (VD: "Thẻ Tự Tu", "Thuốc A", "Tinh Thể")
    name            TEXT        NOT NULL,
    -- Mô tả chi tiết
    description     TEXT,
    -- Giá tính bằng K (số nguyên, VD: 1, 5, 10, 100, 10000)
    price_k         INTEGER     NOT NULL CHECK (price_k >= 0),
    -- Biểu tượng emoji
    icon            TEXT        NOT NULL DEFAULT '📦',
    -- Màu sắc badge (hex)
    color           TEXT        NOT NULL DEFAULT '#0F766E',
    -- Người đăng (NULL = system item cho app store)
    seller_id       UUID        REFERENCES users(id) ON DELETE SET NULL,
    -- Số lượng có sẵn (NULL = vô hạn — dùng cho system items)
    stock           INTEGER     CHECK (stock IS NULL OR stock >= 0),
    -- Số lượng đã bán
    sold_count      INTEGER     NOT NULL DEFAULT 0,
    -- Trạng thái: active / inactive / sold_out
    status          TEXT        NOT NULL DEFAULT 'active'
                                CHECK (status IN ('active', 'inactive', 'sold_out')),
    -- Ảnh bìa (URL)
    image_url       TEXT,
    -- Phép thuật/hiệu ứng (JSONB, tuỳ loại vật phẩm)
    effects         JSONB       DEFAULT '{}',
    -- Sắp xếp thứ tự
    sort_order      INTEGER     NOT NULL DEFAULT 0,
    -- PvP listing: thời hạn đăng tối đa 7 ngày
    expires_at      TIMESTAMPTZ,
    is_active       BOOLEAN     NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_shop_items_store_category
    ON shop_items (store, category, sort_order)
    WHERE is_active = true;

CREATE INDEX IF NOT EXISTS idx_shop_items_seller
    ON shop_items (seller_id)
    WHERE store = 'pvp';

-- 2. Bảng cart_items — giỏ hàng (chưa thanh toán)
--    Mỗi user có thể thêm nhiều item vào giỏ, khi thanh toán thì chuyển sang transaction
CREATE TABLE IF NOT EXISTS cart_items (
    id              BIGSERIAL PRIMARY KEY,
    user_id         UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    item_id         BIGINT      NOT NULL REFERENCES shop_items(id) ON DELETE CASCADE,
    -- Số lượng muốn mua
    quantity        INTEGER     NOT NULL DEFAULT 1 CHECK (quantity > 0),
    -- Thời gian thêm vào giỏ
    added_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, item_id)
);

CREATE INDEX IF NOT EXISTS idx_cart_items_user
    ON cart_items (user_id);

-- 3. Bảng transactions — lịch sử giao dịch K
--    Ghi lại mọi giao dịch mua/bán/chuyển K
CREATE TABLE IF NOT EXISTS transactions (
    id              BIGSERIAL PRIMARY KEY,
    -- Loại giao dịch: purchase (mua), sale (bán), transfer (chuyển K), refund (hoàn)
    tx_type         TEXT        NOT NULL CHECK (tx_type IN ('purchase', 'sale', 'transfer', 'refund')),
    -- Người mua / người gửi
    buyer_id        UUID        NOT NULL REFERENCES users(id) ON DELETE SET NULL,
    -- Người bán / người nhận (NULL = hệ thống)
    seller_id       UUID        REFERENCES users(id) ON DELETE SET NULL,
    -- Vật phẩm (NULL nếu transfer K thuần)
    item_id         BIGINT      REFERENCES shop_items(id) ON DELETE SET NULL,
    -- Số lượng
    quantity        INTEGER     NOT NULL DEFAULT 1 CHECK (quantity > 0),
    -- Số K giao dịch (tổng = price_k * quantity)
    amount_k        INTEGER     NOT NULL CHECK (amount_k >= 0),
    -- Phí giao dịch (PvP: 20% fee)
    fee_k           INTEGER     NOT NULL DEFAULT 0 CHECK (fee_k >= 0),
    -- Trạng thái: pending / completed / failed / refunded
    status          TEXT        NOT NULL DEFAULT 'completed'
                                CHECK (status IN ('pending', 'completed', 'failed', 'refunded')),
    -- Ghi chú
    note            TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_transactions_buyer
    ON transactions (buyer_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_transactions_seller
    ON transactions (seller_id, created_at DESC);

-- =====================================================================
-- Seed data — Vật phẩm hệ thống cho Cửa Hàng Ứng Dụng (store = 'app')
-- Theo tài liệu "Hệ Thống Và Chức Năng Chi Tiết.docx" mục V.1
-- =====================================================================

INSERT INTO shop_items (store, category, name, description, price_k, icon, color, stock, sort_order, effects)
VALUES
    -- 📿 Thẻ tu học
    ('app', 'the_tu_hoc', 'Thẻ Tự Tu', 'Thẻ tự tu — hành trình tu học cá nhân. Tăng giới hạn niệm Phật +1000/lần.', 1, '📿', '#2E7D32', NULL, 1, '{"niem_bonus": 1000}'),
    ('app', 'the_tu_hoc', 'Thẻ Cộng Tu', 'Thẻ cộng tu — kết nối tu học cùng đạo hữu. Tăng giới hạn niệm Phật +2000/lần.', 2, '🪷', '#1565C0', NULL, 2, '{"niem_bonus": 2000}'),
    ('app', 'the_tu_hoc', 'Thẻ Exp', 'Thẻ kinh nghiệm — tăng tốc tích lũy kinh nghiệm tu hành.', 5, '⭐', '#FF6F00', NULL, 3, '{"exp_multiplier": 1.5}'),

    -- 🏷️ Thẻ đổi tên / pháp danh
    ('app', 'the_doi_ten', 'Thẻ Đổi Tên', 'Đổi tên hiển thị 1 lần.', 5, '✏️', '#6A1B9A', NULL, 10, '{}'),
    ('app', 'the_doi_ten', 'Thẻ Pháp Danh', 'Cấp/đổi pháp danh — danh xưng Phật giáo.', 5, '🙏', '#0F766E', NULL, 11, '{}'),
    ('app', 'the_doi_ten', 'Thẻ Pháp Hiệu', 'Cấp/đổi pháp hiệu — hiệu Phật giáo.', 5, '🕯️', '#795548', NULL, 12, '{}'),

    -- 🤝 Thẻ hỗ trợ
    ('app', 'the_ho_tro', 'Thẻ Ủng Hộ', 'Ủng hộ cộng đồng Từ Bi — tất cả K góp vào Quỹ Từ Bi.', 5, '🤝', '#C62828', NULL, 20, '{"donate_to_fund": true}'),
    ('app', 'the_ho_tro', 'Thẻ Bẫy Quà', 'Đặt bẫy quà — thành viên may mắn nhận được quà ngẫu nhiên.', 5, '🎁', '#E91E63', NULL, 21, '{}'),
    ('app', 'the_ho_tro', 'Thẻ Hộp Quà', 'Mở hộp quà — nhận vật phẩm ngẫu nhiên giá trị cao.', 10, '📦', '#FF5722', NULL, 22, '{}'),

    -- 👥 Thẻ nhóm
    ('app', 'the_nhom', 'Thẻ Tạo Nhóm', 'Tạo nhóm mới trong Cộng Đồng.', 10, '👥', '#3F51B5', NULL, 30, '{}'),
    ('app', 'the_nhom', 'Thẻ Tạo Không Gian Nhóm', 'Tạo Không Gian riêng cho nhóm — không gian tu hành chung.', 100, '🏠', '#009688', NULL, 31, '{}'),
    ('app', 'the_nhom', 'Thẻ Mời Cộng Tu', 'Mời bạn bè cùng tu — gói 10 lời mời.', 10, '✉️', '#607D8B', NULL, 32, '{"invite_count": 10}'),

    -- 🗳️ Thẻ bầu chọn
    ('app', 'the_bau_chon', 'Thẻ Bầu Chọn', 'Tạo cuộc bầu chọn trong nhóm/cộng đồng.', 50, '🗳️', '#673AB7', NULL, 40, '{}'),

    -- 🌸 Hoa hồng & vật phẩm
    ('app', 'vat_pham', 'Hoa Hồng', 'Tặng hoa hồng cho bài viết/review — thể hiện trân trọng.', 10, '🌹', '#E91E63', NULL, 50, '{}'),
    ('app', 'vat_pham', 'Ô Vật Phẩm', 'Mở ô vật phẩm — chứa 5 slot lưu trữ.', 10, '🗂️', '#795548', NULL, 51, '{"storage_slots": 5}'),
    ('app', 'vat_pham', 'Thẻ Yêu Cầu', 'Gửi yêu cầu đặc biệt đến admin/ban quản lý.', 20, '📋', '#455A64', NULL, 52, '{}'),

    -- 🪷 Vật phẩm cao cấp
    ('app', 'cao_cap', 'Phiếu Từ Bi', 'Phiếu Từ Bi — chuyển đổi sang Bi sau 100 ngày. Giá trị lớn nhất.', 100, '🪷', '#0F766E', NULL, 60, '{"converts_to_bi": true, "days": 100}'),
    ('app', 'cao_cap', 'Thẻ Người Tốt', 'Danh hiệu Người Tốt — vinh danh đóng góp thiện lành.', 100, '🏅', '#FFD600', NULL, 61, '{}'),
    ('app', 'cao_cap', 'Thẻ Thiện Nhân', 'Danh hiệu Thiện Nhân — hạng cao nhất cho đạo hữu từ bi.', 10000, '👑', '#FFD600', NULL, 62, '{}')
ON CONFLICT DO NOTHING;

-- =====================================================================
-- Comments
-- =====================================================================

COMMENT ON TABLE shop_items IS
    'v0.9.35 — Vật phẩm Thương Thành. 2 store: app (hệ thống), pvp (người dùng đăng bán).';
COMMENT ON TABLE cart_items IS
    'v0.9.35 — Giỏ hàng: user thêm vật phẩm trước khi thanh toán. Mỗi user-item unique.';
COMMENT ON TABLE transactions IS
    'v0.9.35 — Giao dịch K: mua/bán/chuyển/refund. Ghi lại mọi giao dịch tiền tệ K.';
