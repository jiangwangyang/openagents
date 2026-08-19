// ==========================================
// 核心模块：全局状态、生命周期初始化、公共工具与视图路由
// ==========================================

// ===== 1. 全局 DOM 节点缓存 =====
const viewDialog = document.getElementById('viewDialog');
const chatContainer = document.getElementById('chatContainer');
const messageInput = document.getElementById('messageInput');
const sendButton = document.getElementById('sendButton');
const conversationList = document.getElementById('conversationList');
const conversationInfo = document.getElementById('conversationInfo');
const usageInfo = document.getElementById('usageInfo');
const emptyState = document.getElementById('emptyState');
const globalInputWrapper = document.getElementById('globalInputWrapper');

// ===== 2. 运行时状态 =====
// 会话与流控状态
let currentConversationId = null;
let isTyping = false;
// 当前会话是否为任务/定时来源的只读会话（仅供查看，禁止发送消息）
let currentConvReadonly = false;
let currentEventSource = null;
let currentWorkdir = '';

// 流式渲染状态
let streamWrapper = null;
let streamContentNode = null;
let streamRawText = '';
let streamChunkCount = 0;
// token 用量累计（每次 connectStream 时重置）
let usageInputTokens = 0;
let usageOutputTokens = 0;
let usageCacheTokens = 0;
// 当次 usage 事件三项之和，表示本轮对话的总 token 量
let usageTotalTokens = 0;

// 会话滚动控制状态
let isAtBottom = true;
let userScroll = false;
let programScroll = false;

// ===== 3. SVG 图标与骨架屏资产 =====
const FOLD_SVG = `<svg viewBox="0 0 24 24" class="fold-icon" width="14" height="14" stroke="currentColor" stroke-width="2.5" fill="none" stroke-linecap="square" stroke-linejoin="miter"><polyline points="6 9 12 15 18 9"></polyline></svg>`;
const ARROW_SVG = `<svg viewBox="0 0 24 24" class="info-card-arrow" width="14" height="14" stroke="currentColor" stroke-width="2.5" fill="none"><polyline points="9 18 15 12 9 6"></polyline></svg>`;
const DELETE_SVG = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>`;
const SKELETON_HTML = '<div class="skeleton-loader"><span></span><span></span><span></span></div>';

// ===== 4. 视图路由配置 =====
// key 为视图名，load 为对应面板的数据加载函数名（调用时按名解析，避免加载顺序依赖）
const VIEW_CONFIG = {
    dialog: {nav: 'navDialogBtn', view: 'viewDialog', infoKey: null, load: null},
    task: {nav: 'navTaskBtn', view: 'viewTask', infoKey: 'header.coreTask', load: 'fetchTaskList', unload: 'cleanupTaskView'},
    cron: {nav: 'navCronBtn', view: 'viewCron', infoKey: 'header.coreCron', load: 'fetchCronTasks'},
    agent: {nav: 'navAgentBtn', view: 'viewAgent', infoKey: 'header.coreAgent', load: 'fetchAgentRegistry'},
    skill: {nav: 'navSkillBtn', view: 'viewSkill', infoKey: 'header.coreSkill', load: 'fetchSkillData'},
    mcp: {nav: 'navMcpBtn', view: 'viewMcp', infoKey: 'header.coreMcp', load: 'fetchMcpRegistry'},
    config: {nav: 'navConfigBtn', view: 'viewConfig', infoKey: 'header.coreConfig', load: 'fetchGlobalSettings'}
};

// ===== 5. 生命周期与监听初始化 =====
document.addEventListener('DOMContentLoaded', () => {
    // 初始化主题动态特效层
    initThemeEffects();

    // 初始化基础交互输入状态
    messageInput.disabled = false;
    sendButton.disabled = false;

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
    document.getElementById('manualPathInput').addEventListener('keydown', (e) => {
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

// ===== 6. 通用公共工具 =====
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
    return escapeHtml(decoded).replaceAll('\n', '<br>');
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

// 取对话最后一条消息的展示文本：msg.content 为 pi 消息 JSON，其内部 content 为字符串时直取，为 block 数组时取最后一个 block 的 text
function getLastMessageText(messages) {
    if (!messages || messages.length === 0) {
        return t('task.noMessages');
    }
    const piMessage = messages[messages.length - 1].content;
    const content = piMessage ? piMessage.content : null;
    if (typeof content === 'string') {
        return content;
    }
    if (Array.isArray(content) && content.length > 0) {
        return content[content.length - 1].text || '';
    }
    return '';
}

// 渲染执行记录项（任务阶段/定时执行记录通用）：执行中的 agent 对话高亮提示，点击打开覆盖式弹窗查看对话内容
function createStageRecordItem(conversation) {
    // agent 对话且无消息说明正在执行中，提示点击查看实时流式内容；用户对话由用户自己处理，不在执行
    const isRunning = conversation.agent_id != null && (!conversation.messages || conversation.messages.length === 0);
    const snippet = isRunning ? t('task.generating') : getLastMessageText(conversation.messages);
    const item = document.createElement('div');
    item.className = 'task-stage-item';
    item.id = `stage-item-${conversation.id}`;
    item.innerHTML = `
        <div class="task-stage-title">
            <span>${escapeHtml(conversation.title)}</span>
            <span class="stage-time">${escapeHtml(conversation.update_time || '')}</span>
        </div>
        <div class="task-stage-snippet${isRunning ? ' stage-running' : ''}">${escapeHtml(snippet)}</div>
    `;
    // 点击打开覆盖式弹窗展示阶段对话，避免跳转对话页丢失任务上下文
    item.onclick = () => showStageDialog(conversation);
    return item;
}

// ===== 阶段对话列表排序（任务/定时面板共用） =====
// 排序方向存储键（后端 Web 存储持久化记忆）
const STAGE_SORT_KEY = 'openagents_stage_sort';
// 当前排序方向：asc 升序（默认）/ desc 降序
let stageSortOrder = 'asc';

// 按当前排序方向返回对话列表副本（后端按 id 升序返回，降序时反转）
function sortedStageConversations(conversations) {
    const list = (conversations || []).slice();
    if (stageSortOrder === 'desc') {
        list.reverse();
    }
    return list;
}

// 创建排序按钮：显示当前排序方向，点击切换升/降序
function createStageSortButton() {
    const button = document.createElement('button');
    button.className = 'btn btn-sm btn-secondary btn-card-xs stage-sort-btn';
    button.textContent = t(stageSortOrder === 'asc' ? 'common.sortAsc' : 'common.sortDesc');
    button.title = t('common.sortToggle');
    button.onclick = toggleStageSortOrder;
    return button;
}

// 切换排序方向：持久化到后端存储，同步全部按钮文本并重渲染两个面板中已展开的列表
function toggleStageSortOrder() {
    stageSortOrder = stageSortOrder === 'asc' ? 'desc' : 'asc';
    setWebStorage(STAGE_SORT_KEY, stageSortOrder);
    document.querySelectorAll('.stage-sort-btn').forEach(button => {
        button.textContent = t(stageSortOrder === 'asc' ? 'common.sortAsc' : 'common.sortDesc');
    });
    // 重渲染任务面板已展开卡片的阶段列表
    Object.keys(taskControllers).forEach(taskId => {
        if (taskControllers[taskId].expanded) {
            renderTaskStages(parseInt(taskId));
        }
    });
    // 重渲染定时面板已展开卡片的执行记录
    document.querySelectorAll('[id^="cron-stages-"]').forEach(stageList => {
        if (stageList.closest('.info-card').hasAttribute('open')) {
            loadCronDetail(parseInt(stageList.id.replace('cron-stages-', '')));
        }
    });
}

// 摘要展开状态：true 时列表完整显示对话内容（仅运行时状态，不持久化记忆）
let stageSnippetExpanded = false;

// 创建摘要展开按钮：点击切换 3 行截断 / 完整显示
function createStageExpandButton() {
    const button = document.createElement('button');
    button.className = 'btn btn-sm btn-secondary btn-card-xs stage-expand-btn';
    button.textContent = t(stageSnippetExpanded ? 'common.collapse' : 'common.expand');
    button.onclick = toggleStageSnippetExpanded;
    return button;
}

// 切换摘要展开状态：同步全部按钮文本并切换所有列表容器的展开类
function toggleStageSnippetExpanded() {
    stageSnippetExpanded = !stageSnippetExpanded;
    document.querySelectorAll('.stage-expand-btn').forEach(button => {
        button.textContent = t(stageSnippetExpanded ? 'common.collapse' : 'common.expand');
    });
    document.querySelectorAll('.task-stage-list').forEach(list => {
        list.classList.toggle('snippet-expanded', stageSnippetExpanded);
    });
}

// 初始化：从后端存储恢复排序方向记忆
(function initStageSortOrder() {
    getWebStorage(STAGE_SORT_KEY).then(savedOrder => {
        if (savedOrder === 'asc' || savedOrder === 'desc') {
            stageSortOrder = savedOrder;
        }
    });
})();

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

// 通用确认弹窗：危险操作二次确认，确认后执行 onConfirm 回调
function showConfirmDialog({title, text, onConfirm}) {
    // 先清理已有弹窗，避免叠加
    closeConfirmDialog();

    // 内容超长时截断并追加省略号，避免弹窗高度超出屏幕导致按钮无法点击
    const displayText = text.length > 100 ? text.slice(0, 100) + '...' : text;

    const overlay = document.createElement('div');
    overlay.className = 'confirm-overlay';
    overlay.style.display = 'flex';
    overlay.innerHTML = `
        <div class="confirm-dialog">
            <div class="confirm-title">${escapeHtml(title)}</div>
            <div class="confirm-text">${escapeHtml(displayText)}</div>
            <div class="confirm-buttons">
                <button class="confirm-btn cancel" onclick="closeConfirmDialog()">${t('common.abort')}</button>
                <button class="confirm-btn danger" id="globalConfirmExecuteBtn">${t('common.purge')}</button>
            </div>
        </div>
    `;
    document.body.appendChild(overlay);

    document.getElementById('globalConfirmExecuteBtn').onclick = () => {
        if (typeof onConfirm === 'function') {
            onConfirm();
        }
        closeConfirmDialog();
    };

    // 点击遮罩层关闭弹窗
    overlay.addEventListener('click', (e) => {
        if (e.target === overlay) {
            closeConfirmDialog();
        }
    });
}

function closeConfirmDialog() {
    const overlay = document.querySelector('.confirm-overlay');
    if (overlay) {
        overlay.remove();
    }
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

// ===== 7. 视图路由导航 =====
// 当前激活视图名：离开视图时按配置调用清理钩子
let currentViewName = 'dialog';

function switchView(viewName) {
    const cfg = VIEW_CONFIG[viewName];
    if (!cfg) {
        return;
    }
    // 离开旧视图前执行清理钩子（任务面板借此停止轮询与 SSE 跟随）
    if (currentViewName !== viewName) {
        const prevCfg = VIEW_CONFIG[currentViewName];
        if (prevCfg && prevCfg.unload && typeof window[prevCfg.unload] === 'function') {
            window[prevCfg.unload]();
        }
    }
    currentViewName = viewName;
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

// ===== 8. 阶段对话弹窗与消息渲染（任务/定时面板共用） =====
// 当前打开的阶段弹窗状态：持有弹窗独立的 SSE 连接与流式渲染器
let stageDialogState = null;

// 流式渲染器工厂：handleChunk 处理 SSE chunk 并往容器追加块，delta 合并入当前块；渲染规则与对话页一致
function createStreamRenderer(container) {
    let wrapper = null;
    let contentNode = null;
    let rawText = '';
    // 结束当前块：移除流式光标并折叠详情块
    const finalize = () => {
        if (contentNode) {
            contentNode.classList.remove('streaming-active');
            const prevDetails = contentNode.closest('details');
            if (prevDetails) {
                prevDetails.open = false;
            }
            contentNode = null;
        }
    };
    const handleChunk = (data) => {
        if (!data || !data.type) {
            return;
        }
        // 系统提示词：可折叠块，展示在消息流开头
        if (data.type === 'system') {
            if (data.text) {
                const details = document.createElement('details');
                details.className = 'system-details';
                details.innerHTML = `<summary>${FOLD_SVG} ${t('stream.systemPrompt')}</summary><div class="content"></div>`;
                details.querySelector('.content').innerHTML = formatMarkdown(data.text);
                container.appendChild(details);
            }
            return;
        }
        // 错误/手动停止提示条
        if (data.type === 'error' || data.type === 'stopped') {
            finalize();
            wrapper = null;
            const div = document.createElement('div');
            div.className = data.type === 'error' ? 'user-message stream-error' : 'user-message stream-stopped';
            div.textContent = data.type === 'error' ? `⚠ ${data.text || t('stream.unknownError')}` : t('stream.stopped');
            container.appendChild(div);
            return;
        }
        // 用户消息气泡
        if (data.type === 'user') {
            finalize();
            wrapper = null;
            const div = document.createElement('div');
            div.className = 'user-message';
            div.innerHTML = `${formatMarkdown((data.text || '').trim())}<div class="message-time"></div>`;
            container.appendChild(div);
            return;
        }
        // 助手消息块：thinking / text / tool_use / tool_result
        if (data.type === 'thinking' || data.type === 'text' || data.type === 'tool_use' || data.type === 'tool_result') {
            finalize();
            if (!wrapper) {
                wrapper = document.createElement('div');
                wrapper.className = 'assistant-message';
                container.appendChild(wrapper);
            }
            rawText = data.text || '';
            if (data.type === 'thinking') {
                const details = document.createElement('details');
                details.className = 'think-details';
                details.open = true;
                details.innerHTML = `<summary>${FOLD_SVG} ${t('stream.thoughtProcess')}</summary><div class="content streaming-active"></div>`;
                wrapper.appendChild(details);
                contentNode = details.querySelector('.content');
            } else if (data.type === 'text') {
                const div = document.createElement('div');
                div.className = 'reply-content streaming-active';
                wrapper.appendChild(div);
                contentNode = div;
            } else if (data.type === 'tool_use') {
                const details = document.createElement('details');
                details.className = 'tool-details';
                details.open = true;
                details.innerHTML = `<summary>${FOLD_SVG} ${t('stream.callPrefix')}: ${escapeHtml(data.name || t('stream.tool'))}</summary><div class="content streaming-active"></div>`;
                wrapper.appendChild(details);
                contentNode = details.querySelector('.content');
            } else {
                const details = document.createElement('details');
                details.className = 'tool-details';
                const status = data.is_error ? t('stream.toolError') : t('stream.toolResult');
                details.innerHTML = `<summary>${FOLD_SVG} ${t('stream.tool')} ${status} [${escapeHtml(String(data.id || ''))}]</summary><div class="content streaming-active"></div>`;
                wrapper.appendChild(details);
                contentNode = details.querySelector('.content');
            }
            if (rawText && contentNode) {
                contentNode.innerHTML = formatMarkdown(rawText);
            }
            container.scrollTop = container.scrollHeight;
            return;
        }
        // 追加消息文本
        if (data.type === 'delta') {
            rawText += data.text || '';
            if (contentNode) {
                contentNode.innerHTML = formatMarkdown(rawText);
            }
            container.scrollTop = container.scrollHeight;
        }
    };
    return {handleChunk, finalize};
}

// 打开阶段对话弹窗：遮罩+窗口卡片覆盖页面，右上角关闭、页脚提供对话页入口；弹窗独立建立 SSE 连接回放历史并实时跟随
function showStageDialog(conversation) {
    closeStageDialog();
    const overlay = document.createElement('div');
    overlay.className = 'confirm-overlay stage-dialog-overlay';
    overlay.style.display = 'flex';
    overlay.innerHTML = `
        <div class="dir-modal stage-dialog">
            <div class="dir-modal-header">
                <div class="confirm-title">${escapeHtml(conversation.title)}</div>
                <button class="delete-btn always-visible" id="stageDialogCloseBtn">${DELETE_SVG}</button>
            </div>
            <div class="dir-modal-content stage-dialog-body" id="stageDialogBody"></div>
            <div class="dir-modal-footer stage-dialog-footer">
                <button class="stage-dialog-link" id="stageDialogOpenLink">${t('task.openInDialog')}</button>
            </div>
        </div>
    `;
    document.body.appendChild(overlay);
    const body = document.getElementById('stageDialogBody');
    // 弹窗独立建立 SSE 连接：服务端先回放全部历史 chunks 再实时跟随新数据，不依赖任务跟随链的既有流
    const renderer = createStreamRenderer(body);
    const source = new EventSource(`/conversation/${conversation.id}/stream`);
    stageDialogState = {conversationId: conversation.id, renderer: renderer, eventSource: source};
    source.onmessage = (event) => {
        renderer.handleChunk(JSON.parse(event.data));
    };
    source.onerror = () => {
        // 流结束（回放完毕或对话完成）：关闭连接阻止浏览器自动重连，并收尾当前流式块
        source.close();
        if (stageDialogState && stageDialogState.eventSource === source) {
            stageDialogState.eventSource = null;
        }
        renderer.finalize();
    };
    document.getElementById('stageDialogCloseBtn').onclick = closeStageDialog;
    // 次级入口：跳转对话页完整回放（只读）
    document.getElementById('stageDialogOpenLink').onclick = () => {
        closeStageDialog();
        switchView('dialog');
        loadConversation(conversation.id, true);
    };
    // 点击遮罩层关闭弹窗
    overlay.addEventListener('click', (e) => {
        if (e.target === overlay) {
            closeStageDialog();
        }
    });
}

function closeStageDialog() {
    const overlay = document.querySelector('.stage-dialog-overlay');
    if (overlay) {
        overlay.remove();
    }
    // 关闭弹窗独立的 SSE 连接，避免后台持续占用流
    if (stageDialogState && stageDialogState.eventSource) {
        stageDialogState.eventSource.close();
    }
    stageDialogState = null;
}