// ==========================================
// 会话模块：会话历史、模型路由与 SSE 流式交互 (DIALOG)
// ==========================================

// ===== 1. 会话状态与持久化缓存 =====
// 当前已加载会话的系统提示词来源状态（loadConversation 时赋值，startNewChat 时清空）
let currentSystemPrompt = '';
let currentConvAgentName = '';

// 上次模型配置与智能体选择记忆（后端 Web 存储，仅记最近一次并自动填入）
const LAST_MODEL_CONFIG_KEY = 'openagents_last_model_config';
const LAST_AGENT_KEY = 'openagents_last_agent_id';

// 内存缓存：页面加载时从后端拉取，变更时整体写回
let lastModelConfigCache = null;
let lastAgentIdCache = '';
// 供应商模型列表缓存：key 为供应商 id，值为 Promise（并发去重），避免聚焦时重复请求
const providerModelsCache = {};

// ===== 2. 提示词来源提示 =====
// 已建会话按快照判断（智能体 > 工作目录 AGENTS.md > 无），新会话按当前控件状态预判
function updatePromptSourceHint() {
    const hint = document.getElementById('promptSourceHint');
    let text;
    let title;
    if (currentConversationId) {
        if (currentConvAgentName) {
            text = t('input.promptSourceAgent', {name: currentConvAgentName});
            title = text;
        } else if (currentSystemPrompt) {
            // 无智能体且提示词非空，说明创建时读取到了工作目录下的 AGENTS.md
            const path = `${currentWorkdir}/AGENTS.md`;
            text = t('input.promptSourceFile', {path: path});
            title = path;
        } else {
            text = t('input.promptSourceNone');
            title = text;
        }
    } else {
        const agentSelect = document.getElementById('agentSelect');
        if (agentSelect.value) {
            const selectedOption = agentSelect.options[agentSelect.selectedIndex];
            const name = selectedOption ? selectedOption.textContent : agentSelect.value;
            text = t('input.promptSourceAgent', {name: name});
            title = text;
        } else if (currentWorkdir) {
            // 未选智能体的新会话，提示词将在启动时从工作目录 AGENTS.md 读取
            const path = `${currentWorkdir}/AGENTS.md`;
            text = t('input.promptSourcePending', {path: path});
            title = path;
        } else {
            text = t('input.promptSourceNone');
            title = text;
        }
    }
    hint.textContent = text;
    hint.title = title;
    hint.style.display = '';
}

// ===== 3. 输入区状态控制 =====
function autoResize() {
    messageInput.style.height = 'auto';
    messageInput.style.height = Math.min(messageInput.scrollHeight, 160) + 'px';
    // 同步刷新字数计数（输入、清空等所有路径都会经过本函数）
    const charCounter = document.getElementById('inputCharCount');
    if (charCounter) {
        charCounter.textContent = `${messageInput.value.length}/${messageInput.maxLength}`;
        // 达到上限时计数器标红；maxlength 会静默截断粘贴内容，需显式提示用户
        const atLimit = messageInput.value.length >= messageInput.maxLength;
        charCounter.classList.toggle('at-limit', atLimit);
        if (atLimit) {
            showToast(t('input.charLimit', {max: messageInput.maxLength}), 'error');
        }
    }
}

function enableInput() {
    messageInput.disabled = false;
    sendButton.disabled = false;
    messageInput.focus();
}

function setTyping(typing) {
    isTyping = typing;
    // 流式期间发送按钮切换为停止按钮（含只读会话），非流式时恢复发送；只读会话非流式时禁用发送
    sendButton.textContent = typing ? t('input.stop') : t('input.execute');
    sendButton.disabled = currentConvReadonly && !typing;
    messageInput.disabled = typing || currentConvReadonly;
}

// 只读会话切换：任务/定时来源的对话仅供查看，禁用输入框与发送按钮；恢复启用时需尊重流式输出中的禁用状态
function setConversationReadonly(readonly) {
    currentConvReadonly = readonly;
    messageInput.disabled = readonly || isTyping;
    // 只读会话仅流式期间允许点击（此时按钮为停止），与 setTyping 的按钮禁用逻辑保持一致
    sendButton.disabled = readonly && !isTyping;
    messageInput.placeholder = readonly ? t('input.readonlyPlaceholder') : t('input.placeholder');
}

// 锁定/解锁会话上下文：对话创建后工作目录与智能体不允许修改，锁定期间展示说明文字避免误解
function setContextLocked(locked) {
    const workspaceBtn = document.getElementById('workspaceBtn');
    const agentSelect = document.getElementById('agentSelect');
    // 禁用态视觉统一由 CSS :disabled 规则（透明度 0.4 + 禁止光标）承担
    workspaceBtn.disabled = locked;
    agentSelect.disabled = locked;
    document.getElementById('contextLockHint').style.display = locked ? '' : 'none';
}

// ===== 4. 会话列表与会话加载 =====
async function loadConversationList() {
    try {
        const response = await fetch('/conversation/list');
        const conversations = await response.json();
        conversationList.innerHTML = '';
        conversations.forEach(conversation => {
            const item = document.createElement('div');
            item.className = 'conversation-item';
            item.dataset.id = conversation.id;
            item.innerHTML = `
                <span class="conversation-item-text">${escapeHtml(conversation.title)}</span>
                <button class="delete-btn">${DELETE_SVG}</button>
            `;
            // 删除按钮通过闭包绑定，避免标题中的引号破坏内联 onclick 字符串
            const deleteBtn = item.querySelector('.delete-btn');
            deleteBtn.title = t('common.purge');
            deleteBtn.onclick = (event) => {
                event.stopPropagation();
                confirmDeleteConversation(conversation.id, conversation.title.replaceAll('\n', ''));
            };
            item.onclick = () => {
                switchView('dialog');
                loadConversation(conversation.id);
            };
            conversationList.appendChild(item);
        });
        if (currentConversationId) {
            const items = conversationList.querySelectorAll('.conversation-item');
            items.forEach(item => item.classList.toggle('active', String(item.dataset.id) === String(currentConversationId)));
        }
    } catch (e) {
        // 静默处理错误
    }
}

// 加载指定对话：切换当前会话并从对话详情接口同步工作目录与模型路由配置；readonly 为 true 时（任务/定时来源）禁止发送消息
async function loadConversation(conversationId, readonly = false) {
    currentConversationId = conversationId;
    currentConvReadonly = readonly;
    // 对话已创建，锁定工作目录与智能体选择
    setContextLocked(true);
    // 从对话详情接口获取工作目录与配置（智能体/模型提供方/模型/是否思考）
    try {
        const response = await fetch(`/conversation/${conversationId}`);
        if (response.ok) {
            const conversation = await response.json();
            // 缓存系统提示词来源状态，供来源提示展示
            currentSystemPrompt = conversation.system_prompt || '';
            currentConvAgentName = (conversation.agent && conversation.agent.id != null) ? (conversation.agent.name || String(conversation.agent.id)) : '';
            updateWorkspaceUI(conversation.work_dir);
            const agentSelect = document.getElementById('agentSelect');
            const providerSelect = document.getElementById('providerSelect');
            const modelInput = document.getElementById('modelSelect');
            const thinkingSelect = document.getElementById('thinkingSelect');
            const agent = conversation.agent;
            if (agent && agent.id != null) {
                // 智能体下拉若不存在该选项则补齐
                if (!agentSelect.querySelector(`option[value="${agent.id}"]`)) {
                    const opt = document.createElement('option');
                    opt.value = agent.id;
                    opt.textContent = agent.name || String(agent.id);
                    agentSelect.appendChild(opt);
                }
                agentSelect.value = String(agent.id);
                // 模型提供方下拉若不存在该选项则补齐
                if (agent.model_provider_id != null && !providerSelect.querySelector(`option[value="${agent.model_provider_id}"]`)) {
                    const opt = document.createElement('option');
                    opt.value = agent.model_provider_id;
                    opt.textContent = (agent.model_provider && agent.model_provider.name) || String(agent.model_provider_id);
                    providerSelect.appendChild(opt);
                }
                if (agent.model_provider_id != null) {
                    providerSelect.value = String(agent.model_provider_id);
                }
                if (agent.model) {
                    modelInput.value = agent.model;
                }
                if (agent.thinking != null) {
                    thinkingSelect.value = String(agent.thinking);
                }
                // 智能体执行对话的模型配置固定，锁定不可修改
                providerSelect.disabled = true;
                modelInput.disabled = true;
                thinkingSelect.disabled = true;
            } else {
                // 用户对话：清空智能体选择并解锁模型路由控件
                agentSelect.value = '';
                providerSelect.disabled = false;
                modelInput.disabled = false;
                thinkingSelect.disabled = false;
            }
        }
    } catch (e) {
        // 静默处理错误
    }
    updatePromptSourceHint();
    chatContainer.innerHTML = '';
    emptyState.style.display = 'none';
    messageInput.value = '';
    autoResize();
    conversationInfo.textContent = `ID: ${conversationId}`;

    const items = conversationList.querySelectorAll('.conversation-item');
    items.forEach(item => item.classList.toggle('active', String(item.dataset.id) === String(conversationId)));

    // 应用只读状态：任务/定时来源的会话禁用输入框与发送按钮
    setConversationReadonly(readonly);
    // 通过对话流式接口回放历史消息并实时跟随
    connectStream(conversationId);
}

function confirmDeleteConversation(conversationId, convTitle) {
    showConfirmDialog({
        title: t('stream.purgeTitle'),
        text: t('stream.purgeText', {name: convTitle}),
        onConfirm: async () => {
            try {
                await fetch(`/conversation/${conversationId}`, {method: 'DELETE'});
                if (String(currentConversationId) === String(conversationId)) {
                    startNewChat();
                }
                await loadConversationList();
            } catch (e) {
                // 静默处理错误
            }
        }
    });
}

async function startNewChat() {
    currentConversationId = null;
    // 清空已加载会话的提示词来源状态，恢复为新会话预判模式
    currentSystemPrompt = '';
    currentConvAgentName = '';
    // 新会话恢复可输入状态（清除任务/定时来源的只读标记与占位文案）
    setConversationReadonly(false);
    // 取消历史列表中所有条目的选中高亮
    conversationList.querySelectorAll('.conversation-item').forEach(item => item.classList.remove('active'));
    chatContainer.innerHTML = '';
    chatContainer.appendChild(emptyState);
    emptyState.style.display = 'flex';
    messageInput.value = '';
    autoResize();
    enableInput();
    conversationInfo.textContent = t('header.newTrace');
    usageInfo.textContent = '';
    initDefaultWorkspace();
    // 先加载后端持久化偏好，再恢复智能体与模型配置
    await loadDialogPrefs();
    loadAgentSelect();
    loadModelSelect();
    setContextLocked(false);
    updatePromptSourceHint();
    switchView('dialog');
}

// ===== 5. 模型路由偏好（后端 Web 存储持久化） =====
// 从后端加载会话页持久化偏好到内存缓存（上次模型配置/上次智能体）
async function loadDialogPrefs() {
    const [configRaw, agentIdRaw] = await Promise.all([
        getWebStorage(LAST_MODEL_CONFIG_KEY),
        getWebStorage(LAST_AGENT_KEY)
    ]);
    try {
        lastModelConfigCache = configRaw ? JSON.parse(configRaw) : null;
    } catch (e) {
        lastModelConfigCache = null;
    }
    lastAgentIdCache = agentIdRaw || '';
}

// 读取上次模型配置
function getLastModelConfig() {
    return lastModelConfigCache;
}

// 读取上次智能体选择
function getLastAgentId() {
    return lastAgentIdCache;
}

// 保存当前模型配置到后端 Web 存储
function saveLastModelConfig() {
    const providerId = document.getElementById('providerSelect').value;
    const model = document.getElementById('modelSelect').value.trim();
    const thinking = document.getElementById('thinkingSelect').value;
    if (providerId) {
        lastModelConfigCache = {provider_id: providerId, model: model, thinking: thinking};
        setWebStorage(LAST_MODEL_CONFIG_KEY, JSON.stringify(lastModelConfigCache));
    }
}

// 保存智能体选择到后端 Web 存储
function saveLastAgentId(agentId) {
    lastAgentIdCache = agentId ? String(agentId) : '';
    setWebStorage(LAST_AGENT_KEY, lastAgentIdCache);
}

// 恢复上次模型配置到控件
function restoreLastModelConfig() {
    const config = getLastModelConfig();
    if (!config) {
        return;
    }
    // 已选择智能体时由其配置覆盖，不恢复手动模型配置
    const agentSelect = document.getElementById('agentSelect');
    if (agentSelect && agentSelect.value) {
        return;
    }
    const providerSelect = document.getElementById('providerSelect');
    const modelInput = document.getElementById('modelSelect');
    const thinkingSelect = document.getElementById('thinkingSelect');
    // 供应商下拉框需等选项加载完再赋值
    if (config.provider_id) {
        providerSelect.value = config.provider_id;
    }
    if (config.model) {
        modelInput.value = config.model;
    }
    if (config.thinking) {
        thinkingSelect.value = config.thinking;
    }
}

// ===== 6. 模型路由控件 =====
// 加载智能体下拉框，首项为默认（不选智能体），选择结果仅在新会话首次发送时生效
async function loadAgentSelect() {
    const select = document.getElementById('agentSelect');
    try {
        const response = await fetch('/agent/list');
        const agents = await response.json();
        select.innerHTML = `<option value="" data-i18n="common.none">${t('common.none')}</option>`;
        agents.forEach(agent => {
            const opt = document.createElement('option');
            opt.value = agent.id;
            opt.textContent = agent.name;
            select.appendChild(opt);
        });
        // 恢复上次选择的智能体（若仍存在）
        const lastAgentId = getLastAgentId();
        if (lastAgentId && agents.some(a => String(a.id) === String(lastAgentId))) {
            select.value = lastAgentId;
        }
        // 触发选择逻辑：填入对应模型配置或解禁手动模型
        onAgentSelectChange();
    } catch (e) {
        // 静默处理错误
    }
}

// 选择智能体后填入其模型配置并禁止修改，取消选择（NONE）后解除禁用
async function onAgentSelectChange() {
    const agentId = document.getElementById('agentSelect').value;
    // 智能体选择变化会影响新会话的提示词来源预判
    updatePromptSourceHint();
    // 记录智能体选择，供下次新对话自动填入
    saveLastAgentId(agentId);
    const providerSelect = document.getElementById('providerSelect');
    const modelInput = document.getElementById('modelSelect');
    const thinkingSelect = document.getElementById('thinkingSelect');
    if (!agentId) {
        providerSelect.disabled = false;
        modelInput.disabled = false;
        thinkingSelect.disabled = false;
        return;
    }
    try {
        const response = await fetch('/agent/list');
        const agents = await response.json();
        const agent = agents.find(a => String(a.id) === String(agentId));
        if (agent) {
            providerSelect.value = String(agent.model_provider_id);
            modelInput.value = agent.model || '';
            thinkingSelect.value = String(agent.thinking);
            providerSelect.disabled = true;
            modelInput.disabled = true;
            thinkingSelect.disabled = true;
        }
    } catch (e) {
        // 静默处理错误
    }
}

// 加载对话输入区的供应商下拉框
async function loadModelSelect() {
    const providerSelect = document.getElementById('providerSelect');
    const prevProvider = providerSelect.value;
    try {
        const pResponse = await fetch('/model-provider/list');
        const providers = await pResponse.json();

        // 填充供应商下拉并保持刷新前的选中项，加载完成后恢复上次模型配置
        fillSelectOptions(providerSelect, providers, prevProvider);
        restoreLastModelConfig();
    } catch (e) {
        // 静默处理错误
    }
}

// 渲染模型下拉列表：聚焦时调用模型列表接口拉取当前供应商全部模型并全量展示
async function renderModelComboList() {
    const comboList = document.getElementById('modelComboList');
    const input = document.getElementById('modelSelect');
    const providerId = document.getElementById('providerSelect').value;
    if (!providerId) {
        comboList.classList.remove('open');
        return;
    }
    // 缓存未命中时调用后端接口实时拉取该供应商的模型列表，失败时移除缓存以便下次重试
    if (!providerModelsCache[providerId]) {
        providerModelsCache[providerId] = (async () => {
            try {
                const response = await fetch(`/model-provider/${providerId}/model/list`);
                if (!response.ok) {
                    return [];
                }
                const models = await response.json();
                return Array.isArray(models) ? models : [];
            } catch (e) {
                return [];
            }
        })();
    }
    const models = await providerModelsCache[providerId];
    if (models.length === 0) {
        delete providerModelsCache[providerId];
    }
    // 等待期间供应商已切换则放弃本次渲染
    if (String(providerId) !== document.getElementById('providerSelect').value) {
        return;
    }
    comboList.innerHTML = '';
    // 拉取失败或供应商无模型时展示提示，告知用户可直接手动输入模型名
    if (models.length === 0) {
        const hintItem = document.createElement('div');
        hintItem.className = 'model-combo-item model-combo-hint';
        hintItem.textContent = t('input.modelListUnavailable');
        comboList.appendChild(hintItem);
        comboList.classList.add('open');
        return;
    }
    models.forEach(model => {
        const item = document.createElement('div');
        item.className = 'model-combo-item';
        const textSpan = document.createElement('span');
        textSpan.className = 'model-combo-item-text';
        textSpan.textContent = model;
        textSpan.onclick = () => {
            input.value = model;
            comboList.classList.remove('open');
        };
        item.appendChild(textSpan);
        comboList.appendChild(item);
    });
    comboList.classList.add('open');
}

// ===== 7. 消息发送 =====
// 发送按钮统一入口：流式输出中按钮为停止，否则发送消息（Enter 键走 sendMessage，不受停止逻辑影响）
function sendButtonClick() {
    if (isTyping) {
        stopConversation();
        return;
    }
    sendMessage();
}

// 停止当前对话：调用后端停止接口，409（未在运行/已停止）幂等静默忽略
async function stopConversation() {
    if (!currentConversationId) {
        return;
    }
    try {
        const response = await fetch(`/conversation/${currentConversationId}/stop`, {method: 'POST'});
        if (!response.ok && response.status !== 409) {
            showToast(t('stream.stopFailed'), 'error');
        }
    } catch (e) {
        showToast(t('stream.stopFailed'), 'error');
    }
}

async function sendMessage() {
    const message = messageInput.value.trim();
    // 只读会话（任务/定时来源）禁止发送，作为禁用控件之外的防御性校验
    if (!message || isTyping || currentConvReadonly) {
        return;
    }

    // 从模型路由控件读取发送参数，缺少供应商或模型时明确提示配置缺失，与启动失败区分
    const providerId = document.getElementById('providerSelect').value;
    const modelName = document.getElementById('modelSelect').value.trim();
    if (!providerId || !modelName) {
        showToast(t('stream.configMissing'), 'error');
        return;
    }
    const modelConfig = {model_provider_id: parseInt(providerId), model: modelName, thinking: document.getElementById('thinkingSelect').value === 'true'};
    // 保存当前模型配置供下次新对话自动填入（仅记最近一次）
    saveLastModelConfig();

    // 启动对话：新会话先创建，已有会话直接启动
    try {
        if (!currentConversationId) {
            // 新会话发送前校验工作目录，避免未设置时静默落到后端默认目录
            if (!currentWorkdir) {
                showToast(t('stream.workdirMissing'), 'error');
                return;
            }
            // 新会话可指定智能体，未选择（空值）则不携带 agent_id
            const payload = {task_content: message, work_dir: currentWorkdir, ...modelConfig};
            const agentId = document.getElementById('agentSelect').value;
            if (agentId) {
                payload.agent_id = parseInt(agentId);
            }
            const response = await fetch('/conversation/start', {
                method: 'POST',
                headers: {'Content-Type': 'application/json'},
                body: JSON.stringify(payload)
            });
            if (!response.ok) {
                showToast(t('stream.startFailed'), 'error');
                return;
            }
            currentConversationId = await response.json();
            conversationInfo.textContent = `ID: ${currentConversationId}`;
            // 对话已创建，锁定工作目录与智能体选择
            setContextLocked(true);
            await loadConversationList();
        } else {
            const response = await fetch(`/conversation/${currentConversationId}/start`, {
                method: 'POST',
                headers: {'Content-Type': 'application/json'},
                body: JSON.stringify({task_content: message, ...modelConfig})
            });
            if (!response.ok) {
                showToast(t('stream.startFailed'), 'error');
                return;
            }
        }
    } catch (e) {
        showToast(t('stream.startFailed'), 'error');
        return;
    }

    messageInput.value = '';
    autoResize();
    // 清空后由流式接口回放全部内容（含新消息），避免重复渲染
    chatContainer.innerHTML = '';
    emptyState.style.display = 'none';
    connectStream(currentConversationId);
    scrollToBottom();
}

// ===== 8. SSE 流式渲染 =====
function addUserMessage(content, time) {
    emptyState.style.display = 'none';
    const div = document.createElement('div');
    div.className = 'user-message';
    div.innerHTML = `${formatMarkdown(content.trim())}<div class="message-time">${time}</div>`;
    chatContainer.appendChild(div);
}

// 结束当前流式块：移除光标样式并折叠详情
function finalizeStreamBlock() {
    if (streamContentNode) {
        streamContentNode.classList.remove('streaming-active');
        const prevDetails = streamContentNode.closest('details');
        if (prevDetails) {
            prevDetails.open = false;
        }
        streamContentNode = null;
    }
}

// 连接对话流式接口：先回放历史 chunks，再实时跟随新数据
function connectStream(conversationId) {
    // 关闭旧连接，重置流式渲染状态
    if (currentEventSource) {
        currentEventSource.close();
        currentEventSource = null;
    }
    streamWrapper = null;
    streamContentNode = null;
    streamRawText = '';
    streamChunkCount = 0;
    // 重置 token 用量累计并清空 header 展示
    usageInputTokens = 0;
    usageOutputTokens = 0;
    usageCacheTokens = 0;
    usageTotalTokens = 0;
    usageInfo.textContent = '';
    setTyping(true);

    const source = new EventSource(`/conversation/${conversationId}/stream`);
    currentEventSource = source;

    source.onmessage = (event) => {
        const data = JSON.parse(event.data);
        streamChunkCount += 1;

        // 系统提示词：渲染为可折叠块，展示在消息流开头
        if (data.type === 'system') {
            if (data.text) {
                const details = document.createElement('details');
                details.className = 'system-details';
                details.innerHTML = `<summary>${FOLD_SVG} ${t('stream.systemPrompt')}</summary><div class="content"></div>`;
                details.querySelector('.content').innerHTML = formatMarkdown(data.text);
                chatContainer.appendChild(details);
            }
        }

        // 错误消息
        if (data.type === 'error') {
            finalizeStreamBlock();
            streamWrapper = null;
            const errorDiv = document.createElement('div');
            errorDiv.className = 'user-message stream-error';
            errorDiv.innerHTML = `\u26a0 ${escapeHtml(data.text || t('stream.unknownError'))}`;
            chatContainer.appendChild(errorDiv);
        }

        // 手动停止提示条：结束当前流式块，展示中性提示
        if (data.type === 'stopped') {
            finalizeStreamBlock();
            streamWrapper = null;
            const stoppedDiv = document.createElement('div');
            stoppedDiv.className = 'user-message stream-stopped';
            stoppedDiv.textContent = t('stream.stopped');
            chatContainer.appendChild(stoppedDiv);
        }

        // 用户消息：结束当前助手消息组，渲染用户气泡
        if (data.type === 'user') {
            finalizeStreamBlock();
            streamWrapper = null;
            addUserMessage(data.text || '', '');
        }

        // 助手消息块：thinking / text / tool_use / tool_result
        if (data.type === 'thinking' || data.type === 'text' || data.type === 'tool_use' || data.type === 'tool_result') {
            finalizeStreamBlock();
            if (!streamWrapper) {
                streamWrapper = document.createElement('div');
                streamWrapper.className = 'assistant-message';
                chatContainer.appendChild(streamWrapper);
            }
            streamRawText = data.text || '';

            // 添加新消息块
            if (data.type === 'thinking') {
                const details = document.createElement('details');
                details.className = 'think-details';
                details.open = true;
                details.innerHTML = `<summary>${FOLD_SVG} ${t('stream.thoughtProcess')}</summary><div class="content streaming-active"></div>`;
                streamWrapper.appendChild(details);
                streamContentNode = details.querySelector('.content');
            } else if (data.type === 'text') {
                const div = document.createElement('div');
                div.className = 'reply-content streaming-active';
                streamWrapper.appendChild(div);
                streamContentNode = div;
            } else if (data.type === 'tool_use') {
                const details = document.createElement('details');
                details.className = 'tool-details';
                details.open = true;
                details.innerHTML = `<summary>${FOLD_SVG} ${t('stream.callPrefix')}: ${escapeHtml(data.name || t('stream.tool'))}</summary><div class="content streaming-active"></div>`;
                streamWrapper.appendChild(details);
                streamContentNode = details.querySelector('.content');
            } else if (data.type === 'tool_result') {
                const details = document.createElement('details');
                details.className = 'tool-details';
                details.open = false;
                const status = data.is_error ? t('stream.toolError') : t('stream.toolResult');
                details.innerHTML = `<summary>${FOLD_SVG} ${t('stream.tool')} ${status} [${escapeHtml(String(data.id || ''))}]</summary><div class="content streaming-active"></div>`;
                streamWrapper.appendChild(details);
                streamContentNode = details.querySelector('.content');
            }

            // 设置消息文本
            if (streamRawText && streamContentNode) {
                streamContentNode.innerHTML = formatMarkdown(streamRawText);
            }
        }

        // token 用量：累计本次连接的全部 usage（工具循环会有多条），更新 header 展示
        if (data.type === 'usage') {
            usageInputTokens += data.input_tokens || 0;
            usageOutputTokens += data.output_tokens || 0;
            usageCacheTokens += data.cache_read_input_tokens || 0;
            // 当次 usage 事件三项之和，表示本轮对话的总 token 量
            usageTotalTokens = (data.input_tokens || 0) + (data.output_tokens || 0) + (data.cache_read_input_tokens || 0);
            const formatTokens = (count) => count >= 1000 ? (count / 1000).toFixed(1) + 'k' : String(count);
            let usageText = `↑ ${formatTokens(usageInputTokens)} ${t('stream.usageIn')} · ${formatTokens(usageOutputTokens)} ${t('stream.usageOut')}`;
            if (usageCacheTokens > 0) {
                usageText += ` · ${formatTokens(usageCacheTokens)} ${t('stream.usageCache')}`;
            }
            usageText += ` · ${formatTokens(usageTotalTokens)} ${t('stream.usageTotal')}`;
            usageInfo.textContent = usageText;
        }

        // 追加消息文本
        if (data.type === 'delta') {
            streamRawText += (data.text || '');
            if (streamContentNode) {
                streamContentNode.innerHTML = formatMarkdown(streamRawText);
            }
        }

        // 跟随滚动
        scrollToBottomIfNotUserScroll();
    };

    source.onerror = async () => {
        // 已被新连接替换时忽略
        if (currentEventSource !== source) {
            return;
        }
        // 关闭连接，仅收尾，不重新打开避免无限重连
        source.close();
        currentEventSource = null;
        finalizeStreamBlock();
        // 流关闭且无任何数据时回退到空状态页
        if (streamChunkCount === 0) {
            chatContainer.appendChild(emptyState);
            emptyState.style.display = 'flex';
        }
        setTyping(false);
        // 刷新会话列表
        await loadConversationList();
    };
}
