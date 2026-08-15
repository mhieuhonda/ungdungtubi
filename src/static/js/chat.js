/**
 * Ứng Dụng Từ Bi — Chat Module (v0.9.21 — Giai đoạn 26)
 *
 * v0.9.21 thay đổi:
 *   - Xoá hoàn toàn liveChat() (group live chat) — chỉ giữ Chat Chung
 *   - Xoá tất cả TubiSound references (sound effects đã bị loại bỏ)
 *   - Xoá "đang kết nối..." message — không hiển thị trạng thái connecting
 *   - Fix CtrlMessage::Error → CtrlMessage::Text cho pong (backend fix)
 *
 * Components:
 *   - globalChat()      — Chat Chung toàn platform (draggable bubble popup)
 *   - dmChat(opts)      — Direct Message 1-1
 *   - chatBubble()      — Draggable bubble mở global chat
 *   - notificationBadge() — Poll notification count mỗi 30s
 */

// ====================================================================
// Shared helpers — dùng chung cho các loại chat
// ====================================================================

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
 * Mixin chứa WebSocket logic chung cho chat.
 * v0.9.21: Xoá tất cả TubiSound.playXxx() calls.
 */
function chatSocketMixin(getUrl) {
    return {
        // --- State ---
        connected: false,
        socket: null,
        error: '',
        reconnectAttempts: 0,
        maxReconnectAttempts: 10,
        reconnectTimer: null,
        _pingTimer: null,
        _healthTimer: null,
        _lastReceivedAt: 0,
        _queue: [],
        _pending: new Map(),

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
                return;
            }

            this.socket.onopen = () => {
                this.connected = true;
                this.error = '';
                this.reconnectAttempts = 0;
                this._lastReceivedAt = Date.now();

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

                if (event.code === 1000) return;

                if (event.code === 1008) {
                    this.error = event.reason || 'Không có quyền chat';
                    return;
                }

                this.scheduleReconnect();
            };

            this.socket.onerror = () => {
                this.connected = false;
            };
        },

        _startPing() {
            this._stopPing();
            this._pingTimer = setInterval(() => {
                if (this.socket && this.socket.readyState === WebSocket.OPEN) {
                    try {
                        this.socket.send('{"type":"ping"}');
                    } catch (_) {}
                }
            }, 30000);
        },

        _stopPing() {
            if (this._pingTimer) {
                clearInterval(this._pingTimer);
                this._pingTimer = null;
            }
        },

        _startHealthCheck() {
            this._stopHealthCheck();
            this._healthTimer = setInterval(() => {
                if (this._lastReceivedAt === 0) return;
                const elapsed = Date.now() - this._lastReceivedAt;
                if (elapsed > 60000) {
                    try { this.socket.close(4000, 'health check timeout'); } catch (_) {}
                    this.socket = null;
                    this.connected = false;
                    this.scheduleReconnect();
                }
            }, 15000);
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
            const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts - 1), 30000);
            // v0.9.21: Không hiển thị thông báo reconnect — tránh gây rối
            clearTimeout(this.reconnectTimer);
            this.reconnectTimer = setTimeout(() => this.connect(), delay);
        },

        _sendRaw(body) {
            if (!this.socket || this.socket.readyState !== WebSocket.OPEN) return false;
            try {
                this.socket.send(body);
                return true;
            } catch (_) {
                return false;
            }
        },

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
                // v0.9.21: Không hiển thị "đang kết nối lại" — tránh gây rối
                if (!this.connected) this.scheduleReconnect();
                return;
            }

            const sent = this._sendRaw(body);
            if (sent) {
                this.draft = '';
                this.error = '';
            } else {
                this._queue.push(body);
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
                setTimeout(() => { if (this.error === data.message) this.error = ''; }, 3000);
                return;
            }

            if (data.id && data.body !== undefined && data.author_display_name !== undefined) {
                if (this.messages.some((m) => m.id === data.id)) return;
                this.messages.push(data);
                this._trimMessages();
                this.$nextTick(() => this.scrollToBottom());
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
                if (!this.connected) this.scheduleReconnect();
                return;
            }

            const sent = this._sendRaw(body);
            if (sent) {
                this.draft = '';
                this.error = '';
            } else {
                this._queue.push(body);
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
