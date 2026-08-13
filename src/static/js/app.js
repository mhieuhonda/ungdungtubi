/**
 * Ứng Dụng Từ Bi - Client-side JavaScript
 * Domain: tubi.louis.vangioitutien.com
 * 
 * HTMX + Alpine.js helpers
 */

// HTMX configuration
document.addEventListener('htmx:configRequest', function(event) {
    // Add CSRF token if available
    const token = document.querySelector('meta[name="csrf-token"]');
    if (token) {
        event.detail.headers['X-CSRF-Token'] = token.content;
    }
});

// Prayer counter with localStorage persistence
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
    }
};

// Session heartbeat (keep session alive)
function sessionHeartbeat() {
    setInterval(() => {
        fetch('/api/heartbeat', { method: 'POST', credentials: 'same-origin' })
            .catch(() => {}); // Silently fail
    }, 5 * 60 * 1000); // Every 5 minutes
}

// Initialize
document.addEventListener('DOMContentLoaded', function() {
    // Start session heartbeat if logged in
    if (document.cookie.includes('session_id')) {
        sessionHeartbeat();
    }
    
    // Service Worker registration (for background prayer counting)
    if ('serviceWorker' in navigator) {
        // Will be implemented in Phase 5
    }
});

// Alpine.js global store
document.addEventListener('alpine:init', () => {
    Alpine.store('tubi', {
        // Current user info
        user: null,
        
        // Prayer state
        prayer: {
            count: PrayerCounter.getCount(),
            isRunning: false,
        },
        
        // Currency
        a: 0,
        k: 0,
        
        // Methods
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

// Utility: Format number with Vietnamese locale
function formatNumber(num) {
    return new Intl.NumberFormat('vi-VN').format(num);
}

// Utility: Time ago in Vietnamese
function timeAgo(date) {
    const seconds = Math.floor((Date.now() - date.getTime()) / 1000);
    
    if (seconds < 60) return 'vừa xong';
    if (seconds < 3600) return `${Math.floor(seconds / 60)} phút trước`;
    if (seconds < 86400) return `${Math.floor(seconds / 3600)} giờ trước`;
    if (seconds < 604800) return `${Math.floor(seconds / 86400)} ngày trước`;
    
    return new Intl.DateTimeFormat('vi-VN').format(date);
}
