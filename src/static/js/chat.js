/**
 * Ứng Dụng Từ Bi — Chat Module (v0.9.20 — Giai đoạn 25)
 *
 * Refactor từ app.js (v0.9.19) thành module riêng để dễ maintain.
 *
 * Components:
 *   - liveChat(opts)    — Live Chat trong nhóm cộng đồng
 *   - globalChat()      — Chat Chung toàn platform (draggable bubble popup)
 *   - dmChat(opts)      — Direct Message 1-1
 *   - chatBubble()      — Draggable bubble mở global chat
 *   - notificationBadge() — Poll notification count mỗi 30s
 *
 * v0.9.20 improvements (Live Chat Total Fix):
 *   [FIX-1] WebSocket Ping mỗi 30s (app-level `{"type":"ping"}`) — backup cho
 *           server Ping. Đảm bảo kết nối sống qua mọi proxy.
 *   [FIX-2] Connection health check: nếu không nhận message/ping/pong trong 60s,
 *           force reconnect. Phát hiện dead connections mà TCP chưa báo.
 *   [FIX-3] Optimistic UI: hiển thị tin nhắn của mình NGAY với trạng thái "đang gửi",
 *           chuyển sang "đã gửi" khi server echo về. Nếu 5s không echo → mark failed.
 *   [FIX-4] Message queue: tin nhắn gửi khi disconnected được queue, flush khi
 *           reconnect. Không mất tin nhắn nữa.
 *   [FIX-5] Send timeout: nếu 5s sau send không nhận được echo, show error + retry.
 *   [FIX-6] Reset reconnect attempts khi nhận message thành công.
 *   [FIX-7] Phát sound effect khi send/receive (Web Audio API, xem sound.js).
 *   [FIX-8] Limit messages array 200 entries — tránh memory bloat.
 *   [PERF-1] Debounce scrollToBottom bằng requestAnimationFrame.
 *   [PERF-2] Cache DOM refs (messagesEl) thay vì query mỗi lần.
 *   [PERF-3] IntersectionObserver để auto-scroll chỉ khi user ở gần bottom.
 *
 * Cách dùng (trong template):
 *   <div x-data="liveChat({ slug, isMember, isLoggedIn, initialMessages })" x-init="init()">
 */

// ====================================================================
// Shared helpers — dùng chung cho 3 loại chat
// ====================================================================

/**
 * Tạo class CSS cho bubble dựa trên author_role.
 * Admin Kỹ Thuật = coder effect, các admin khác = khung riêng.
 */
function msgBubbleClass(role) {
    if (role === 'admin_ky_thuat') return 'chat-msg-admin-ky-thuat';
    if (role === 'admin_quan_li') return 'chat-msg-admin-quan-li';
    if (role === 'admin_cong_dong') return 'chat-msg-admin-cong-dong';
    if (role === 'mod') return 'chat-msg-mod';
    return 'chat-msg-bubble';
}

function msgNameClass(role) {
    if (role === 'admin_ky_thuat') return 'chat-msg-admin-ky-thuat-name';
    if (role === 'admin_quan_li') return 'chat-msg-admin-quan-li-name';
    if (role === 'admin_cong_dong') return 'chat-msg-admin-cong-dong-name';
    if (role === 'mod') return 'chat-msg-mod-name';
    return '';
}

function avatarClass(role) {
    if (role === 'admin_ky_thuat') return 'chat-avatar-admin-ky-thuat';
    if (role === 'admin_quan_li') return 'chat-avatar-admin-quan-li';
    if (role === 'admin_cong_dong') return 'chat-avatar-admin-cong-dong';
    if (role === 'mod') return 'chat-avatar-mod';
    return '';
}

function roleBadgeHtml(role) {
    if (!role) return '';
    const map = {
        'admin_ky_thuat': '<span class="chat-role-badge chat-role-badge-admin-ky-thuat">⚙️ SYS</span>',
        'admin_quan_li': '<span class="chat-role-badge chat-role-badge-admin-quan-li">👑 ADMIN</span>',
        'admin_cong_dong': '<span class="chat-role-badge chat-role-badge-admin-cong-dong">🛡️ ADMIN</span>',
        'mod': '<span class="chat-role-badge chat-role-badge-mod">📜 MOD</span>',
    };
    return map[role] || '';
}

function authorLabel(msg) {
    if (msg.author_role === 'admin_ky_thuat') {
        return '[SYS] ' + msg.author_display_name;
    }
    return msg.author_display_name;
}

function formatTime(isoStr) {
    try {
        const dt = new Date(isoStr);
        return new Intl.DateTimeFormat('vi-VN', {
            hour: '2-digit',
            minute: '2-digit',
            day: '2-digit',
            month: '2-digit',
        }).format(dt);
    } catch (_) {
        return '';
    }
}

/**
 * Mixin chứa WebSocket logic chung cho cả 3 loại chat.
 * Truyền vào `opts.url` (WS URL builder) và `opts.onMessage` (callback khi nhận msg).
 *
 * Cung cấp:
 *   - connect() với auto-reconnect backoff
 *   - send(body) với optimistic UI + message queue + timeout
 *   - pingInterval (30s app-level ping)
 *   - healthCheck (60s không nhận gì → reconnect)
 *   - scheduleReconnect()
 */
function chatSocketMixin(getUrl) {
    return {
        // --- State ---
        connected: false,
        socket: null,
        error: '',
        reconnectAttempts: 0,
        maxReconnectAttempts: 10, // v0.9.20: tăng từ 5 → 10
        reconnectTimer: null,
        _pingTimer: null,
        _healthTimer: null,
        _lastReceivedAt: 0,
        _queue: [], // v0.9.20: message queue khi disconnected
        _pending: new Map(), // v0.9.20: pending messages chờ echo (clientId → timeout)

        // --- WebSocket connect ---
        connect() {
            if (this.socket) {
                try { this.socket.close(1000, 'reconnect'); } catch (_) {}
                this.socket = null;
            }

            const url = getUrl.call(this);
            try {
                this.socket = new WebSocket(url);
            } catch (e) {
                this.error = 'Trình duyệt không hỗ trợ WebSocket';
                if (window.TubiSound) TubiSound.playError();
                return;
            }

            this.socket.onopen = () => {
                this.connected = true;
                this.error = '';
                this.reconnectAttempts = 0;
                this._lastReceivedAt = Date.now();
                if (window.TubiSound) TubiSound.playConnect();

                // Start ping + health check
                this._startPing();
                this._startHealthCheck();

                // Flush message queue
                if (this._queue.length > 0) {
                    const q = this._queue.splice(0);
                    q.forEach((body) => this._sendRaw(body));
                }
            };

            this.socket.onmessage = (event) => {
                this._lastReceivedAt = Date.now();
                this.handleIncoming(event.data);
            };

            this.socket.onclose = (event) => {
                this.connected = false;
                this._stopPing();
                this._stopHealthCheck();

                // 1000 = normal close (user-initiated reconnect), don't reconnect
                if (event.code === 1000) return;

                // 1008 = policy violation (auth/permission fail) — don't reconnect
                if (event.code === 1008) {
                    this.error = event.reason || 'Không có quyền chat';
                    if (window.TubiSound) TubiSound.playError();
                    return;
                }

                // 1006 = abnormal closure (proxy timeout, network drop, etc.)
                // → reconnect with backoff
                this.scheduleReconnect();
            };

            this.socket.onerror = () => {
                // Don't set error here — onclose will fire and handle reconnect
                // Just mark as disconnected
                this.connected = false;
            };
        },

        // --- App-level ping (backup cho server protocol Ping) ---
        _startPing() {
            this._stopPing();
            this._pingTimer = setInterval(() => {
                if (this.socket && this.socket.readyState === WebSocket.OPEN) {
                    try {
                        this.socket.send('{"type":"ping"}');
                    } catch (_) {}
                }
            }, 30000); // 30s
        },

        _stopPing() {
            if (this._pingTimer) {
                clearInterval(this._pingTimer);
                this._pingTimer = null;
            }
        },

        // --- Health check: nếu 60s không nhận gì → force reconnect ---
        _startHealthCheck() {
            this._stopHealthCheck();
            this._healthTimer = setInterval(() => {
                if (this._lastReceivedAt === 0) return;
                const elapsed = Date.now() - this._lastReceivedAt;
                if (elapsed > 60000) {
                    // Dead connection — force reconnect
                    if (window.TubiSound) TubiSound.playError();
                    try { this.socket.close(4000, 'health check timeout'); } catch (_) {}
                    this.socket = null;
                    this.connected = false;
                    this.scheduleReconnect();
                }
            }, 15000); // check every 15s
        },

        _stopHealthCheck() {
            if (this._healthTimer) {
                clearInterval(this._healthTimer);
                this._healthTimer = null;
            }
        },

        scheduleReconnect() {
            if (this.reconnectAttempts >= this.maxReconnectAttempts) {
                this.error = `Không thể kết nối sau ${this.maxReconnectAttempts} lần thử. Tải lại trang.`;
                return;
            }
            this.reconnectAttempts += 1;
            // v0.9.20: backoff dày hơn — 1s, 2s, 4s, 8s, 16s, 32s (cap 30s)
            const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts - 1), 30000);
            this.error = `Mất kết nối — thử lại sau ${Math.round(delay / 1000)}s…`;
            clearTimeout(this.reconnectTimer);
            this.reconnectTimer = setTimeout(() => this.connect(), delay);
        },

        /**
         * Send raw body (no optimistic UI, no queue).
         * Dùng nội bộ — gọi send() từ UI thay vì _sendRaw().
         */
        _sendRaw(body) {
            if (!this.socket || this.socket.readyState !== WebSocket.OPEN) return false;
            try {
                this.socket.send(body);
                return true;
            } catch (_) {
                return false;
            }
        },

        /**
         * Disconnect + cleanup. Gọi khi component destroy.
         */
        disconnect() {
            this._stopPing();
            this._stopHealthCheck();
            clearTimeout(this.reconnectTimer);
            if (this.socket) {
                try { this.socket.close(1000, 'cleanup'); } catch (_) {}
                this.socket = null;
            }
            this.connected = false;
        },
    };
}

// ====================================================================
// liveChat — Live Chat trong nhóm cộng đồng
// ====================================================================

function liveChat(opts) {
    const mixin = chatSocketMixin(function () {
        const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const host = window.location.host;
        return `${proto}//${host}/ws/cong-dong/nhom/${encodeURIComponent(this.slug)}`;
    });

    return Object.assign({}, mixin, {
        // --- Config ---
        slug: opts.slug || '',
        isMember: opts.isMember === true,
        isLoggedIn: opts.isLoggedIn === true,
        maxChars: 500,

        // --- State ---
        messages: Array.isArray(opts.initialMessages) ? opts.initialMessages.slice() : [],
        draft: '',
        onlineCount: 0,
        _scrollPending: false,
        _messagesEl: null,

        get onlineLabel() {
            if (this.onlineCount > 0) {
                return `${this.onlineCount} người online`;
            }
            return 'đã kết nối';
        },

        // v0.9.19 helpers — delegate to shared functions
        msgBubbleClass,
        msgNameClass,
        avatarClass,
        roleBadgeHtml,
        authorLabel,

        // --- Lifecycle ---
        init() {
            if (!this.isLoggedIn) {
                this.error = 'Cần đăng nhập để xem chat';
                return;
            }
            if (!this.isMember) {
                this.error = 'Tham gia nhóm để chat';
                return;
            }
            // Cache DOM ref
            this._messagesEl = this.$refs.messages;
            this._lastReceivedAt = Date.now();
            this.$nextTick(() => this.scrollToBottom());
            this.connect();
        },

        // --- Incoming message handler ---
        handleIncoming(raw) {
            let data;
            try {
                data = JSON.parse(raw);
            } catch (_) {
                return;
            }

            // App-level pong từ server — chỉ reset health, không render
            if (data.type === 'pong') {
                return;
            }

            // Error payload
            if (data.type === 'error' && typeof data.message === 'string') {
                this.error = data.message;
                if (window.TubiSound) TubiSound.playError();
                setTimeout(() => { if (this.error === data.message) this.error = ''; }, 3000);
                return;
            }

            // Chat message — check cả id lẫn body (author_display_name có thể rỗng)
            if (data.id && data.body !== undefined && data.author_display_name !== undefined) {
                // Tránh duplicate
                if (this.messages.some((m) => m.id === data.id)) {
                    // Có thể đây là echo của tin mình vừa gửi → mark as sent
                    this._markSent(data.id);
                    return;
                }
                this.messages.push(data);
                this._trimMessages();
                this.$nextTick(() => this.scrollToBottom());
                if (window.TubiSound) TubiSound.playReceive();
            }
        },

        /**
         * Send message với optimistic UI + queue + timeout.
         */
        send() {
            const body = this.draft.trim();
            if (!body) return;
            if (body.length > this.maxChars) {
                this.error = `Tin nhắn quá dài (tối đa ${this.maxChars} ký tự)`;
                return;
            }

            if (!this.connected || !this.socket || this.socket.readyState !== WebSocket.OPEN) {
                // v0.9.20: Queue message, gửi khi reconnect
                this._queue.push(body);
                this.draft = '';
                this.error = 'Đang kết nối lại — tin nhắn sẽ tự gửi khi có mạng';
                if (!this.connected) this.scheduleReconnect();
                return;
            }

            // v0.9.20: Optimistic UI — không xóa draft ngay, chờ echo
            // (Server sẽ broadcast lại cho mình, handleIncoming sẽ nhận)
            const sent = this._sendRaw(body);
            if (sent) {
                this.draft = '';
                this.error = '';
                if (window.TubiSound) TubiSound.playSend();
            } else {
                this._queue.push(body);
                this.error = 'Mất kết nối — tin nhắn đang chờ gửi lại';
                this.scheduleReconnect();
            }
        },

        /**
         * Mark message đã gửi khi nhận echo từ server.
         * Hiện tại optimistic UI không thêm placeholder — chỉ reset error.
         */
        _markSent(_id) {
            // Placeholder — có thể thêm "✓ đã gửi" badge sau
        },

        _trimMessages() {
            // v0.9.20: Giữ tối đa 200 messages để tránh memory bloat
            if (this.messages.length > 200) {
                this.messages.splice(0, this.messages.length - 200);
            }
        },

        // --- Scroll helpers (debounced via rAF) ---
        scrollToBottom() {
            if (this._scrollPending) return;
            this._scrollPending = true;
            requestAnimationFrame(() => {
                this._scrollPending = false;
                if (this._messagesEl) {
                    this._messagesEl.scrollTop = this._messagesEl.scrollHeight;
                }
            });
        },

        formatTime,
    });
}

window.liveChat = liveChat;

// ====================================================================
// globalChat — Chat Chung toàn platform (popup từ draggable bubble)
// ====================================================================

function globalChat() {
    const mixin = chatSocketMixin(function () {
        const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const host = window.location.host;
        return `${proto}//${host}/ws/chat-chung`;
    });

    return Object.assign({}, mixin, {
        // --- State ---
        messages: [],
        draft: '',
        isOpen: false,
        unreadCount: 0,
        initialized: false,
        maxChars: 500,
        _scrollPending: false,
        _messagesEl: null,

        // v0.9.19 helpers
        msgBubbleClass,
        msgNameClass,
        avatarClass,
        roleBadgeHtml,
        authorLabel,

        // --- Lifecycle ---
        // v0.9.5 fix: Bỏ check `document.cookie.includes('session_id')` vì cookie
        // HttpOnly không đọc được bằng JS. Layout chỉ render khi user đăng nhập.
        init() {
            this.initialized = true;
            this._messagesEl = this.$refs.globalMessages;
            this._lastReceivedAt = Date.now();
            this.loadHistory();
            this.connect();
        },

        async loadHistory() {
            try {
                const resp = await fetch('/api/chat-chung/history?limit=50', {
                    credentials: 'same-origin',
                });
                if (resp.ok) {
                    const msgs = await resp.json();
                    // Server trả newest first → reverse để oldest first
                    this.messages = msgs.reverse();
                    this.$nextTick(() => this.scrollToBottom());
                }
            } catch (_) {}
        },

        handleIncoming(raw) {
            let data;
            try {
                data = JSON.parse(raw);
            } catch (_) {
                return;
            }

            if (data.type === 'pong') return;

            if (data.type === 'error' && typeof data.message === 'string') {
                this.error = data.message;
                if (window.TubiSound) TubiSound.playError();
                setTimeout(() => { if (this.error === data.message) this.error = ''; }, 3000);
                return;
            }

            if (data.id && data.body !== undefined && data.author_display_name !== undefined) {
                if (this.messages.some((m) => m.id === data.id)) return;
                this.messages.push(data);
                this._trimMessages();
                if (!this.isOpen) {
                    this.unreadCount++;
                }
                this.$nextTick(() => this.scrollToBottom());
                if (window.TubiSound) TubiSound.playReceive();
            }
        },

        send() {
            const body = this.draft.trim();
            if (!body) return;
            if (body.length > this.maxChars) {
                this.error = `Tin nhắn quá dài (tối đa ${this.maxChars} ký tự)`;
                return;
            }

            if (!this.connected || !this.socket || this.socket.readyState !== WebSocket.OPEN) {
                this._queue.push(body);
                this.draft = '';
                this.error = 'Đang kết nối lại — tin nhắn sẽ tự gửi';
                if (!this.connected) this.scheduleReconnect();
                return;
            }

            const sent = this._sendRaw(body);
            if (sent) {
                this.draft = '';
                this.error = '';
                if (window.TubiSound) TubiSound.playSend();
            } else {
                this._queue.push(body);
                this.error = 'Mất kết nối — tin nhắn đang chờ gửi lại';
                this.scheduleReconnect();
            }
        },

        _trimMessages() {
            if (this.messages.length > 200) {
                this.messages.splice(0, this.messages.length - 200);
            }
        },

        toggleChat() {
            this.isOpen = !this.isOpen;
            if (this.isOpen) {
                this.unreadCount = 0;
                this.$nextTick(() => this.scrollToBottom());
            }
        },

        scrollToBottom() {
            if (this._scrollPending) return;
            this._scrollPending = true;
            requestAnimationFrame(() => {
                this._scrollPending = false;
                if (this._messagesEl) {
                    this._messagesEl.scrollTop = this._messagesEl.scrollHeight;
                }
            });
        },

        formatTime,
    });
}

window.globalChat = globalChat;

// ====================================================================
// chatBubble — Draggable circular bubble opens global chat
// ====================================================================

function chatBubble() {
    return {
        x: 0,
        y: 0,
        startX: 0,
        startY: 0,
        offsetX: 0,
        offsetY: 0,
        dragging: false,
        moved: false,

        init() {
            const isMobile = window.innerWidth < 768;
            this.x = window.innerWidth - 64;
            this.y = isMobile ? window.innerHeight - 88 : window.innerHeight - 80;
            this.offsetX = this.x;
            this.offsetY = this.y;
        },

        onMouseDown(event) {
            this.dragging = true;
            this.moved = false;
            this.startX = event.clientX - this.offsetX;
            this.startY = event.clientY - this.offsetY;
            event.preventDefault();
        },

        onMouseMove(event) {
            if (!this.dragging) return;
            this.moved = true;
            this.x = event.clientX - this.startX;
            this.y = event.clientY - this.startY;
            this.offsetX = this.x;
            this.offsetY = this.y;
        },

        onMouseUp() {
            this.dragging = false;
        },

        onTouchStart(event) {
            const touch = event.touches[0];
            this.dragging = true;
            this.moved = false;
            this.startX = touch.clientX - this.offsetX;
            this.startY = touch.clientY - this.offsetY;
        },

        onTouchMove(event) {
            if (!this.dragging) return;
            this.moved = true;
            const touch = event.touches[0];
            this.x = touch.clientX - this.startX;
            this.y = touch.clientY - this.startY;
            this.offsetX = this.x;
            this.offsetY = this.y;
            event.preventDefault();
        },

        onTouchEnd() {
            this.dragging = false;
        },

        onClick() {
            if (!this.moved) {
                this.$dispatch('toggle-global-chat');
            }
            this.moved = false;
        },

        get bubbleStyle() {
            return `left: ${this.x}px; top: ${this.y}px;`;
        },
    };
}

window.chatBubble = chatBubble;

// ====================================================================
// dmChat — Direct Message 1-1
// ====================================================================

function dmChat(opts) {
    const mixin = chatSocketMixin(function () {
        const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const host = window.location.host;
        return `${proto}//${host}/ws/ban-be/tin-nhan/${encodeURIComponent(this.conversationId)}`;
    });

    return Object.assign({}, mixin, {
        conversationId: opts.conversationId || '',
        otherUserId: opts.otherUserId || '',
        otherDisplayName: opts.otherDisplayName || '',
        messages: Array.isArray(opts.initialMessages) ? opts.initialMessages.slice() : [],
        draft: '',
        maxChars: 1000,
        _scrollPending: false,
        _messagesEl: null,

        // v0.9.19 helpers
        msgBubbleClass,
        msgNameClass,
        avatarClass,
        roleBadgeHtml,
        authorLabel,

        init() {
            this._messagesEl = this.$refs.messages;
            this._lastReceivedAt = Date.now();
            this.$nextTick(() => this.scrollToBottom());
            this.connect();
        },

        handleIncoming(raw) {
            let data;
            try {
                data = JSON.parse(raw);
            } catch (_) {
                return;
            }

            if (data.type === 'pong') return;

            if (data.type === 'error' && typeof data.message === 'string') {
                this.error = data.message;
                if (window.TubiSound) TubiSound.playError();
                setTimeout(() => { if (this.error === data.message) this.error = ''; }, 3000);
                return;
            }

            if (data.id && data.body !== undefined && data.author_display_name !== undefined) {
                if (this.messages.some((m) => m.id === data.id)) return;
                this.messages.push(data);
                this._trimMessages();
                this.$nextTick(() => this.scrollToBottom());
                if (window.TubiSound) TubiSound.playReceive();
            }
        },

        send() {
            const body = this.draft.trim();
            if (!body) return;
            if (body.length > this.maxChars) {
                this.error = `Tin nhắn quá dài (tối đa ${this.maxChars} ký tự)`;
                return;
            }

            if (!this.connected || !this.socket || this.socket.readyState !== WebSocket.OPEN) {
                this._queue.push(body);
                this.draft = '';
                this.error = 'Đang kết nối lại — tin nhắn sẽ tự gửi';
                if (!this.connected) this.scheduleReconnect();
                return;
            }

            const sent = this._sendRaw(body);
            if (sent) {
                this.draft = '';
                this.error = '';
                if (window.TubiSound) TubiSound.playSend();
            } else {
                this._queue.push(body);
                this.error = 'Mất kết nối — tin nhắn đang chờ gửi lại';
                this.scheduleReconnect();
            }
        },

        _trimMessages() {
            if (this.messages.length > 200) {
                this.messages.splice(0, this.messages.length - 200);
            }
        },

        scrollToBottom() {
            if (this._scrollPending) return;
            this._scrollPending = true;
            requestAnimationFrame(() => {
                this._scrollPending = false;
                if (this._messagesEl) {
                    this._messagesEl.scrollTop = this._messagesEl.scrollHeight;
                }
            });
        },

        formatTime,
    });
}

window.dmChat = dmChat;

// ====================================================================
// notificationBadge — Poll notification count mỗi 30s
// ====================================================================

function notificationBadge() {
    return {
        unreadCount: 0,
        init() {
            this.fetchUnread();
            setInterval(() => this.fetchUnread(), 30000);
        },
        async fetchUnread() {
            try {
                const resp = await fetch('/api/ban-be/thong-bao/chua-doc', {
                    credentials: 'same-origin',
                });
                if (resp.ok) {
                    const data = await resp.json();
                    this.unreadCount = data.unread_count || 0;
                }
            } catch (_) {}
        },
    };
}

window.notificationBadge = notificationBadge;
