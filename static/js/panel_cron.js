// ==========================================
// CRON 核心排程自动化面板
// ==========================================
const cronListContainer = document.getElementById('cronListContainer');
const addCronPanel = document.getElementById('addCronPanel');
// 最近一次拉取的定时任务列表缓存，供保存时按 id 读取 enabled 等未编辑字段
let cronTasksCache = [];

async function loadCronAgentOptions() {
    try {
        const response = await fetch('/agent/list');
        const agents = await response.json();
        const select = document.getElementById('cronAgentId');
        select.innerHTML = '';
        agents.forEach(agent => {
            const option = document.createElement('option');
            option.value = agent.id;
            option.textContent = agent.name;
            select.appendChild(option);
        });
    } catch (e) {}
}

// 打开目录选择弹窗，选中后写入 cron 面板的目录显示
function selectCronWorkspace() {
    openDirModal((path) => {
        const display = document.getElementById('cronWorkspaceDisplay');
        display.textContent = path || t('input.unset');
        display.title = path;
    }, document.getElementById('cronWorkspaceDisplay').title || '');
}

// 打开目录选择弹窗，选中后写入指定 cron 卡片的工作目录显示
function selectCronCardWorkspace(taskId) {
    openDirModal((path) => {
        const display = document.getElementById(`cron-workdir-display-${taskId}`);
        display.textContent = path || t('input.unset');
        display.title = path;
    }, document.getElementById(`cron-workdir-display-${taskId}`).title || '');
}

// 解析后端 6 段 cron 表达式到各个字段
function parseCronExpr(expr) {
    const parts = (expr || '').trim().split(/\s+/);
    return {
        second: parts[0] || '0',
        minute: parts[1] || '*',
        hour: parts[2] || '*',
        day: parts[3] || '*',
        month: parts[4] || '*',
        day_of_week: parts[5] || '*',
    };
}

// 重置 cron 新增表单
function resetCronForm() {
    document.getElementById('cronName').value = '';
    document.getElementById('cronContent').value = '';
    document.getElementById('cronAgentId').value = '';
    document.getElementById('cronMin').value = '0';
    document.getElementById('cronHour').value = '9';
    document.getElementById('cronDay').value = '*';
    document.getElementById('cronMonth').value = '*';
    document.getElementById('cronWeek').value = '*';
    const display = document.getElementById('cronWorkspaceDisplay');
    display.textContent = currentWorkdir || t('input.unset');
    display.title = currentWorkdir || '';
}

async function fetchCronTasks() {
    cronListContainer.innerHTML = SKELETON_HTML;
    try {
        const [taskResponse, agentResponse] = await Promise.all([fetch('/schedule/list'), fetch('/agent/list')]);
        const tasks = await taskResponse.json();
        const agents = await agentResponse.json();
        cronTasksCache = tasks;
        cronListContainer.innerHTML = '';

        if (tasks.length === 0) {
            cronListContainer.innerHTML = emptyListHtml('cron.empty');
            return;
        }

        tasks.forEach(task => {
            const cron = parseCronExpr(task.trigger);
            const enabledText = task.enabled ? t('cron.enabled') : t('cron.disabled');
            const enabledColor = task.enabled ? 'var(--success-color, #22c55e)' : 'var(--slate-400)';
            const card = document.createElement('div');
            card.className = 'info-card';
            card.id = `cron-card-${task.id}`;
            card.innerHTML = `
                <div class="info-card-summary" onclick="toggleCardOpen(this.parentNode)">
                    <div class="info-card-main">
                        ${ARROW_SVG}
                        <span class="info-card-name" style="min-width:180px; max-width:280px;">${escapeHtml(task.name || t('cron.unnamed'))}</span>
                        <span class="info-card-snippet">${escapeHtml(task.content || '')}</span>
                        <span style="font-size:10px; font-family:var(--font-mono); color:${enabledColor}; margin-left:8px; flex-shrink:0;">${enabledText}</span>
                    </div>
                    <div style="display:flex; gap:4px; align-items:center;" onclick="event.stopPropagation();">
                        <button class="btn btn-sm btn-secondary cron-toggle-btn" style="height:28px; padding:0 8px; font-size:10px;">${task.enabled ? t('cron.disable') : t('cron.enable')}</button>
                        <button class="btn btn-sm send-button cron-save-btn" style="height:28px; padding:0 8px; font-size:10px;">${t('common.save')}</button>
                        <button class="delete-btn" style="opacity:1; padding:6px;">${DELETE_SVG}</button>
                    </div>
                </div>
                <div class="info-card-details" style="display: none;">
                    <div class="form-row">
                        <div class="form-group">
                            <label>${t('cron.name')}</label>
                            <input type="text" id="cron-name-${task.id}" class="form-control" value="${escapeHtml(task.name || '')}">
                        </div>
                        <div class="form-group">
                            <label>${t('cron.workingDir')}</label>
                            <button class="workspace-btn" style="margin-bottom:0;" title="Set Directory Context" onclick="selectCronCardWorkspace(${task.id})">
                                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                    <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
                                </svg>
                                <span>CWD:</span>
                                <span class="workspace-path" id="cron-workdir-display-${task.id}" title="${escapeHtml(task.work_dir || currentWorkdir || '')}">${escapeHtml(task.work_dir || currentWorkdir || t('input.unset'))}</span>
                            </button>
                        </div>
                    </div>
                    <div class="form-row">
                        <div class="form-group"><label>${t('cron.agent')}</label><select id="cron-agent-${task.id}" class="form-control"></select></div>
                        <div class="form-group"><label>${t('cron.minute')}</label><input type="text" id="cron-min-${task.id}" class="form-control mono" value="${escapeHtml(cron.minute)}"></div>
                        <div class="form-group"><label>${t('cron.hour')}</label><input type="text" id="cron-hour-${task.id}" class="form-control mono" value="${escapeHtml(cron.hour)}"></div>
                        <div class="form-group"><label>${t('cron.day')}</label><input type="text" id="cron-day-${task.id}" class="form-control mono" value="${escapeHtml(cron.day)}"></div>
                        <div class="form-group"><label>${t('cron.month')}</label><input type="text" id="cron-month-${task.id}" class="form-control mono" value="${escapeHtml(cron.month)}"></div>
                        <div class="form-group"><label>${t('cron.week')}</label><input type="text" id="cron-week-${task.id}" class="form-control mono" value="${escapeHtml(cron.day_of_week)}"></div>
                    </div>
                    <div class="form-row">
                        <div class="form-group">
                            <label>${t('cron.execContent')}</label>
                            <textarea id="cron-content-${task.id}" class="form-control mono" rows="5" style="resize: vertical; font-size:11px;">${escapeHtml(task.content || '')}</textarea>
                        </div>
                    </div>
                    <div class="form-row" style="display:flex; align-items:center; gap:12px;">
                        <div style="flex: 0 0 auto;">
                            <label style="font-family:var(--font-display); font-size:11px; font-weight:700; text-transform:uppercase; letter-spacing:0.5px; color:var(--slate-400);">${t('cron.nextFire')}</label>
                        </div>
                        <div style="font-family:var(--font-mono); font-size:12px; color:var(--charcoal-800);">${escapeHtml(task.next_fire_time || t('cron.suspended'))}</div>
                    </div>
                </div>
            `;
            // 保存按钮
            card.querySelector('.cron-save-btn').onclick = () => updateSingleCron(task.id);
            // 启用/禁用切换按钮
            card.querySelector('.cron-toggle-btn').onclick = () => toggleCronEnabled(task);
            // 删除按钮通过闭包绑定，避免任务名中的引号破坏内联 onclick 字符串
            card.querySelector('.delete-btn').onclick = (event) => {
                event.stopPropagation();
                removeCronTask(task.id, task.name);
            };
            cronListContainer.appendChild(card);
            // 异步填充 agent 下拉并选中当前值
            loadCronCardAgentOptions(`cron-agent-${task.id}`, task.agent_id, agents);
        });
    } catch (e) {
        cronListContainer.innerHTML = errorListHtml('common.fetchFailed');
    }
}

// 填充 cron 卡片内的 agent 下拉选项
function loadCronCardAgentOptions(elementId, selectedId, agents) {
    const select = document.getElementById(elementId);
    select.innerHTML = '';
    agents.forEach(agent => {
        const opt = document.createElement('option');
        opt.value = agent.id;
        opt.textContent = agent.name;
        if (String(agent.id) === String(selectedId)) {
            opt.selected = true;
        }
        select.appendChild(opt);
    });
}

function toggleAddCronPanel() {
    const isOpening = addCronPanel.style.display === 'none';
    addCronPanel.style.display = isOpening ? 'flex' : 'none';
    if (isOpening) {
        resetCronForm();
        loadCronAgentOptions();
    }
}

// 切换定时任务启用/禁用状态
async function toggleCronEnabled(task) {
    const cron = parseCronExpr(task.trigger);
    const payload = {
        name: task.name,
        content: task.content,
        work_dir: task.work_dir,
        minute: cron.minute,
        hour: cron.hour,
        day: cron.day,
        month: cron.month,
        day_of_week: cron.day_of_week,
        second: cron.second,
        agent_id: task.agent_id,
        enabled: !task.enabled,
    };
    try {
        const response = await fetch(`/schedule/${task.id}`, {
            method: 'PUT',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify(payload)
        });
        if (response.ok) {
            await fetchCronTasks();
        }
    } catch (e) {
        alert(t('common.syncCrashed'));
    }
}

// 保存单个定时任务的编辑
async function updateSingleCron(taskId) {
    const name = document.getElementById(`cron-name-${taskId}`).value.trim();
    const content = document.getElementById(`cron-content-${taskId}`).value.trim();
    const work_dir = document.getElementById(`cron-workdir-display-${taskId}`).title || '';
    const agentIdVal = document.getElementById(`cron-agent-${taskId}`).value;
    if (!name || !content || !work_dir || !agentIdVal) {
        alert(t('common.requiredMissing'));
        return;
    }
    // 从缓存列表中获取当前 enabled 状态（该字段不在编辑表单内）
    const existing = cronTasksCache.find(taskItem => taskItem.id === taskId);
    const enabled = existing ? existing.enabled : true;

    const payload = {
        name, content, work_dir,
        minute: document.getElementById(`cron-min-${taskId}`).value,
        hour: document.getElementById(`cron-hour-${taskId}`).value,
        day: document.getElementById(`cron-day-${taskId}`).value,
        month: document.getElementById(`cron-month-${taskId}`).value,
        day_of_week: document.getElementById(`cron-week-${taskId}`).value,
        second: '0',
        agent_id: parseInt(agentIdVal),
        enabled: enabled,
    };
    try {
        const response = await fetch(`/schedule/${taskId}`, {
            method: 'PUT',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify(payload)
        });
        if (response.ok) {
            alert(t('cron.synced', {name: name}));
            await fetchCronTasks();
        }
    } catch (e) {
        alert(t('common.syncCrashed'));
    }
}

async function submitCronTask() {
    const name = document.getElementById('cronName').value.trim();
    const content = document.getElementById('cronContent').value.trim();
    const work_dir = document.getElementById('cronWorkspaceDisplay').title || '';

    const agentIdVal = document.getElementById('cronAgentId').value;
    if (!name || !content || !work_dir || !agentIdVal) {
        alert(t('common.requiredMissing'));
        return;
    }
    const payload = {
        name, content, work_dir,
        minute: document.getElementById('cronMin').value,
        hour: document.getElementById('cronHour').value,
        day: document.getElementById('cronDay').value,
        month: document.getElementById('cronMonth').value,
        day_of_week: document.getElementById('cronWeek').value,
        second: '0',
        agent_id: parseInt(agentIdVal)
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
        alert(t('common.creationFault'));
    }
}

function removeCronTask(taskId, taskName) {
    showConfirmDialog({
        title: t('cron.purgeTitle'),
        text: t('cron.purgeText', {name: taskName}),
        onConfirm: async () => {
            try {
                const response = await fetch(`/schedule/${taskId}`, {method: 'DELETE'});
                if (response.ok) {
                    await fetchCronTasks();
                }
            } catch (e) {
                alert(t('cron.purgeFailed'));
            }
        }
    });
}
