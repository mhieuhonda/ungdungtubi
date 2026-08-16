/**
 * Ứng Dụng Từ Bi - Client-side JavaScript (main entry)
 * Domain: tubi.louis.vangioitutien.com
 *
 * v0.9.21 — Giai đoạn 26:
 *   - Xoá window.toggleSound (sound effects đã bị loại bỏ hoàn toàn)
 *   - Xoá TubiSound references
 *   - Chat components chuyển sang /static/js/chat.js
 *
 * Load order trong layout.html:
 *   1. /static/js/chat.js   (chat Alpine.js components)
 *   2. /static/js/app.js    (this file — main init)
 */

// HTMX configuration — add CSRF token if available
document.addEventListener('htmx:configRequest', function (event) {
    const token = document.querySelector('meta[name="csrf-token"]');
    if (token) {
        event.detail.headers['X-CSRF-Token'] = token.content;
    }
});

// ====================================================================
// Prayer counter with localStorage persistence
// ====================================================================

const PrayerCounter = {
    key: 'tubi_prayer_count',

    getCount() {
        return parseInt(localStorage.getItem(this.key) || '0', 10);
    },

    setCount(count) {
        localStorage.setItem(this.key, count.toString());
    },

    increment() {
        const count = this.getCount() + 1;
        this.setCount(count);
        return count;
    },
};

// ====================================================================
// Session heartbeat (keep session alive)
// v0.9.29: Tăng interval từ 5 phút → 10 phút để giảm tải server.
// ====================================================================

function sessionHeartbeat() {
    setInterval(() => {
        fetch('/api/heartbeat', { method: 'POST', credentials: 'same-origin' }).catch(() => {});
    }, 10 * 60 * 1000);
}

// ====================================================================
// Initialize on DOM ready
// ====================================================================

document.addEventListener('DOMContentLoaded', function () {
    const bodyLoggedIn = document.body && document.body.dataset.loggedIn === 'true';
    if (bodyLoggedIn) {
        sessionHeartbeat();
    }

    if ('serviceWorker' in navigator) {
        // Will be implemented in Phase 5
    }
});

// ====================================================================
// Alpine.js global store
// ====================================================================

document.addEventListener('alpine:init', () => {
    Alpine.store('tubi', {
        user: null,
        prayer: {
            count: PrayerCounter.getCount(),
            isRunning: false,
        },
        a: 0,
        k: 0,

        incrementPrayer() {
            this.prayer.count = PrayerCounter.increment();
            this.a++;
            if (this.a >= 1000) {
                this.k++;
                this.a -= 1000;
            }
        },
    });
});

// ====================================================================
// Utility functions
// ====================================================================

function formatNumber(num) {
    return new Intl.NumberFormat('vi-VN').format(num);
}

function timeAgo(date) {
    const seconds = Math.floor((Date.now() - date.getTime()) / 1000);

    if (seconds < 60) return 'vừa xong';
    if (seconds < 3600) return Math.floor(seconds / 60) + ' phút trước';
    if (seconds < 86400) return Math.floor(seconds / 3600) + ' giờ trước';
    if (seconds < 604800) return Math.floor(seconds / 86400) + ' ngày trước';

    return new Intl.DateTimeFormat('vi-VN').format(date);
}

window.formatNumber = formatNumber;
window.timeAgo = timeAgo;
window.PrayerCounter = PrayerCounter;

// ====================================================================
// v0.9.37 — 429-aware fetch wrapper + global toast notification
// ====================================================================
// Vấn đề: Khi user đổi tab liên tục hoặc click nhanh nhiều lần, các fetch()
// gọi tới /api/ban-be/thong-bao/chua-doc, /api/heartbeat, /api/chat-chung/history
// có thể vượt rate limit (60/phút API + 60/phút social) → server trả 429.
// Trước đây 429 trả plain-text "429 — Quá nhiều request..." → fetch() chỉ throw
// error mà không có UX recovery. User không biết tại sao fetch fail.
//
// Giải pháp v0.9.37:
//   1. Wrap window.fetch với tubiFetch() — tự catch 429, hiển thị toast
//      "Vui lòng chờ X giây" + đọc Retry-After header + disable nút bấm.
//   2. Tạo 1 toast container toàn cục (tubi-toast-container) render từ app.js.
//   3. KHÔNG override window.fetch (sẽ phá các thư viện dùng fetch internally)
//      → chỉ expose window.tubiFetch để code app tự dùng.
//   4. Code hiện có (chat.js, notifications.html) gọi fetch() trực tiếp — không
//      bắt buộc refactor. Wrapper chỉ dùng cho code mới.
//
// Lưu ý: server-side đã tăng limit (api 60→180, social 60→180, general 120→300)
// → 429 giờ RẤT hiếm khi xảy ra trong usage bình thường. Wrapper này là safety net.

(function initTubiToast() {
    // Tạo toast container nếu chưa có
    let container = document.getElementById('tubi-toast-container');
    if (!container) {
        container = document.createElement('div');
        container.id = 'tubi-toast-container';
        container.style.cssText = [
            'position:fixed',
            'top:1rem',
            'right:1rem',
            'z-index:9999',
            'display:flex',
            'flex-direction:column',
            'gap:0.5rem',
            'pointer-events:none',
            'max-width:calc(100vw - 2rem)',
        ].join(';');
        document.body.appendChild(container);
    }
})();

/**
 * Hiển thị toast notification (auto-dismiss sau 4s).
 * @param {string} message - Nội dung toast
 * @param {string} type - 'info' | 'success' | 'warning' | 'error'
 */
window.tubiToast = function(message, type = 'info') {
    const container = document.getElementById('tubi-toast-container');
    if (!container) return;
    const colors = {
        info:    { bg: '#dbeafe', border: '#bfdbfe', text: '#1e40af', emoji: 'ℹ️' },
        success: { bg: '#dcfce7', border: '#bbf7d0', text: '#14532d', emoji: '✅' },
        warning: { bg: '#fef3c7', border: '#fde68a', text: '#78350f', emoji: '⚠️' },
        error:   { bg: '#fee2e2', border: '#fecaca', text: '#7f1d1d', emoji: '❌' },
    };
    const c = colors[type] || colors.info;
    const toast = document.createElement('div');
    toast.style.cssText = [
        `background:${c.bg}`,
        `border:1px solid ${c.border}`,
        `color:${c.text}`,
        'padding:0.75rem 1rem',
        'border-radius:0.5rem',
        'font-size:0.875rem',
        'box-shadow:0 4px 6px -1px rgba(0,0,0,0.1)',
        'display:flex',
        'align-items:center',
        'gap:0.5rem',
        'pointer-events:auto',
        'max-width:360px',
        'transition:opacity 0.3s',
    ].join(';');
    toast.innerHTML = `<span style="font-size:1rem">${c.emoji}</span><span>${message}</span>`;
    container.appendChild(toast);
    setTimeout(() => {
        toast.style.opacity = '0';
        setTimeout(() => toast.remove(), 300);
    }, 4000);
};

/**
 * v0.9.37 — 429-aware fetch wrapper.
 * Tự catch HTTP 429, đọc Retry-After header, hiển thị toast, và reject promise
 * với error có `.retryAfter` property để caller có thể handle thêm nếu cần.
 *
 * Usage: thay `fetch(url, opts)` bằng `tubiFetch(url, opts)`.
 *
 * @returns {Promise<Response>} - Resolve nếu status !== 429, Reject nếu 429.
 */
window.tubiFetch = async function(url, opts = {}) {
    const res = await fetch(url, opts);
    if (res.status === 429) {
        const retryAfter = parseInt(res.headers.get('retry-after') || '30', 10);
        let group = 'general';
        let serverMsg = '';
        try {
            // Server có thể trả JSON nếu request có Accept: application/json
            const ct = res.headers.get('content-type') || '';
            if (ct.includes('application/json')) {
                const data = await res.json();
                group = data.group || group;
                serverMsg = data.message || serverMsg;
            }
        } catch (_) {}
        const msg = serverMsg || `Quá nhiều request. Vui lòng thử lại sau ${retryAfter} giây. 🪷`;
        window.tubiToast(msg, 'warning');
        const err = new Error(`HTTP 429: ${msg}`);
        err.retryAfter = retryAfter;
        err.group = group;
        err.isRateLimited = true;
        throw err;
    }
    return res;
};

// ====================================================================
// v0.9.37 — Pause polling when tab is hidden (giảm 429 risk)
// ====================================================================
// Vấn đề: Khi user mở nhiều tab, mỗi tab đều poll /api/ban-be/thong-bao/chua-doc
// mỗi 60s. Nếu user mở 5 tab → 5 requests mỗi 60s = 300/h, có thể vượt limit.
//
// Giải pháp: Khi document.hidden = true (tab không visible), pause tất cả poll.
// Khi tab visible lại, resume poll ngay lập tức.
//
// Cách implement: expose window.__tubiPollingPaused boolean, chat.js sẽ check
// trước khi poll. Nếu paused → skip, chờ tab visible lại.

window.__tubiPollingPaused = false;
document.addEventListener('visibilitychange', () => {
    const wasPaused = window.__tubiPollingPaused;
    window.__tubiPollingPaused = document.hidden;
    if (wasPaused && !document.hidden) {
        // Tab vừa visible lại — dispatch event để poll ngay lập tức
        window.dispatchEvent(new CustomEvent('tubi-tab-visible'));
    }
});
