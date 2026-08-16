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
