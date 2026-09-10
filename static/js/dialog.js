// ==========================================
// 会话模块: 会话历史, 模型路由与 SSE 流式交互 (DIALOG)
// ==========================================

// ===== 1. 会话状态与持久化缓存 =====
// 当前已加载会话的系统提示词来源状态(loadConversation 时赋值, startNewChat 时清空)
let currentSystemPrompt = '';
let currentConvAgentName = '';

// 上次模型配置与智能体选择记忆(后端 Web 存储, 仅记最近一次并自动填入)
const LAST_MODEL_CONFIG_KEY = 'openagents_last_model_config';
const LAST_AGENT_KEY = 'openagents_last_agent_id';

// 内存缓存: 页面加载时从后端拉取, 变更时整体写回
let lastModelConfigCache = null;
let lastAgentIdCache = '';
// 供应商模型列表缓存: key 为供应商 id, 值为 Promise(并发去重), 避免聚焦时重复请求
const providerModelsCache = {};

// ===== 2. 提示词来源提示 =====
// 已建会话按快照判断(智能体 > 工作目录 AGENTS.md > 无), 新会话按当前控件状态预判
function updatePromptSourceHint() {
    const hint = document.getElementById('promptSourceHint');
    let text;
    let title;
    if (currentConversationId) {
        if (currentConvAgentName) {
            text = t('input.promptSourceAgent', {name: currentConvAgentName});
            title = text;
        } else if (currentSystemPrompt) {
            // 无智能体且提示词非空, 说明创建时读取到了工作目录下的 AGENTS.md
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
            // 未选智能体的新会话, 提示词将在启动时从工作目录 AGENTS.md 读取
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
    // 同步刷新字数计数(输入, 清空等所有路径都会经过本函数)
    const charCounter = document.getElementById('inputCharCount');
    if (charCounter) {
        charCounter.textContent = `${messageInput.value.length}/${messageInput.maxLength}`;
        // 达到上限时计数器标红; maxlength 会静默截断粘贴内容, 需显式提示用户
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
    // 流式期间发送按钮切换为停止按钮(含只读会话), 非流式时恢复发送; 只读会话非流式时禁用发送
    sendButton.textContent = typing ? t('input.stop') : t('input.execute');
    sendButton.disabled = currentConvReadonly && !typing;
    messageInput.disabled = typing || currentConvReadonly;
}

// 只读会话切换: 任务/定时来源的对话仅供查看, 禁用输入框与发送按钮; 恢复启用时需尊重流式输出中的禁用状态
function setConversationReadonly(readonly) {
    currentConvReadonly = readonly;
    messageInput.disabled = readonly || isTyping;
    // 只读会话仅流式期间允许点击(此时按钮为停止), 与 setTyping 的按钮禁用逻辑保持一致
    sendButton.disabled = readonly && !isTyping;
    messageInput.placeholder = readonly ? t('input.readonlyPlaceholder') : t('input.placeholder');
}

// 锁定/解锁会话上下文: 对话创建后工作目录与智能体不允许修改, 锁定期间展示说明文字避免误解
function setContextLocked(locked) {
    const workspaceBtn = document.getElementById('workspaceBtn');
    const agentSelect = document.getElementById('agentSelect');
    // 禁用态视觉统一由 CSS :disabled 规则(透明度 0.4 + 禁止光标)承担
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
            // 标题拆为内层标题与流式光标两层: 光标常驻 DOM 由 CSS 控制显隐, 长标题截断不影响光标显示
            item.innerHTML = `
                <span class="conversation-item-text">
                    <span class="conversation-item-title">${escapeHtml(conversation.title)}</span>
                    <span class="stream-cursor">▌</span>
                </span>
                <button class="delete-btn">${DELETE_SVG}</button>
            `;
            // 删除按钮通过闭包绑定, 避免标题中的引号破坏内联 onclick 字符串
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
        // 按流会话注册表补充流式标志(页面初始化/列表刷新后恢复进行中的圆点)
        syncStreamDots();
    } catch (e) {
        // 静默处理错误
    }
}

// 同步会话列表流式标志: 依据流会话注册表逐项切换 streaming 类, 在流开始/结束与列表重渲染时调用
function syncStreamDots() {
    conversationList.querySelectorAll('.conversation-item').forEach(item => {
        item.classList.toggle('streaming', streamSessions[item.dataset.id] != null);
    });
}

// 加载指定对话: 切换当前会话并从对话详情接口同步工作目录与模型路由配置; readonly 为 true 时(任务/定时来源)禁止发送消息
async function loadConversation(conversationId, readonly = false) {
    currentConversationId = conversationId;
    currentConvReadonly = readonly;
    // 对话已创建, 锁定工作目录与智能体选择
    setContextLocked(true);
    // 从对话详情接口获取工作目录与配置(智能体/模型提供方/模型/是否思考)
    try {
        const response = await fetch(`/conversation/${conversationId}`);
        if (response.ok) {
            const conversation = await response.json();
            // 缓存系统提示词来源状态, 供来源提示展示
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
                // 智能体执行对话的模型配置固定, 锁定不可修改
                providerSelect.disabled = true;
                modelInput.disabled = true;
                thinkingSelect.disabled = true;
            } else {
                // 用户对话: 清空智能体选择并解锁模型路由控件
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
    messageInput.value = '';
    autoResize();
    conversationInfo.textContent = `ID: ${conversationId}`;

    const items = conversationList.querySelectorAll('.conversation-item');
    items.forEach(item => item.classList.toggle('active', String(item.dataset.id) === String(conversationId)));

    // 应用只读状态: 任务/定时来源的会话禁用输入框与发送按钮
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
                // 被删除对话可能仍有后台流会话, 关闭连接并移除注册项避免悬挂
                const session = streamSessions[conversationId];
                if (session) {
                    session.source.close();
                    delete streamSessions[conversationId];
                }
                if (String(currentConversationId) === String(conversationId)) {
                    startNewChat();
                }
                await loadConversationList();
            } catch (e) {
                showToast(t('common.purgeFailure'), 'error');
            }
        }
    });
}

async function startNewChat() {
    currentConversationId = null;
    // 清空已加载会话的提示词来源状态, 恢复为新会话预判模式
    currentSystemPrompt = '';
    currentConvAgentName = '';
    // 后台流会话保持运行不断开, 下方清空聊天容器仅摘下其会话容器(容器与已渲染内容保留在注册表中)
    // 新会话恢复可输入状态(清除任务/定时来源的只读标记与占位文案)
    setConversationReadonly(false);
    // 重置流式状态: 旧会话可能仍在输出, 新建会话需将停止按钮恢复为发送按钮
    setTyping(false);
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
    // 先加载后端持久化偏好, 再恢复智能体与模型配置
    await loadDialogPrefs();
    loadAgentSelect();
    loadModelSelect();
    setContextLocked(false);
    updatePromptSourceHint();
    switchView('dialog');
}

// ===== 5. 模型路由偏好(后端 Web 存储持久化) =====
// 从后端加载会话页持久化偏好到内存缓存(上次模型配置/上次智能体)
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
    // 已选择智能体时由其配置覆盖, 不恢复手动模型配置
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
        // 兼容旧版布尔存储: true/false 映射为 medium/off
        thinkingSelect.value = config.thinking === 'true' ? 'medium' : config.thinking === 'false' ? 'off' : config.thinking;
    }
}

// ===== 6. 模型路由控件 =====
// 加载智能体下拉框, 首项为默认(不选智能体), 选择结果仅在新会话首次发送时生效
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
        // 恢复上次选择的智能体(若仍存在)
        const lastAgentId = getLastAgentId();
        if (lastAgentId && agents.some(a => String(a.id) === String(lastAgentId))) {
            select.value = lastAgentId;
        }
        // 触发选择逻辑: 填入对应模型配置或解禁手动模型
        onAgentSelectChange();
    } catch (e) {
        // 静默处理错误
    }
}

// 选择智能体后填入其模型配置并禁止修改, 取消选择(NONE)后解除禁用
async function onAgentSelectChange() {
    const agentId = document.getElementById('agentSelect').value;
    // 智能体选择变化会影响新会话的提示词来源预判
    updatePromptSourceHint();
    // 记录智能体选择, 供下次新对话自动填入
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

        // 填充供应商下拉并保持刷新前的选中项, 加载完成后恢复上次模型配置
        fillSelectOptions(providerSelect, providers, prevProvider);
        restoreLastModelConfig();
    } catch (e) {
        // 静默处理错误
    }
}

// 渲染模型下拉列表: 聚焦时调用模型列表接口拉取当前供应商全部模型并全量展示(对话页与智能体页共用, 按参数指定输入框/列表/供应商来源)
async function renderModelComboList(inputId, comboListId, providerSelectId) {
    const comboList = document.getElementById(comboListId);
    const input = document.getElementById(inputId);
    const providerId = document.getElementById(providerSelectId).value;
    if (!providerId) {
        comboList.classList.remove('open');
        return;
    }
    // 缓存未命中时调用后端接口实时拉取该供应商的模型列表, 失败时移除缓存以便下次重试
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
    if (String(providerId) !== document.getElementById(providerSelectId).value) {
        return;
    }
    comboList.innerHTML = '';
    // 拉取失败或供应商无模型时展示提示, 告知用户可直接手动输入模型名
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
// 发送按钮统一入口: 流式输出中按钮为停止, 否则发送消息(Enter 键走 sendMessage, 不受停止逻辑影响)
function sendButtonClick() {
    if (isTyping) {
        stopConversation();
        return;
    }
    sendMessage();
}

// 停止当前对话: 调用后端停止接口, 409(未在运行/已停止)幂等静默忽略
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

// 回退到指定用户消息: 后台删除该消息及其之后的全部记录, 返回的消息文本填入输入框, 前端重新连接流刷新页面
async function rollbackMessage(messageId) {
    // 只读会话与流式输出中禁止回退, 作为隐藏按钮之外的防御性校验
    if (isTyping || currentConvReadonly || !currentConversationId) {
        return;
    }
    showConfirmDialog({
        title: t('stream.rollbackTitle'),
        text: t('stream.rollbackText'),
        onConfirm: async () => {
            try {
                const response = await fetch(`/conversation/${currentConversationId}/rollback/${messageId}`, {method: 'DELETE'});
                if (!response.ok) {
                    showToast(t('stream.rollbackFailed'), 'error');
                    return;
                }
                // 被回退的用户消息文本填入输入框, 用户可编辑后重新发送
                messageInput.value = await response.json();
                autoResize();
            } catch (e) {
                showToast(t('stream.rollbackFailed'), 'error');
                return;
            }
            // 清空后由流式接口回放回退后的历史, 避免重复渲染
            chatContainer.innerHTML = '';
            emptyState.style.display = 'none';
            connectStream(currentConversationId);
            scrollToBottom();
        }
    });
}

async function sendMessage() {
    const message = messageInput.value.trim();
    // 只读会话(任务/定时来源)禁止发送, 作为禁用控件之外的防御性校验; 新会话首条消息不允许为空, 已有会话允许空消息
    if (isTyping || currentConvReadonly || (!message && !currentConversationId)) {
        return;
    }

    // 从模型路由控件读取发送参数, 缺少供应商或模型时明确提示配置缺失, 与启动失败区分
    const providerId = document.getElementById('providerSelect').value;
    const modelName = document.getElementById('modelSelect').value.trim();
    if (!providerId || !modelName) {
        showToast(t('stream.configMissing'), 'error');
        return;
    }
    const modelConfig = {model_provider_id: parseInt(providerId), model: modelName, thinking: document.getElementById('thinkingSelect').value};
    // 保存当前模型配置供下次新对话自动填入(仅记最近一次)
    saveLastModelConfig();

    // 启动对话: 新会话先创建, 已有会话直接启动
    try {
        if (!currentConversationId) {
            // 新会话发送前校验工作目录, 避免未设置时静默落到后端默认目录
            if (!currentWorkdir) {
                showToast(t('stream.workdirMissing'), 'error');
                return;
            }
            // 新会话可指定智能体, 未选择(空值)则不携带 agent_id
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
            // 对话已创建, 锁定工作目录与智能体选择
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
    // 清空后由流式接口回放全部内容(含新消息), 避免重复渲染
    chatContainer.innerHTML = '';
    emptyState.style.display = 'none';
    connectStream(currentConversationId);
    scrollToBottom();
}

// ===== 8. SSE 流式渲染 =====
// 连接对话流式接口: 注册表已有该对话的流会话则收养挂回(后台流持续渲染, 切换零重放), 否则新建会话先回放历史 chunks 再实时跟随; 流自然结束时移除注册项, 下次访问重新请求接口回放最新数据; chunk 渲染复用 core.js 的流式渲染器(与阶段弹窗同一套规则)
function connectStream(conversationId) {
    // 摘下当前挂着的流会话(若仍在注册表中): 连接保持后台运行, 仅记录滚动位置并将容器移出可视区
    Object.keys(streamSessions).forEach(id => {
        const session = streamSessions[id];
        if (session.container.parentNode === chatContainer) {
            session.atBottom = isAtBottom;
            session.scrollTop = viewDialog.scrollTop;
            session.container.remove();
        }
    });
    // 清空可视区残留内容(空态提示/已结束会话的旧容器), 会话容器统一走挂接
    chatContainer.innerHTML = '';
    emptyState.style.display = 'none';

    // 收养已有会话: 容器挂回可视区, 同步用量展示/流式按钮状态/滚动位置
    const existing = streamSessions[conversationId];
    if (existing) {
        chatContainer.appendChild(existing.container);
        usageInfo.textContent = existing.usageText;
        setTyping(true);
        if (existing.atBottom === false) {
            userScroll = true;
            isAtBottom = false;
            programScroll = true;
            viewDialog.scrollTop = existing.scrollTop;
        } else {
            scrollToBottom();
        }
        return;
    }

    // 新建流会话: 独立容器与渲染器, 容器挂入可视区, 连接保持打开直到流自然结束
    const container = document.createElement('div');
    // 会话容器不生成布局盒, 消息块直接参与聊天容器 flex 布局(align-self 等样式表现与直连渲染一致)
    container.style.display = 'contents';
    const source = new EventSource(`/conversation/${conversationId}/stream`);
    const session = {
        source: source,
        renderer: createStreamRenderer(container, !currentConvReadonly),
        container: container,
        chunkCount: 0,
        usageInput: 0,
        usageOutput: 0,
        usageCache: 0,
        usageText: '',
        atBottom: true,
        scrollTop: null
    };
    streamSessions[conversationId] = session;
    // 流开始, 点亮左侧列表的流式标志
    syncStreamDots();
    chatContainer.appendChild(container);
    usageInfo.textContent = '';
    setTyping(true);

    source.onmessage = (event) => {
        const data = JSON.parse(event.data);
        session.chunkCount += 1;
        const isCurrent = String(currentConversationId) === String(conversationId);

        // token 用量: 对话页特有, 累计本连接的全部 usage(工具循环会有多条), 仅当前可见会话更新 header 展示
        if (data.type === 'usage') {
            session.usageInput += data.input_tokens || 0;
            session.usageOutput += data.output_tokens || 0;
            session.usageCache += data.cache_read_input_tokens || 0;
            // 当次 usage 事件三项之和, 表示本轮对话的总 token 量
            const usageTotal = (data.input_tokens || 0) + (data.output_tokens || 0) + (data.cache_read_input_tokens || 0);
            const formatTokens = (count) => count >= 1000 ? (count / 1000).toFixed(1) + 'k' : String(count);
            let usageText = `↑ ${formatTokens(session.usageInput)} ${t('stream.usageIn')} · ${formatTokens(session.usageOutput)} ${t('stream.usageOut')}`;
            if (session.usageCache > 0) {
                usageText += ` · ${formatTokens(session.usageCache)} ${t('stream.usageCache')}`;
            }
            usageText += ` · ${formatTokens(usageTotal)} ${t('stream.usageTotal')}`;
            session.usageText = usageText;
            if (isCurrent) {
                usageInfo.textContent = usageText;
            }
            return;
        }

        // 后台会话持续向各自容器渲染, 滚动仅跟随当前可见会话
        session.renderer.handleChunk(data);
        if (isCurrent) {
            scrollToBottomIfNotUserScroll();
        }
    };

    source.onerror = async () => {
        // 流自然结束: 关闭连接并移除注册项, 下次访问该对话时重新请求接口回放
        source.close();
        delete streamSessions[conversationId];
        // 流结束, 熄灭左侧列表的流式标志(含后台结束的会话)
        syncStreamDots();
        session.renderer.finalize();
        // 后台结束的会话仅移除注册项, 界面收尾仅对当前可见会话执行
        if (String(currentConversationId) !== String(conversationId)) {
            return;
        }
        // 流关闭且无任何数据时回退到空状态页
        if (session.chunkCount === 0) {
            chatContainer.appendChild(emptyState);
            emptyState.style.display = 'flex';
        }
        setTyping(false);
        // 刷新会话列表
        await loadConversationList();
    };
}
