// ==========================================
// 6. TASK 任务流水线管理逻辑
// ==========================================
function toggleAddTaskPanel() {
    addTaskPanel.style.display = addTaskPanel.style.display === 'none' ? 'flex' : 'none';
}

// 渲染候选 Agent 多选列表，checkedIds 为已选中的 Agent id
function renderAgentCheckList(container, checkedIds) {
    container.innerHTML = '';
    if (globalAgentsCachedList.length === 0) {
        container.innerHTML = '<div style="font-size:12px; color:var(--slate-400); font-family:var(--font-mono);">NO REGISTERED AGENTS</div>';
        return;
    }
    globalAgentsCachedList.forEach(agent => {
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
        globalAgentsCachedList = await agentResponse.json();
        taskListContainer.innerHTML = '';
        renderAgentCheckList(document.getElementById('addTaskAgentList'), []);

        if (tasks.length === 0) {
            taskListContainer.innerHTML = '<div style="padding:30px; text-align:center; border:1px dashed var(--border-hard); color:var(--slate-400)">NO TASK PIPELINES FOUND</div>';
            return;
        }

        tasks.forEach(task => {
            // 候选 Agent 名称列表（只读展示）
            const agentNames = (task.agent_ids || []).map(agentId => {
                const agent = globalAgentsCachedList.find(a => a.id === agentId);
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
                        <button class="delete-btn" style="opacity:1; padding:6px;" onclick="removeTask(${task.id}, '${escapeHtml(task.title).replace(/'/g, "\\'")}')">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
                        </button>
                    </div>
                </div>
                <div class="info-card-details" style="display: none;">
                    <div class="details-grid">
                        <div class="details-label">Task Title</div>
                        <div class="details-value">${escapeHtml(task.title)}</div>
                        <div class="details-label">Task Content</div>
                        <div class="details-value" style="white-space:pre-wrap;">${escapeHtml(task.content)}</div>
                        <div class="details-label">Working Directory</div>
                        <div class="details-value">${escapeHtml(task.work_dir || 'Inherited Environment')}</div>
                        <div class="details-label">Candidate Agents</div>
                        <div class="details-value">${escapeHtml(agentNames) || 'NONE'}</div>
                        <div class="details-label">Launch Pipeline</div>
                        <div class="details-value">
                            <div style="display:flex; gap:8px; flex-wrap:wrap;">
                                <select id="task-start-agent-${task.id}" class="form-control" style="flex:1; min-width:150px;"></select>
                                <button class="btn btn-sm send-button" onclick="startTask(${task.id})">Start</button>
                            </div>
                        </div>
                        <div class="details-block-container">
                            <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom: 6px;">
                                <div class="details-label" style="margin-bottom: 0;">Stage Progress</div>
                                <button class="btn btn-sm btn-secondary" style="height:24px; padding:0 8px; font-size:10px;" onclick="loadTaskDetail(${task.id})">Refresh</button>
                            </div>
                            <div class="task-stage-list" id="task-stages-${task.id}"></div>
                        </div>
                    </div>
                </div>
            `;
            taskListContainer.appendChild(card);
            // 启动下拉仅列出候选 Agent
            const startSelect = document.getElementById(`task-start-agent-${task.id}`);
            (task.agent_ids || []).forEach(agentId => {
                const agent = globalAgentsCachedList.find(a => a.id === agentId);
                if (agent) {
                    const opt = document.createElement('option');
                    opt.value = agent.id;
                    opt.textContent = agent.name;
                    startSelect.appendChild(opt);
                }
            });
        });
    } catch (e) {
        taskListContainer.innerHTML = '<div style="padding:20px; color:var(--danger-color)">FETCH FAILED</div>';
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
            stageList.innerHTML = '<div style="font-family:var(--font-mono); font-size:12px; color:var(--danger-color)">FETCH FAILED</div>';
            return;
        }
        const task = await response.json();
        stageList.innerHTML = '';
        if (!task.conversations || task.conversations.length === 0) {
            stageList.innerHTML = '<div style="font-size:12px; color:var(--slate-400); font-style:italic;">PIPELINE NOT STARTED. ZERO STAGES.</div>';
            return;
        }
        task.conversations.forEach(conversation => {
            // agent 对话且无消息说明正在执行中，提示点击查看实时流式内容；用户对话由用户自己处理，不在执行
            const isRunning = conversation.agent_id != null && (!conversation.messages || conversation.messages.length === 0);
            const snippet = isRunning ? '\u26a1 GENERATING... CLICK TO VIEW LIVE STREAM' : getLastMessageText(conversation.messages);
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
                <textarea id="task-review-input-${taskId}" class="form-control" rows="2" style="flex:1; resize:vertical; font-size:11px;" placeholder="Enter review feedback..."></textarea>
                <button class="btn btn-sm send-button" onclick="submitTaskReview(${taskId}, ${lastConversation.id})">Review</button>
            `;
            stageList.appendChild(reviewArea);
        }
    } catch (e) {
        stageList.innerHTML = '<div style="font-family:var(--font-mono); font-size:12px; color:var(--danger-color)">FETCH FAILED</div>';
    }
}

// 提交用户审核意见：向当前用户对话追加一条用户消息后刷新阶段列表
async function submitTaskReview(taskId, conversationId) {
    const input = document.getElementById(`task-review-input-${taskId}`);
    const content = input.value.trim();
    if (!content) {
        alert("REVIEW FEEDBACK IS REQUIRED.");
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
            alert("REVIEW FAULT");
        }
    } catch (e) {
        alert("REVIEW FAULT");
    }
}

// 取对话最后一条消息的展示文本：content 为数组时取最后一个 block 的 text
function getLastMessageText(messages) {
    if (!messages || messages.length === 0) {
        return 'NO MESSAGES';
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
        alert("REQUIRED FIELDS MISSING.");
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
        alert("CREATION FAULT");
    }
}

function removeTask(taskId, taskTitle) {
    showConfirmDialog({
        title: "PURGE TASK PIPELINE",
        text: `Are you sure you want to destroy task pipeline [${taskTitle}]? All stage conversations attached will be wiped out instantly.`,
        onConfirm: async () => {
            try {
                const response = await fetch(`/task/${taskId}`, {method: 'DELETE'});
                if (response.ok) {
                    await fetchTaskList();
                }
            } catch (e) {
                alert("PURGE FAILURE");
            }
        }
    });
}

async function startTask(taskId) {
    const agentId = document.getElementById(`task-start-agent-${taskId}`).value;
    if (!agentId) {
        alert("AGENT IS REQUIRED.");
        return;
    }

    try {
        const response = await fetch(`/task/${taskId}/start`, {
            method: 'POST',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({agent_id: parseInt(agentId)})
        });
        if (response.ok) {
            alert("TASK PIPELINE LAUNCHED.");
            await loadTaskDetail(taskId);
        } else if (response.status === 409) {
            alert("TASK ALREADY RUNNING");
        } else {
            alert("START FAULT");
        }
    } catch (e) {
        alert("START FAULT");
    }
}

// ==========================================
// 7. AGENT 管理逻辑
// ==========================================
function toggleAddAgentPanel() {
    addAgentPanel.style.display = addAgentPanel.style.display === 'none' ? 'flex' : 'none';
}

async function fetchAgentRegistry() {
    agentListContainer.innerHTML = SKELETON_HTML;
    try {
        const response = await fetch('/agent/list');
        const agents = await response.json();
        agentListContainer.innerHTML = '';

        if (agents.length === 0) {
            agentListContainer.innerHTML = '<div style="padding:30px; text-align:center; border:1px dashed var(--border-hard); color:var(--slate-400)">NO REGISTERED AGENTS FOUND</div>';
            return;
        }

        agents.forEach(agent => {
            const card = document.createElement('div');
            card.className = 'info-card';
            card.id = `agent-card-${agent.id}`;

            card.innerHTML = `
                <div class="info-card-summary" onclick="toggleCardOpen(this.parentNode)">
                    <div class="info-card-main">
                        ${ARROW_SVG}
                        <span class="info-card-name" style="min-width:180px; max-width:280px;">${escapeHtml(agent.name)}</span>
                        <span class="info-card-snippet">${escapeHtml(agent.description)}</span>
                    </div>
                    <div style="display:flex; gap:8px; align-items:center;" onclick="event.stopPropagation();">
                        <button class="btn btn-sm send-button" style="height:28px; padding:0 8px; font-size:10px;" onclick="updateSingleAgent(${agent.id})">Save</button>
                        <button class="delete-btn" style="opacity:1; padding:6px;" onclick="removeAgent(${agent.id}, '${escapeHtml(agent.name).replace(/'/g, "\\'")}')">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
                        </button>
                    </div>
                </div>
                <div class="info-card-details" style="display: none;">
                    <div class="details-grid">
                        <div class="details-label">Agent Name</div>
                        <div class="details-value">
                            <input type="text" id="agent-name-${agent.id}" class="form-control" value="${escapeHtml(agent.name)}">
                        </div>

                        <div class="details-label">Description</div>
                        <div class="details-value">
                            <input type="text" id="agent-desc-${agent.id}" class="form-control" value="${escapeHtml(agent.description)}">
                        </div>

                        <div class="details-label">System Prompt</div>
                        <div class="details-value">
                            <textarea id="agent-prompt-${agent.id}" class="form-control mono" rows="6" style="resize: vertical; font-size:11px;">${escapeHtml(agent.prompt)}</textarea>
                        </div>
                    </div>
                </div>
            `;
            agentListContainer.appendChild(card);
        });
    } catch (e) {
        agentListContainer.innerHTML = '<div style="padding:20px; color:var(--danger-color)">ROSTER CAPTURE CRASHED</div>';
    }
}

async function submitAgent() {
    const name = document.getElementById('agentName').value.trim();
    const description = document.getElementById('agentDesc').value.trim();
    const prompt = document.getElementById('agentPrompt').value.trim();
    if (!name) {
        alert("AGENT NAME IS REQUIRED.");
        return;
    }

    try {
        const response = await fetch('/agent', {
            method: 'POST',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({name: name, description: description, prompt: prompt})
        });
        if (response.ok) {
            document.getElementById('agentName').value = '';
            document.getElementById('agentDesc').value = '';
            document.getElementById('agentPrompt').value = '';
            toggleAddAgentPanel();
            await fetchAgentRegistry();
        }
    } catch (e) {
        alert("COMMIT FAILURE");
    }
}

async function updateSingleAgent(id) {
    const name = document.getElementById(`agent-name-${id}`).value.trim();
    const description = document.getElementById(`agent-desc-${id}`).value.trim();
    const prompt = document.getElementById(`agent-prompt-${id}`).value.trim();
    if (!name) {
        alert("AGENT NAME IS REQUIRED.");
        return;
    }

    try {
        const response = await fetch(`/agent/${id}`, {
            method: 'PUT',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({name: name, description: description, prompt: prompt})
        });
        if (response.ok) {
            alert(`AGENT [${name}] SYNCHRONIZED.`);
            await fetchAgentRegistry();
        }
    } catch (e) {
        alert("SYNC CRASHED");
    }
}

function removeAgent(id, name) {
    showConfirmDialog({
        title: "PURGE AGENT NODE",
        text: `Are you sure you want to completely eject agent [${name}] from system roster? Conversations attached will be detached instantly.`,
        onConfirm: async () => {
            try {
                const response = await fetch(`/agent/${id}`, {method: 'DELETE'});
                if (response.ok) {
                    await fetchAgentRegistry();
                }
            } catch (e) {
                alert("PURGE FAILURE");
            }
        }
    });
}

// ==========================================
// 8. CRON 核心排程自动化逻辑
// ==========================================
async function fetchCronTasks() {
    cronListContainer.innerHTML = SKELETON_HTML;
    try {
        const response = await fetch('/schedule/list');
        const tasks = await response.json();
        cronListContainer.innerHTML = '';

        if (tasks.length === 0) {
            cronListContainer.innerHTML = '<div style="padding:30px; text-align:center; border:1px dashed var(--border-hard); color:var(--slate-400)">NO SCHEDULED CRON PROCESSES FOUND</div>';
            return;
        }

        tasks.forEach(task => {
            const card = document.createElement('div');
            card.className = 'info-card';
            card.innerHTML = `
                <div class="info-card-summary" onclick="toggleCardOpen(this.parentNode)">
                    <div class="info-card-main">
                        ${ARROW_SVG}
                        <span class="info-card-name">${escapeHtml(task.name || 'Unnamed Task')}</span>
                        <span class="info-card-snippet">${escapeHtml(task.content || '')}</span>
                    </div>
                    <button class="delete-btn" style="opacity:1;" onclick="event.stopPropagation(); removeCronTask('${task.id}', '${escapeHtml(task.name)}')">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
                    </button>
                </div>
                <div class="info-card-details" style="display: none;">
                    <div class="details-grid">
                        <div class="details-label">Working Dir</div>
                        <div class="details-value">${escapeHtml(task.work_dir || 'Inherited Environment')}</div>
                        <div class="details-label">Next Fire</div>
                        <div class="details-value" style="font-family:var(--font-mono); font-size:12px; color:var(--charcoal-900)">${escapeHtml(task.next_fire_time || 'Suspended')}</div>
                        <div class="details-label">Trigger Spec</div>
                        <div class="details-value"><code>${escapeHtml(task.trigger || '* * * * *')}</code></div>
                        <div class="details-block-container">
                            <div class="details-label" style="margin-bottom: 6px;">Execution Content</div>
                            <div class="reply-content">${formatMarkdown(task.content || '')}</div>
                        </div>
                    </div>
                </div>
            `;
            cronListContainer.appendChild(card);
        });
    } catch (e) {
        cronListContainer.innerHTML = '<div style="padding:20px; color:var(--danger-color)">FETCH FAILED</div>';
    }
}

function toggleAddCronPanel() {
    addCronPanel.style.display = addCronPanel.style.display === 'none' ? 'flex' : 'none';
}

async function submitCronTask() {
    const name = document.getElementById('cronName').value.trim();
    const content = document.getElementById('cronContent').value.trim();
    const work_dir = document.getElementById('cronWorkDir').value.trim();

    if (!name || !content || !work_dir) {
        alert("REQUIRED FIELDS MISSING.");
        return;
    }

    const payload = {
        name, content, work_dir,
        minute: document.getElementById('cronMin').value,
        hour: document.getElementById('cronHour').value,
        day: document.getElementById('cronDay').value,
        month: document.getElementById('cronMonth').value,
        day_of_week: document.getElementById('cronWeek').value,
        second: '0'
    };

    try {
        const response = await fetch('/schedule', {
            method: 'POST',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify(payload)
        });
        if (response.ok) {
            toggleAddCronPanel();
            await fetchCronTasks();
        }
    } catch (e) {
        alert("CREATION FAULT");
    }
}

function removeCronTask(taskId, taskName) {
    showConfirmDialog({
        title: "PURGE CRON PIPELINE",
        text: `Are you sure you want to destroy scheduled process [${taskName}]? This pipeline sequence will be wiped out from kernel queue.`,
        onConfirm: async () => {
            try {
                const response = await fetch(`/schedule/${taskId}`, {method: 'DELETE'});
                if (response.ok) {
                    await fetchCronTasks();
                }
            } catch (e) {
                alert("PURGE FAILED");
            }
        }
    });
}

// ==========================================
// 9. SKILLS 核心原子能力提取层
// ==========================================
async function fetchSkillData() {
    skillListContainer.innerHTML = SKELETON_HTML;
    try {
        const response = await fetch('/skill/list');
        const skills = await response.json();
        skillListContainer.innerHTML = '';

        if (skills.length === 0) {
            skillListContainer.innerHTML = '<div style="padding:30px; text-align:center; border:1px dashed var(--border-hard); color:var(--slate-400)">NO SKILLS EXTRACTED</div>';
            return;
        }

        skills.forEach(skill => {
            const card = document.createElement('div');
            card.className = 'info-card';
            card.innerHTML = `
                <div class="info-card-summary" onclick="toggleCardOpen(this.parentNode)">
                    <div class="info-card-main">
                        ${ARROW_SVG}
                        <span class="info-card-name">${escapeHtml(skill.name || 'Unnamed Skill')}</span>
                        <span class="info-card-snippet">${escapeHtml(skill.description || '')}</span>
                    </div>
                </div>
                <div class="info-card-details" style="display: none;">
                    <div class="details-grid">
                        <div class="details-label">Description</div>
                        <div class="details-value">${escapeHtml(skill.description || 'No description assigned.')}</div>
                        <div class="details-label">FS Path</div>
                        <div class="details-value" style="font-family:var(--font-mono); font-size:12px; color:var(--slate-400)">${escapeHtml(skill.path || '')}</div>
                        <div class="details-block-container">
                            <div class="details-label" style="margin-bottom: 6px;">Source Manifest</div>
                            <div class="reply-content">${formatMarkdown(skill.content || '')}</div>
                        </div>
                    </div>
                </div>
            `;
            skillListContainer.appendChild(card);
        });
    } catch (e) {
        skillListContainer.innerHTML = '<div style="padding:20px; color:var(--danger-color)">FETCH FAILED</div>';
    }
}

// ==========================================
// 10. MCP (Model Context Protocol) 模块矩阵
// ==========================================
function toggleAddMcpPanel() {
    addMcpPanel.style.display = addMcpPanel.style.display === 'none' ? 'flex' : 'none';
}

function adaptMcpFormFields() {
    const type = document.getElementById('mcpType').value;
    document.getElementById('mcpNetworkRow').style.display = (type === 'stdio') ? 'none' : 'flex';
    document.getElementById('mcpLocalRow').style.display = (type === 'stdio') ? 'flex' : 'none';
}

async function fetchMcpRegistry() {
    mcpListContainer.innerHTML = SKELETON_HTML;
    try {
        const response = await fetch('/mcp-server/list');
        globalMcpCachedList = await response.json();
        mcpListContainer.innerHTML = '';

        if (globalMcpCachedList.length === 0) {
            mcpListContainer.innerHTML = '<div style="padding:30px; text-align:center; border:1px dashed var(--border-hard); color:var(--slate-400)">NO REGISTERED MCP CONTEXTS</div>';
            return;
        }

        globalMcpCachedList.forEach(server => {
            const name = server.name;
            const card = document.createElement('div');
            card.className = 'info-card';
            card.id = `mcp-card-${name}`;

            const snippet = server.url || (server.command ? `${server.command} ${server.args?.join(' ')}` : 'Local Context');
            const isStdio = server.type === 'stdio';

            card.innerHTML = `
                <div class="info-card-summary" onclick="toggleCardOpen(this.parentNode)">
                    <div class="info-card-main">
                        ${ARROW_SVG}
                        <span class="info-card-name" style="min-width:180px; max-width:280px;">${escapeHtml(name)}</span>
                        <span class="info-card-snippet">${escapeHtml(snippet)}</span>
                    </div>
                    <div style="display:flex; gap:8px; align-items:center;" onclick="event.stopPropagation();">
                        <button class="btn btn-sm send-button" style="height:28px; padding:0 8px; font-size:10px;" onclick="updateSingleMcp('${name}')">Save</button>
                        <button class="btn btn-sm send-button" style="height:28px;" onclick="testMcpServerTools('${name}')">Test Probe</button>
                        <button class="delete-btn" style="opacity:1; padding:6px;" onclick="removeMcpServer('${name}')">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
                        </button>
                    </div>
                </div>
                <div class="info-card-details" style="display: none;">
                    <input type="hidden" id="mcp-type-${name}" value="${escapeHtml(server.type)}">
                    <div class="details-grid">
                        <div class="details-label">Protocol Type</div>
                        <div class="details-value"><code style="background:var(--inline-code-bg); padding:2px 6px; border-radius:2px;">${escapeHtml(server.type)}</code></div>

                        <div class="details-label">Description</div>
                        <div class="details-value">
                            <input type="text" id="mcp-desc-${name}" class="form-control" value="${escapeHtml(server.description || '')}">
                        </div>

                        ${!isStdio ? `
                            <div class="details-label">Target URL</div>
                            <div class="details-value">
                                <input type="text" id="mcp-url-${name}" class="form-control mono" value="${escapeHtml(server.url || '')}">
                            </div>
                            <div class="details-label">HTTP Context Headers (JSON)</div>
                            <div class="details-value">
                                <textarea id="mcp-headers-${name}" class="form-control mono" rows="3" style="resize: vertical; font-size:11px;">${escapeHtml(JSON.stringify(server.headers || {}, null, 2))}</textarea>
                            </div>
                        ` : `
                            <div class="details-label">Command Execution</div>
                            <div class="details-value">
                                <input type="text" id="mcp-command-${name}" class="form-control mono" value="${escapeHtml(server.command || '')}">
                            </div>
                            <div class="details-label">Arguments (Comma Separated)</div>
                            <div class="details-value">
                                <input type="text" id="mcp-args-${name}" class="form-control mono" value="${escapeHtml(server.args?.join(', ') || '')}">
                            </div>
                        `}

                        <div class="details-block-container" id="mcp-tools-zone-${name}" style="display:none;">
                            <div class="details-label" style="margin-bottom: 6px;">Exposed Capabilities Registry</div>
                            <div class="mcp-tool-badge-list" id="mcp-tools-list-${name}"></div>
                        </div>
                    </div>
                </div>
            `;
            mcpListContainer.appendChild(card);
        });
    } catch (e) {
        mcpListContainer.innerHTML = '<div style="padding:20px; color:var(--danger-color)">TOPOLOGY CAPTURE CRASHED</div>';
    }
}

async function submitMcpServer() {
    const name = document.getElementById('mcpKey').value.trim();
    const description = document.getElementById('mcpDesc').value.trim();
    const type = document.getElementById('mcpType').value;
    if (!name) {
        alert("SERVER UNIQUE NAME IS REQUIRED.");
        return;
    }

    let bodyPayload = {name: name, description: description};
    if (type === 'stdio') {
        bodyPayload.command = document.getElementById('mcpCommand').value.trim();
        const argsStr = document.getElementById('mcpArgs').value.trim();
        bodyPayload.args = argsStr ? argsStr.split(',').map(a => a.trim()) : [];
    } else {
        bodyPayload.url = document.getElementById('mcpUrl').value.trim();
        const headersStr = document.getElementById('mcpHeaders').value.trim();
        if (headersStr) {
            try {
                bodyPayload.headers = JSON.parse(headersStr);
            } catch (e) {
                alert("HEADERS MUST BE A VALID JSON STRING.");
                return;
            }
        } else {
            bodyPayload.headers = {};
        }
    }

    try {
        const response = await fetch(`/mcp-server/${encodeURIComponent(type)}`, {
            method: 'POST',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify(bodyPayload)
        });
        if (response.ok) {
            toggleAddMcpPanel();
            document.getElementById('mcpKey').value = '';
            document.getElementById('mcpDesc').value = '';
            document.getElementById('mcpUrl').value = '';
            document.getElementById('mcpHeaders').value = '';
            document.getElementById('mcpCommand').value = '';
            document.getElementById('mcpArgs').value = '';
            await fetchMcpRegistry();
        }
    } catch (e) {
        alert("REGISTRATION REJECTED");
    }
}

async function updateSingleMcp(name) {
    const type = document.getElementById(`mcp-type-${name}`).value;
    const description = document.getElementById(`mcp-desc-${name}`).value.trim();
    let bodyPayload = {description: description};

    if (type === 'stdio') {
        bodyPayload.command = document.getElementById(`mcp-command-${name}`).value.trim();
        const argsStr = document.getElementById(`mcp-args-${name}`).value.trim();
        bodyPayload.args = argsStr ? argsStr.split(',').map(a => a.trim()).filter(a => a) : [];
    } else {
        bodyPayload.url = document.getElementById(`mcp-url-${name}`).value.trim();
        const headersStr = document.getElementById(`mcp-headers-${name}`).value.trim();
        if (headersStr) {
            try {
                bodyPayload.headers = JSON.parse(headersStr);
            } catch (e) {
                alert("HEADERS MUST BE A VALID JSON STRING.");
                return;
            }
        } else {
            bodyPayload.headers = {};
        }
    }

    try {
        const response = await fetch(`/mcp-server/${encodeURIComponent(name)}/${encodeURIComponent(type)}`, {
            method: 'PUT',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify(bodyPayload)
        });
        if (response.ok) {
            alert(`MCP SERVER [${name}] SYNCHRONIZED.`);
            await fetchMcpRegistry();
        }
    } catch (e) {
        alert("SYNC CRASHED");
    }
}

function removeMcpServer(name) {
    showConfirmDialog({
        title: "PURGE MCP NODE",
        text: `Are you sure you want to completely eject MCP node [${name}] from system environment? Bridge tunnels attached will be disconnected instantly.`,
        onConfirm: async () => {
            try {
                const response = await fetch(`/mcp-server/${encodeURIComponent(name)}`, {method: 'DELETE'});
                if (response.ok) {
                    await fetchMcpRegistry();
                }
            } catch (e) {
                alert("PURGE FAILURE");
            }
        }
    });
}

async function testMcpServerTools(name) {
    const targetServer = globalMcpCachedList.find(s => s.name === name);
    if (!targetServer) {
        return;
    }

    const card = document.getElementById(`mcp-card-${name}`);
    const detailsZone = document.getElementById(`mcp-tools-zone-${name}`);
    const listContainer = document.getElementById(`mcp-tools-list-${name}`);

    if (!card.hasAttribute('open')) {
        toggleCardOpen(card);
    }
    detailsZone.style.display = 'block';
    listContainer.innerHTML = SKELETON_HTML;

    try {
        const response = await fetch(`/mcp-server/${encodeURIComponent(targetServer.type)}/test`, {
            method: 'POST',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify(targetServer)
        });
        const tools = await response.json();
        listContainer.innerHTML = '';
        if (!tools || tools.length === 0) {
            listContainer.innerHTML = '<div style="font-size:12px; color:var(--slate-400); font-style:italic;">CONNECTED SUCCESSFULLY. ZERO TOOLS EXPOSED.</div>';
            return;
        }
        tools.forEach(tool => {
            const item = document.createElement('div');
            item.className = 'mcp-tool-item';
            item.innerHTML = `
                <div class="mcp-tool-name">/tools::${escapeHtml(tool.name)}</div>
                <div class="mcp-tool-desc">${escapeHtml(tool.description || 'No instruction manifest provided.')}</div>
            `;
            listContainer.appendChild(item);
        });
    } catch (e) {
        listContainer.innerHTML = `<div style="font-family:var(--font-mono); font-size:12px; color:var(--danger-color)">SESSION CRASHED: REFUSED CONNECTION</div>`;
    }
}

// ==========================================
// 11. CONFIG 全局环境核心控制台引擎
// ==========================================
function toggleAddProviderPanel() {
    addProviderPanel.style.display = addProviderPanel.style.display === 'none' ? 'flex' : 'none';
}

async function fetchGlobalSettings() {
    const providerList = document.getElementById('providerConfigList');
    providerList.innerHTML = SKELETON_HTML;

    try {
        const pResponse = await fetch('/model-provider/list');
        globalProvidersCachedList = await pResponse.json();

        let currentProviderName = null;
        try {
            const cpResponse = await fetch('/model-provider/current');
            if (cpResponse.ok) {
                const cp = await cpResponse.json();
                currentProviderName = cp.name;
            }
        } catch (e) {
        }

        let currentModelName = null;
        try {
            const cmResponse = await fetch('/model/current');
            if (cmResponse.ok) {
                const cm = await cmResponse.json();
                currentModelName = cm.model;
            }
        } catch (e) {
        }

        let thinkingValue = true;
        try {
            const tResponse = await fetch('/thinking');
            if (tResponse.ok) {
                const t = await tResponse.json();
                thinkingValue = t.thinking;
            }
        } catch (e) {
        }
        document.getElementById('configThinking').value = String(thinkingValue);

        const activeProviderSelect = document.getElementById('configActiveProvider');
        activeProviderSelect.innerHTML = '';

        globalProvidersCachedList.forEach(provider => {
            const opt = document.createElement('option');
            opt.value = provider.name;
            opt.textContent = provider.name;
            if (provider.name === currentProviderName) {
                opt.selected = true;
            }
            activeProviderSelect.appendChild(opt);
        });

        syncModelDropdown(currentModelName);
        renderProvidersFormList();

    } catch (e) {
        providerList.innerHTML = '<div style="padding:20px; color:var(--danger-color)">SETTINGS DESERIALIZATION FAULT</div>';
    }
}

function renderProvidersFormList() {
    const providerList = document.getElementById('providerConfigList');
    providerList.innerHTML = '';

    globalProvidersCachedList.forEach(provider => {
        const name = provider.name;
        const card = document.createElement('div');
        card.className = 'info-card';
        card.id = `provider-card-${name}`;

        const snippet = provider.base_url || 'Endpoint missing';

        card.innerHTML = `
            <div class="info-card-summary" onclick="toggleCardOpen(this.parentNode)">
                <div class="info-card-main">
                    ${ARROW_SVG}
                    <span class="info-card-name" style="min-width:180px; max-width:280px;">${escapeHtml(name)}</span>
                    <span class="info-card-snippet">${escapeHtml(snippet)}</span>
                </div>
                <div style="display:flex; gap:12px; align-items:center;" onclick="event.stopPropagation();">
                    <button class="btn btn-sm send-button" style="height:28px; padding:0 8px; font-size:10px;" onclick="updateSingleProvider('${name}')">Save</button>
                    <button class="delete-btn" style="opacity:1; color:var(--danger-color); font-size:11px; font-family:var(--font-mono); font-weight:700; padding:6px;" onclick="removeModelProvider('${name}')">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
                    </button>
                </div>
            </div>
            <div class="info-card-details" style="display: none; background: var(--bg-surface); border-top: 1px solid var(--border-hard);">
                <div class="form-row" style="margin-bottom: 12px;">
                    <div class="form-group" style="flex: 2;">
                        <label style="font-family: var(--font-display); font-size: 11px; font-weight: 700; text-transform: uppercase; color: var(--charcoal-700);">Base URL Endpoint</label>
                        <input type="text" id="provider-url-${name}" class="form-control mono" value="${escapeHtml(provider.base_url || '')}">
                    </div>
                    <div class="form-group" style="flex: 1;">
                        <label style="font-family: var(--font-display); font-size: 11px; font-weight: 700; text-transform: uppercase; color: var(--charcoal-700);">Secret Token (API Key)</label>
                        <input type="password" id="provider-key-${name}" class="form-control mono" value="${escapeHtml(provider.api_key || '')}" placeholder="••••••••••••">
                    </div>
                </div>
                <div class="form-row">
                    <div class="form-group">
                        <label style="font-family: var(--font-display); font-size: 11px; font-weight: 700; text-transform: uppercase; color: var(--charcoal-700);">Engine Clusters (Comma Separated Models)</label>
                        <input type="text" id="provider-models-${name}" class="form-control mono" value="${escapeHtml(provider.models?.join(', ') || '')}">
                    </div>
                </div>
            </div>
        `;
        providerList.appendChild(card);
    });
}

function syncModelDropdown(targetSelectedModel = null) {
    const activeProviderSelect = document.getElementById('configActiveProvider');
    const modelSelect = document.getElementById('configActiveModel');
    modelSelect.innerHTML = '';

    if (activeProviderSelect.options.length === 0) {
        return;
    }
    const currentProviderName = activeProviderSelect.value;

    const targetProvider = globalProvidersCachedList.find(p => p.name === currentProviderName);
    if (targetProvider && targetProvider.models) {
        targetProvider.models.forEach(modelStr => {
            const opt = document.createElement('option');
            opt.value = modelStr;
            opt.textContent = modelStr;
            if (targetSelectedModel && modelStr === targetSelectedModel) {
                opt.selected = true;
            }
            modelSelect.appendChild(opt);
        });
    }
}

async function submitNewProvider() {
    const name = document.getElementById('newProviderName').value.trim();
    if (!name) {
        alert("PROVIDER NAME IS REQUIRED.");
        return;
    }

    const urlValue = document.getElementById('newProviderUrl').value.trim();
    const keyValue = document.getElementById('newProviderKey').value.trim();
    const modelsValue = document.getElementById('newProviderModels').value.trim();

    const bodyPayload = {
        name: name,
        base_url: urlValue,
        api_key: keyValue,
        models: modelsValue ? modelsValue.split(',').map(m => m.trim()).filter(m => m) : []
    };

    try {
        const response = await fetch('/model-provider', {
            method: 'POST',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify(bodyPayload)
        });
        if (response.ok) {
            document.getElementById('newProviderName').value = '';
            document.getElementById('newProviderUrl').value = '';
            document.getElementById('newProviderKey').value = '';
            document.getElementById('newProviderModels').value = '';
            toggleAddProviderPanel();
            await fetchGlobalSettings();
        }
    } catch (e) {
        alert("INJECTION FAULT");
    }
}

async function updateSingleProvider(name) {
    const urlValue = document.getElementById(`provider-url-${name}`).value.trim();
    const keyValue = document.getElementById(`provider-key-${name}`).value.trim();
    const modelsValue = document.getElementById(`provider-models-${name}`).value.trim();

    const bodyPayload = {
        base_url: urlValue,
        api_key: keyValue,
        models: modelsValue ? modelsValue.split(',').map(m => m.trim()).filter(m => m) : []
    };

    try {
        const response = await fetch(`/model-provider/${encodeURIComponent(name)}`, {
            method: 'PUT',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify(bodyPayload)
        });
        if (response.ok) {
            alert(`PROVIDER CLUSTER [${name}] SYNCHRONIZED.`);
            const pResponse = await fetch('/model-provider/list');
            globalProvidersCachedList = await pResponse.json();
            syncModelDropdown(document.getElementById('configActiveModel').value);
        }
    } catch (e) {
        alert("SYNC CRASHED");
    }
}

function removeModelProvider(name) {
    showConfirmDialog({
        title: "PURGE MODEL PROVIDER",
        text: `Are you sure you want to completely remove infrastructure cluster [${name}]? Layer dependencies pointing here will collapse.`,
        onConfirm: async () => {
            try {
                const response = await fetch(`/model-provider/${encodeURIComponent(name)}`, {method: 'DELETE'});
                if (response.ok) {
                    await fetchGlobalSettings();
                }
            } catch (e) {
                alert("PURGE FAILURE");
            }
        }
    });
}

async function saveActiveModelRoute() {
    const providerSelect = document.getElementById('configActiveProvider');
    const modelSelect = document.getElementById('configActiveModel');

    if (!providerSelect.value || !modelSelect.value) {
        alert("ROUTING PATH IS INCOMPLETE.");
        return;
    }

    try {
        let response = await fetch('/model-provider/current', {
            method: 'PUT',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({name: providerSelect.value})
        });
        if (!response.ok) {
            alert("PROVIDER ROUTING FAILED");
            return;
        }
        response = await fetch('/model/current', {
            method: 'PUT',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({model: modelSelect.value})
        });
        if (!response.ok) {
            alert("MODEL ROUTING FAILED");
            return;
        }
        response = await fetch('/thinking', {
            method: 'PUT',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({thinking: document.getElementById('configThinking').value === 'true'})
        });
        if (response.ok) {
            alert("GLOBAL CONTEXT ACTIVE ROUTE MOUNTED.");
        }
    } catch (e) {
        alert("ROUTING COMMIT FAULT");
    }
}

