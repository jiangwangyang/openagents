// ==========================================
// 1. 全局 DOM 节点缓存与核心状态管理
// ==========================================
const viewDialog = document.getElementById('viewDialog');
const chatContainer = document.getElementById('chatContainer');
const messageInput = document.getElementById('messageInput');
const sendButton = document.getElementById('sendButton');
const conversationList = document.getElementById('conversationList');
const conversationInfo = document.getElementById('conversationInfo');
const usageInfo = document.getElementById('usageInfo');
const emptyState = document.getElementById('emptyState');
const manualPathInput = document.getElementById('manualPathInput');

const globalInputWrapper = document.getElementById('globalInputWrapper');

// 初始化基础交互输入状态
messageInput.disabled = false;
sendButton.disabled = false;

// 运行时会话/流控状态变量
let currentConversationId = null;
let isTyping = false;
// 当前会话是否为任务/定时来源的只读会话（仅供查看，禁止发送消息）
let currentConvReadonly = false;
let currentEventSource = null;
let currentWorkdir = '';

// 流式渲染状态变量
let streamWrapper = null;
let streamContentNode = null;
let streamRawText = '';
let streamChunkCount = 0;
// token 用量累计（每次 connectStream 时重置）
let usageInputTokens = 0;
let usageOutputTokens = 0;
let usageCacheTokens = 0;
// 最后一次 usage 事件三项之和，表示当前对话的总 token 量
let usageTotalTokens = 0;

// 运行时会话滚动控制变量
let isAtBottom = true;
let userScroll = false;
let programScroll = false;

// SVG 图标核心资产定义
const FOLD_SVG = `<svg viewBox="0 0 24 24" class="fold-icon" width="14" height="14" stroke="currentColor" stroke-width="2.5" fill="none" stroke-linecap="square" stroke-linejoin="miter"><polyline points="6 9 12 15 18 9"></polyline></svg>`;
const ARROW_SVG = `<svg viewBox="0 0 24 24" class="info-card-arrow" width="14" height="14" stroke="currentColor" stroke-width="2.5" fill="none"><polyline points="9 18 15 12 9 6"></polyline></svg>`;
const DELETE_SVG = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>`;
const SKELETON_HTML = '<div class="skeleton-loader"><span></span><span></span><span></span></div>';

// 视图路由配置表：key 为视图名，load 为对应面板的数据加载函数名（调用时按名解析，避免加载顺序依赖）
const VIEW_CONFIG = {
    dialog: {nav: 'navDialogBtn', view: 'viewDialog', infoKey: null, load: null},
    task: {nav: 'navTaskBtn', view: 'viewTask', infoKey: 'header.coreTask', load: 'fetchTaskList'},
    cron: {nav: 'navCronBtn', view: 'viewCron', infoKey: 'header.coreCron', load: 'fetchCronTasks'},
    agent: {nav: 'navAgentBtn', view: 'viewAgent', infoKey: 'header.coreAgent', load: 'fetchAgentRegistry'},
    skill: {nav: 'navSkillBtn', view: 'viewSkill', infoKey: 'header.coreSkill', load: 'fetchSkillData'},
    mcp: {nav: 'navMcpBtn', view: 'viewMcp', infoKey: 'header.coreMcp', load: 'fetchMcpRegistry'},
    config: {nav: 'navConfigBtn', view: 'viewConfig', infoKey: 'header.coreConfig', load: 'fetchGlobalSettings'}
};

// ==========================================
// 2. 生命周期与核心监听初始化
// ==========================================
document.addEventListener('DOMContentLoaded', () => {
    // 初始化主题动态特效层
    initThemeEffects();

    // 自动缩放用户输入框
    messageInput.addEventListener('input', autoResize);

    // 监听用户发送消息
    messageInput.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' && !e.shiftKey) {
            e.preventDefault();
            sendMessage();
        }
    });

    // 监听用户选择目录
    manualPathInput.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') {
            handleManualJump();
        }
    });

    // 监听页面滚动
    viewDialog.addEventListener('scroll', () => {
        if (programScroll) {
            return;
        }
        const currentIsAtBottom = viewDialog.scrollHeight - viewDialog.scrollTop - viewDialog.clientHeight < 20;
        if (isAtBottom && !currentIsAtBottom) {
            userScroll = true;
        } else if (!isAtBottom && currentIsAtBottom) {
            userScroll = false;
        }
        isAtBottom = currentIsAtBottom;
    });

    // 监听页面滚动到底部
    viewDialog.addEventListener('scrollend', () => {
        programScroll = false;
    });

    // 模型输入框：聚焦时拉取供应商模型列表全量展示，失焦延迟关闭（留时间给点击事件）
    const modelInput = document.getElementById('modelSelect');
    modelInput.addEventListener('focus', renderModelComboList);
    modelInput.addEventListener('blur', () => {
        setTimeout(() => document.getElementById('modelComboList').classList.remove('open'), 150);
    });

    // 加载历史会话并进入新会话页面
    loadConversationList();
    startNewChat();
});

// ==========================================
// 3. 通用公共核心排版与基础工具引擎
// ==========================================
// textContent 转义仅覆盖 & < >，需额外转义引号以兼容 value="..." 等属性插值场景
function escapeHtml(text) {
    if (!text) {
        return '';
    }
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML.replaceAll('"', '&quot;').replaceAll("'", '&#39;');
}

function formatMarkdown(text) {
    // 解码工具 JSON 等内容中字面量的 Unicode 转义序列（如 \u4e2d\u6587 -> 中文），需在 HTML 转义前执行
    const decoded = text.replace(/\\u([0-9a-fA-F]{4})/g, (match, hex) => String.fromCharCode(parseInt(hex, 16)));
    return escapeHtml(decoded).replaceAll("\n", "<br>");
}

// 各面板列表通用空态/错误态排版，textKey 为 i18n 文案 key，hintKey 为可选的功能引导说明 key
function emptyListHtml(textKey, hintKey) {
    const hintHtml = hintKey ? `<div class="list-empty-hint">${t(hintKey)}</div>` : '';
    return `<div class="list-empty"><div>${t(textKey)}</div>${hintHtml}</div>`;
}

function errorListHtml(textKey) {
    return `<div class="list-error">${t(textKey)}</div>`;
}

function scrollToBottom() {
    isAtBottom = true;
    userScroll = false;
    programScroll = true;
    viewDialog.scrollTop = viewDialog.scrollHeight;
}

function scrollToBottomIfNotUserScroll() {
    if (!userScroll) {
        programScroll = true;
        viewDialog.scrollTop = viewDialog.scrollHeight;
        isAtBottom = true;
    }
}

// 填充下拉框选项：items 为含 id/name 的列表，selectedId 非空时选中匹配项
function fillSelectOptions(select, items, selectedId) {
    select.innerHTML = '';
    items.forEach(item => {
        const opt = document.createElement('option');
        opt.value = item.id;
        opt.textContent = item.name;
        if (selectedId != null && String(item.id) === String(selectedId)) {
            opt.selected = true;
        }
        select.appendChild(opt);
    });
}

// 取对话最后一条消息的展示文本：content 为数组时取最后一个 block 的 text
function getLastMessageText(messages) {
    if (!messages || messages.length === 0) {
        return t('task.noMessages');
    }
    const content = messages[messages.length - 1].content;
    if (typeof content === 'string') {
        return content;
    }
    if (Array.isArray(content) && content.length > 0) {
        return content[content.length - 1].text || '';
    }
    return '';
}

// 渲染执行记录项（任务阶段/定时执行记录通用）：执行中的 agent 对话高亮提示，点击进入对话页只读流式回放
function createStageRecordItem(conversation) {
    // agent 对话且无消息说明正在执行中，提示点击查看实时流式内容；用户对话由用户自己处理，不在执行
    const isRunning = conversation.agent_id != null && (!conversation.messages || conversation.messages.length === 0);
    const snippet = isRunning ? t('task.generating') : getLastMessageText(conversation.messages);
    const item = document.createElement('div');
    item.className = 'task-stage-item';
    item.innerHTML = `
        <div class="task-stage-title">
            <span>${escapeHtml(conversation.title)}</span>
            <span class="stage-time">${escapeHtml(conversation.update_time || '')}</span>
        </div>
        <div class="task-stage-snippet${isRunning ? ' stage-running' : ''}">${escapeHtml(snippet)}</div>
    `;
    item.onclick = () => {
        switchView('dialog');
        loadConversation(conversation.id, true);
    };
    return item;
}

function toggleCardOpen(cardElement) {
    const details = cardElement.querySelector('.info-card-details');
    if (cardElement.hasAttribute('open')) {
        cardElement.removeAttribute('open');
        details.style.display = 'none';
    } else {
        cardElement.setAttribute('open', '');
        details.style.display = 'block';
    }
}

// ----- 通用高级确认弹窗二次沉淀引擎 -----
function showConfirmDialog({title, text, onConfirm}) {
    // 先行做边界清理安全机制
    closeConfirmDialog();

    const overlay = document.createElement('div');
    overlay.className = 'confirm-overlay';
    overlay.style.display = 'flex';
    overlay.innerHTML = `
        <div class="confirm-dialog">
            <div class="confirm-title">${escapeHtml(title)}</div>
            <div class="confirm-text">${escapeHtml(text)}</div>
            <div class="confirm-buttons">
                <button class="confirm-btn cancel" onclick="closeConfirmDialog()">${t('common.abort')}</button>
                <button class="confirm-btn danger" id="globalConfirmExecuteBtn">${t('common.purge')}</button>
            </div>
        </div>
    `;
    document.body.appendChild(overlay);

    // 绑定动作触发回调函数
    document.getElementById('globalConfirmExecuteBtn').onclick = () => {
        if (typeof onConfirm === 'function') {
            onConfirm();
        }
        closeConfirmDialog();
    };

    // 点击背景遮罩层优雅退场
    overlay.addEventListener('click', (e) => {
        if (e.target === overlay) {
            closeConfirmDialog();
        }
    });
}

// 轻量提示：顶部居中短暂展示后自动消失，避免 alert 阻断操作；type 为 'error' 时使用错误配色
function showToast(text, type) {
    const toast = document.createElement('div');
    toast.className = type === 'error' ? 'app-toast error' : 'app-toast';
    toast.textContent = text;
    document.body.appendChild(toast);
    // 下一帧加 visible 类触发淡入过渡
    requestAnimationFrame(() => toast.classList.add('visible'));
    setTimeout(() => {
        toast.classList.remove('visible');
        setTimeout(() => toast.remove(), 300);
    }, 2200);
}

function closeConfirmDialog() {
    const overlay = document.querySelector('.confirm-overlay');
    if (overlay) {
        overlay.remove();
    }
}

// ==========================================
// 4. 核心骨干控制：视图路由导航引擎
// ==========================================
function switchView(viewName) {
    const cfg = VIEW_CONFIG[viewName];
    if (!cfg) {
        return;
    }
    document.querySelectorAll('.header-nav-btn').forEach(btn => btn.classList.remove('active'));
    document.querySelectorAll('.view-container').forEach(view => view.classList.remove('active'));
    document.getElementById(cfg.nav).classList.add('active');
    document.getElementById(cfg.view).classList.add('active');

    if (viewName === 'dialog') {
        globalInputWrapper.style.display = 'block';
        conversationInfo.textContent = currentConversationId ? `ID: ${currentConversationId}` : t('header.newTrace');
    } else {
        globalInputWrapper.style.display = 'none';
        conversationInfo.textContent = t(cfg.infoKey);
        window[cfg.load]();
    }
}

