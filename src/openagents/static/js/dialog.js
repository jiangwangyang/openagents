// ==========================================
// 会话历史/交互流控与 SSE 核心网络模块 (DIALOG)
// ==========================================
function autoResize() {
    messageInput.style.height = 'auto';
    messageInput.style.height = Math.min(messageInput.scrollHeight, 160) + 'px';
}

async function loadConversationList() {
    try {
        const response = await fetch('/conversation/list');
        const conversations = await response.json();
        globalConversationsCachedList = conversations;
        conversationList.innerHTML = '';
        conversations.forEach(conversation => {
            const item = document.createElement('div');
            item.className = 'conversation-item';
            item.dataset.id = conversation.id;
            item.innerHTML = `
                <span class="conversation-item-text">${escapeHtml(conversation.title)}</span>
                <button class="delete-btn">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
                </button>
            `;
            // 删除按钮通过闭包绑定，避免标题中的引号破坏内联 onclick 字符串
            item.querySelector('.delete-btn').onclick = (event) => {
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

function loadConversation(conversationId) {
    currentConversationId = conversationId;
    // 对话已创建，锁定工作目录与智能体选择
    setContextLocked(true);
    // 从缓存的对话列表中读取工作目录
    const cached = globalConversationsCachedList.find(c => String(c.id) === String(conversationId));
    if (cached) {
        updateWorkspaceUI(cached.work_dir);
    }
    chatContainer.innerHTML = '';
    emptyState.style.display = 'none';
    messageInput.value = '';
    autoResize();
    conversationInfo.textContent = `ID: ${conversationId}`;

    const items = conversationList.querySelectorAll('.conversation-item');
    items.forEach(item => item.classList.toggle('active', String(item.dataset.id) === String(conversationId)));

    // 通过 work 流式接口回放历史消息并实时跟随
    connectStream(conversationId);
}

function confirmDeleteConversation(conversationId, convTitle) {
    showConfirmDialog({
        title: "PURGE RECORD",
        text: `Are you sure you want to delete conversation [${convTitle}]? This trace log action is irreversible.`,
        onConfirm: async () => {
            try {
                await fetch(`/conversation/${conversationId}`, {method: 'DELETE'});
                if (String(currentConversationId) === String(conversationId)) {
                    startNewChat();
                }
                await loadConversationList();
            } catch (e) {
                // 异常捕获
            }
        }
    });
}

function startNewChat() {
    currentConversationId = null;
    // 取消历史列表中所有条目的选中高亮
    conversationList.querySelectorAll('.conversation-item').forEach(item => item.classList.remove('active'));
    chatContainer.innerHTML = '';
    chatContainer.appendChild(emptyState);
    emptyState.style.display = 'flex';
    messageInput.value = '';
    autoResize();
    enableInput();
    conversationInfo.textContent = 'NEW TRACE';
    usageInfo.textContent = '';
    initDefaultWorkspace();
    loadAgentSelect();
    setContextLocked(false);
    switchView('dialog');
}

// 锁定/解锁会话上下文：对话创建后工作目录与智能体不允许修改
function setContextLocked(locked) {
    const workspaceBtn = document.getElementById('workspaceBtn');
    const agentSelect = document.getElementById('agentSelect');
    workspaceBtn.disabled = locked;
    agentSelect.disabled = locked;
    workspaceBtn.style.opacity = locked ? '0.4' : '';
    agentSelect.style.opacity = locked ? '0.4' : '';
    workspaceBtn.style.cursor = locked ? 'not-allowed' : '';
    agentSelect.style.cursor = locked ? 'not-allowed' : '';
}

// 加载智能体下拉框，首项为默认（不选智能体），选择结果仅在新会话首次发送时生效
async function loadAgentSelect() {
    const select = document.getElementById('agentSelect');
    try {
        const response = await fetch('/agent/list');
        const agents = await response.json();
        select.innerHTML = '<option value="">NONE</option>';
        agents.forEach(agent => {
            const opt = document.createElement('option');
            opt.value = agent.id;
            opt.textContent = agent.name;
            select.appendChild(opt);
        });
    } catch (e) {
        // 静默处理错误
    }
}

function enableInput() {
    messageInput.disabled = false;
    sendButton.disabled = false;
    messageInput.focus();
}

function addUserMessage(content, time) {
    emptyState.style.display = 'none';
    const div = document.createElement('div');
    div.className = 'user-message';
    div.innerHTML = `${formatMarkdown(content.trim())}<div class="message-time">${time}</div>`;
    chatContainer.appendChild(div);
}

function setTyping(typing) {
    isTyping = typing;
    sendButton.disabled = typing;
    messageInput.disabled = typing;
}

async function sendMessage() {
    const message = messageInput.value.trim();
    if (!message || isTyping) {
        return;
    }

    // 启动 work：新会话先创建，已有会话直接启动
    try {
        if (!currentConversationId) {
            // 新会话可指定智能体，未选择（空值）则不携带 agent_id
            const payload = {task_content: message, work_dir: currentWorkdir};
            const agentId = document.getElementById('agentSelect').value;
            if (agentId) {
                payload.agent_id = parseInt(agentId);
            }
            const response = await fetch('/work/start', {
                method: 'POST',
                headers: {'Content-Type': 'application/json'},
                body: JSON.stringify(payload)
            });
            if (!response.ok) {
                alert("START WORK FAILED");
                return;
            }
            currentConversationId = await response.json();
            conversationInfo.textContent = `ID: ${currentConversationId}`;
            // 对话已创建，锁定工作目录与智能体选择
            setContextLocked(true);
            await loadConversationList();
        } else {
            const response = await fetch(`/work/${currentConversationId}/start`, {
                method: 'POST',
                headers: {'Content-Type': 'application/json'},
                body: JSON.stringify({task_content: message})
            });
            if (!response.ok) {
                alert("START WORK FAILED");
                return;
            }
        }
    } catch (e) {
        alert("START WORK FAILED");
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

// 连接 work 流式接口：先回放历史 chunks，再实时跟随新数据
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
    usageInfo.textContent = '';
    setTyping(true);

    const source = new EventSource(`/work/${conversationId}/stream`);
    currentEventSource = source;

    source.onmessage = (event) => {
        const data = JSON.parse(event.data);
        streamChunkCount += 1;

        // 错误消息
        if (data.type === 'error') {
            finalizeStreamBlock();
            streamWrapper = null;
            const errorDiv = document.createElement('div');
            errorDiv.className = 'user-message';
            errorDiv.style.cssText = 'background: var(--danger-bg); color: var(--danger-color); border: 1px solid var(--danger-color); align-self: center; max-width: 80%;';
            errorDiv.innerHTML = `\u26a0 ${escapeHtml(data.text || 'Unknown Error')}`;
            chatContainer.appendChild(errorDiv);
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
                details.innerHTML = `<summary>${FOLD_SVG} Thought Process</summary><div class="content streaming-active"></div>`;
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
                details.innerHTML = `<summary>${FOLD_SVG} Call: ${escapeHtml(data.name || 'Tool')}</summary><div class="content streaming-active"></div>`;
                streamWrapper.appendChild(details);
                streamContentNode = details.querySelector('.content');
            } else if (data.type === 'tool_result') {
                const details = document.createElement('details');
                details.className = 'tool-details';
                details.open = false;
                const status = data.is_error ? 'Error' : 'Result';
                details.innerHTML = `<summary>${FOLD_SVG} Tool ${status} [${escapeHtml(String(data.id || ''))}]</summary><div class="content streaming-active"></div>`;
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
            const formatTokens = (count) => count >= 1000 ? (count / 1000).toFixed(1) + 'k' : String(count);
            let usageText = `↑ ${formatTokens(usageInputTokens)} in · ${formatTokens(usageOutputTokens)} out`;
            if (usageCacheTokens > 0) {
                usageText += ` · ${formatTokens(usageCacheTokens)} cache`;
            }
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
        // 恢复按钮
        setTyping(false);
        // 刷新列表
        await loadConversationList();
    };
}

