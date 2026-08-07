// ==========================================
// TASK 任务流水线管理面板
// ==========================================
const taskListContainer = document.getElementById('taskListContainer');
const addTaskPanel = document.getElementById('addTaskPanel');

function toggleAddTaskPanel() {
    addTaskPanel.style.display = addTaskPanel.style.display === 'none' ? 'flex' : 'none';
}

// 渲染候选 Agent 多选列表，agents 为 Agent 列表，checkedIds 为已选中的 Agent id
function renderAgentCheckList(container, agents, checkedIds) {
    container.innerHTML = '';
    if (agents.length === 0) {
        container.innerHTML = `<div style="font-size:12px; color:var(--slate-400); font-family:var(--font-mono);">${t('task.noAgents')}</div>`;
        return;
    }
    agents.forEach(agent => {
        const label = document.createElement('label');
        label.className = 'agent-check-item';
        label.innerHTML = `<input type="checkbox" value="${agent.id}" ${checkedIds.includes(agent.id) ? 'checked' : ''}> ${escapeHtml(agent.name)}`;
        container.appendChild(label);
    });
}

// 读取多选列表中选中的 Agent id
function getCheckedAgentIds(container) {
    return Array.from(container.querySelectorAll('input[type=checkbox]:checked')).map(input => parseInt(input.value));
}

async function fetchTaskList() {
    taskListContainer.innerHTML = SKELETON_HTML;
    try {
        // 并行拉取任务清单与 Agent 花名册（用于候选 Agent 多选与启动下拉）
        const [taskResponse, agentResponse] = await Promise.all([fetch('/task/list'), fetch('/agent/list')]);
        const tasks = await taskResponse.json();
        const agents = await agentResponse.json();
        taskListContainer.innerHTML = '';
        renderAgentCheckList(document.getElementById('addTaskAgentList'), agents, []);

        if (tasks.length === 0) {
            taskListContainer.innerHTML = `<div style="padding:30px; text-align:center; border:1px dashed var(--border-hard); color:var(--slate-400)">${t('task.empty')}</div>`;
            return;
        }

        tasks.forEach(task => {
            // 候选 Agent 名称列表（只读展示）
            const agentNames = (task.agent_ids || []).map(agentId => {
                const agent = agents.find(a => a.id === agentId);
                return agent ? agent.name : String(agentId);
            }).join(', ');
            const card = document.createElement('div');
            card.className = 'info-card';
            card.id = `task-card-${task.id}`;
            card.innerHTML = `
                <div class="info-card-summary" onclick="toggleTaskCard(this.parentNode, ${task.id})">
                    <div class="info-card-main">
                        ${ARROW_SVG}
                        <span class="info-card-name" style="min-width:180px; max-width:280px;">${escapeHtml(task.title)}</span>
                        <span class="info-card-snippet">${escapeHtml(task.content)}</span>
                    </div>
                    <div style="display:flex; gap:8px; align-items:center;" onclick="event.stopPropagation();">
                        <button class="delete-btn" style="opacity:1; padding:6px;">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
                        </button>
                    </div>
                </div>
                <div class="info-card-details" style="display: none;">
                    <div class="details-grid">
                        <div class="details-label">${t('task.title')}</div>
                        <div class="details-value">${escapeHtml(task.title)}</div>
                        <div class="details-label">${t('task.content')}</div>
                        <div class="details-value" style="white-space:pre-wrap;">${escapeHtml(task.content)}</div>
                        <div class="details-label">${t('task.workDir')}</div>
                        <div class="details-value">${escapeHtml(task.work_dir || t('common.inheritedEnv'))}</div>
                        <div class="details-label">${t('task.candidateAgents')}</div>
                        <div class="details-value">${escapeHtml(agentNames) || t('common.none')}</div>
                        <div class="details-label">${t('task.launch')}</div>
                        <div class="details-value">
                            <div style="display:flex; gap:8px; flex-wrap:wrap;">
                                <select id="task-start-agent-${task.id}" class="form-control" style="flex:1; min-width:150px;"></select>
                                <button class="btn btn-sm send-button" onclick="startTask(${task.id})">${t('common.start')}</button>
                            </div>
                        </div>
                        <div class="details-block-container">
                            <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom: 6px;">
                                <div class="details-label" style="margin-bottom: 0;">${t('task.stageProgress')}</div>
                                <button class="btn btn-sm btn-secondary" style="height:24px; padding:0 8px; font-size:10px;" onclick="loadTaskDetail(${task.id})">${t('common.refresh')}</button>
                            </div>
                            <div class="task-stage-list" id="task-stages-${task.id}"></div>
                        </div>
                    </div>
                </div>
            `;
            // 删除按钮通过闭包绑定，避免标题中的引号破坏内联 onclick 字符串
            card.querySelector('.delete-btn').onclick = () => removeTask(task.id, task.title);
            taskListContainer.appendChild(card);
            // 启动下拉仅列出候选 Agent
            const startSelect = document.getElementById(`task-start-agent-${task.id}`);
            (task.agent_ids || []).forEach(agentId => {
                const agent = agents.find(a => a.id === agentId);
                if (agent) {
                    const opt = document.createElement('option');
                    opt.value = agent.id;
                    opt.textContent = agent.name;
                    startSelect.appendChild(opt);
                }
            });
        });
    } catch (e) {
        taskListContainer.innerHTML = `<div style="padding:20px; color:var(--danger-color)">${t('common.fetchFailed')}</div>`;
    }
}

// 展开任务卡片时加载任务详情（阶段对话进展）
function toggleTaskCard(cardElement, taskId) {
    toggleCardOpen(cardElement);
    if (cardElement.hasAttribute('open')) {
        loadTaskDetail(taskId);
    }
}

// 加载任务详情：渲染各阶段对话的最后一条消息
async function loadTaskDetail(taskId) {
    const stageList = document.getElementById(`task-stages-${taskId}`);
    stageList.innerHTML = SKELETON_HTML;
    try {
        const response = await fetch(`/task/${taskId}`);
        if (!response.ok) {
            stageList.innerHTML = `<div style="font-family:var(--font-mono); font-size:12px; color:var(--danger-color)">${t('common.fetchFailed')}</div>`;
            return;
        }
        const task = await response.json();
        stageList.innerHTML = '';
        if (!task.conversations || task.conversations.length === 0) {
            stageList.innerHTML = `<div style="font-size:12px; color:var(--slate-400); font-style:italic;">${t('task.notStarted')}</div>`;
            return;
        }
        task.conversations.forEach(conversation => {
            // agent 对话且无消息说明正在执行中，提示点击查看实时流式内容；用户对话由用户自己处理，不在执行
            const isRunning = conversation.agent_id != null && (!conversation.messages || conversation.messages.length === 0);
            const snippet = isRunning ? t('task.generating') : getLastMessageText(conversation.messages);
            const item = document.createElement('div');
            item.className = 'task-stage-item';
            item.innerHTML = `
                <div class="task-stage-title">
                    <span>${escapeHtml(conversation.title)}</span>
                    <span style="font-family:var(--font-mono); font-weight:400; color:var(--slate-300);">${escapeHtml(conversation.update_time || '')}</span>
                </div>
                <div class="task-stage-snippet"${isRunning ? ' style="color:var(--charcoal-900); font-weight:600;"' : ''}>${escapeHtml(snippet)}</div>
            `;
            // 点击阶段项复用对话页右侧展示区：切换视图并流式回放/跟随该阶段对话
            item.onclick = () => {
                switchView('dialog');
                loadConversation(conversation.id);
            };
            stageList.appendChild(item);
        });
        // 最后一条为用户对话（无 agent_id）且尚无消息（审核中）时，追加用户意见输入框与审核按钮，审核即向该对话追加一条用户消息
        const lastConversation = task.conversations[task.conversations.length - 1];
        if (lastConversation.agent_id == null && (!lastConversation.messages || lastConversation.messages.length === 0)) {
            const reviewArea = document.createElement('div');
            reviewArea.style.cssText = 'display:flex; gap:8px; margin-top:8px; align-items:flex-start;';
            reviewArea.innerHTML = `
                <textarea id="task-review-input-${taskId}" class="form-control" rows="2" style="flex:1; resize:vertical; font-size:11px;" placeholder="${t('task.reviewPlaceholder')}"></textarea>
                <button class="btn btn-sm send-button" onclick="submitTaskReview(${taskId}, ${lastConversation.id})">${t('common.review')}</button>
            `;
            stageList.appendChild(reviewArea);
        }
    } catch (e) {
        stageList.innerHTML = `<div style="font-family:var(--font-mono); font-size:12px; color:var(--danger-color)">${t('common.fetchFailed')}</div>`;
    }
}

// 提交用户审核意见：向当前用户对话追加一条用户消息后刷新阶段列表
async function submitTaskReview(taskId, conversationId) {
    const input = document.getElementById(`task-review-input-${taskId}`);
    const content = input.value.trim();
    if (!content) {
        alert(t('task.reviewRequired'));
        return;
    }
    try {
        const response = await fetch(`/conversation/${conversationId}/message`, {
            method: 'POST',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({content: content})
        });
        if (response.ok) {
            await loadTaskDetail(taskId);
        } else {
            alert(t('task.reviewFault'));
        }
    } catch (e) {
        alert(t('task.reviewFault'));
    }
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

async function submitTask() {
    const title = document.getElementById('taskTitle').value.trim();
    const content = document.getElementById('taskContent').value.trim();
    const workDir = currentWorkdir;
    const agentIds = getCheckedAgentIds(document.getElementById('addTaskAgentList'));
    if (!title || !content || !workDir) {
        alert(t('common.requiredMissing'));
        return;
    }

    try {
        const response = await fetch('/task', {
            method: 'POST',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({title: title, content: content, agent_ids: agentIds, work_dir: workDir})
        });
        if (response.ok) {
            document.getElementById('taskTitle').value = '';
            document.getElementById('taskContent').value = '';
            toggleAddTaskPanel();
            await fetchTaskList();
        }
    } catch (e) {
        alert(t('common.creationFault'));
    }
}

function removeTask(taskId, taskTitle) {
    showConfirmDialog({
        title: t('task.purgeTitle'),
        text: t('task.purgeText', {name: taskTitle}),
        onConfirm: async () => {
            try {
                const response = await fetch(`/task/${taskId}`, {method: 'DELETE'});
                if (response.ok) {
                    await fetchTaskList();
                }
            } catch (e) {
                alert(t('common.purgeFailure'));
            }
        }
    });
}

async function startTask(taskId) {
    const agentId = document.getElementById(`task-start-agent-${taskId}`).value;
    if (!agentId) {
        alert(t('task.agentRequired'));
        return;
    }

    try {
        const response = await fetch(`/task/${taskId}/start`, {
            method: 'POST',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({agent_id: parseInt(agentId)})
        });
        if (response.ok) {
            alert(t('task.launched'));
            await loadTaskDetail(taskId);
        } else if (response.status === 409) {
            alert(t('task.alreadyRunning'));
        } else {
            alert(t('task.startFault'));
        }
    } catch (e) {
        alert(t('task.startFault'));
    }
}

