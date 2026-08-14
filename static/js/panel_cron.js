// ==========================================
// CRON 定时任务面板
// ==========================================
const cronListContainer = document.getElementById('cronListContainer');
const addCronPanel = document.getElementById('addCronPanel');
// 最近一次拉取的定时任务列表缓存，供保存时按 id 读取 enabled 等未编辑字段
let cronTasksCache = [];

function toggleAddCronPanel() {
    const isOpening = addCronPanel.style.display === 'none';
    addCronPanel.style.display = isOpening ? 'flex' : 'none';
    if (isOpening) {
        resetCronForm();
        loadCronAgentOptions();
    }
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

async function loadCronAgentOptions() {
    try {
        const response = await fetch('/agent/list');
        const agents = await response.json();
        fillSelectOptions(document.getElementById('cronAgentId'), agents, null);
    } catch (e) {
        // 静默处理错误
    }
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

// 组装定时任务提交载荷：校验必填项与 cron 字段格式，失败时提示并返回 null
function buildCronPayload(name, content, workDir, agentIdVal, cronFieldValues, second, enabled) {
    if (!name || !content || !workDir || !agentIdVal) {
        showToast(t('common.requiredMissing'), 'error');
        return null;
    }
    // 简单格式校验：cron 字段仅允许数字与 * , - / 符号
    if (cronFieldValues.some(fieldValue => !/^[0-9*,\/\-]+$/.test(fieldValue.trim()))) {
        showToast(t('cron.invalidFormat'), 'error');
        return null;
    }
    return {
        name: name,
        content: content,
        work_dir: workDir,
        minute: cronFieldValues[0],
        hour: cronFieldValues[1],
        day: cronFieldValues[2],
        month: cronFieldValues[3],
        day_of_week: cronFieldValues[4],
        second: second,
        agent_id: parseInt(agentIdVal),
        enabled: enabled
    };
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
            cronListContainer.innerHTML = emptyListHtml('cron.empty', 'cron.emptyHint');
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
                <div class="info-card-summary" onclick="toggleCronCard(this.parentNode, ${task.id})">
                    <div class="info-card-main">
                        ${ARROW_SVG}
                        <span class="info-card-name card-name-fixed">${escapeHtml(task.name || t('cron.unnamed'))}</span>
                        <span class="info-card-snippet">${escapeHtml(task.content || '')}</span>
                        <span class="card-meta card-meta-gap" style="color:${enabledColor};">${enabledText}</span>
                        <span class="card-meta card-meta-gap" title="${t('cron.triggerSpec')}">${escapeHtml(`${cron.minute} ${cron.hour} ${cron.day} ${cron.month} ${cron.day_of_week}`)}</span>
                        <span class="card-meta">${t('cron.nextFire')}: ${escapeHtml(task.next_fire_time || t('cron.suspended'))}</span>
                    </div>
                    <div class="card-actions" style="gap:4px;" onclick="event.stopPropagation();">
                        <button class="btn btn-sm btn-secondary cron-toggle-btn btn-card-sm">${task.enabled ? t('cron.disable') : t('cron.enable')}</button>
                        <button class="btn btn-sm send-button cron-save-btn btn-card-sm">${t('common.save')}</button>
                        <button class="delete-btn always-visible">${DELETE_SVG}</button>
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
                            <button class="workspace-btn flush" title="Set Directory Context" onclick="selectPanelWorkspace('cron-workdir-display-${task.id}')">
                                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                    <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
                                </svg>
                                <span>${t('input.cwdLabel')}</span>
                                <span class="workspace-path" id="cron-workdir-display-${task.id}" title="${escapeHtml(task.work_dir || '')}">${escapeHtml(task.work_dir || t('input.unset'))}</span>
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
                    <div class="form-hint">${t('cron.formatHint')}</div>
                    <div class="form-row">
                        <div class="form-group">
                            <label>${t('cron.execContent')}</label>
                            <textarea id="cron-content-${task.id}" class="form-control mono textarea-sm" rows="5">${escapeHtml(task.content || '')}</textarea>
                        </div>
                    </div>
                    <div class="form-row align-center">
                        <label class="nextfire-label">${t('cron.nextFire')}</label>
                        <div class="nextfire-value">${escapeHtml(task.next_fire_time || t('cron.suspended'))}</div>
                    </div>
                    <div class="details-block-container" style="margin-top:12px;">
                        <div class="details-block-header">
                            <div class="details-label">${t('cron.execHistory')}</div>
                            <button class="btn btn-sm btn-secondary btn-card-xs" onclick="loadCronDetail(${task.id})">${t('common.refresh')}</button>
                        </div>
                        <div class="task-stage-list" id="cron-stages-${task.id}"></div>
                    </div>
                </div>
            `;
            // 保存按钮
            card.querySelector('.cron-save-btn').onclick = () => updateSingleCron(task.id);
            // 启用/禁用切换按钮
            card.querySelector('.cron-toggle-btn').onclick = () => toggleCronEnabled(task);
            // 删除按钮通过闭包绑定，避免任务名中的引号破坏内联 onclick 字符串
            const deleteBtn = card.querySelector('.delete-btn');
            deleteBtn.title = t('common.purge');
            deleteBtn.onclick = (event) => {
                event.stopPropagation();
                removeCronTask(task.id, task.name);
            };
            cronListContainer.appendChild(card);
            // 填充 agent 下拉并选中当前值
            fillSelectOptions(document.getElementById(`cron-agent-${task.id}`), agents, task.agent_id);
        });
    } catch (e) {
        cronListContainer.innerHTML = errorListHtml('common.fetchFailed');
    }
}

// 展开 cron 卡片时加载定时任务详情（执行对话记录）
function toggleCronCard(cardElement, scheduleId) {
    toggleCardOpen(cardElement);
    if (cardElement.hasAttribute('open')) {
        loadCronDetail(scheduleId);
    }
}

// 加载定时任务详情：渲染各次执行对话的最后一条消息
async function loadCronDetail(scheduleId) {
    const stageList = document.getElementById(`cron-stages-${scheduleId}`);
    stageList.innerHTML = SKELETON_HTML;
    try {
        const response = await fetch(`/schedule/${scheduleId}`);
        if (!response.ok) {
            stageList.innerHTML = `<div class="text-error-mono">${t('common.fetchFailed')}</div>`;
            return;
        }
        const schedule = await response.json();
        stageList.innerHTML = '';
        if (!schedule.conversations || schedule.conversations.length === 0) {
            stageList.innerHTML = `<div class="text-hint">${t('cron.notTriggered')}</div>`;
            return;
        }
        schedule.conversations.forEach(conversation => {
            // 点击执行记录项复用对话页右侧展示区：切换视图并流式回放/跟随该次执行对话（只读，禁止发送消息）
            stageList.appendChild(createStageRecordItem(conversation));
        });
    } catch (e) {
        stageList.innerHTML = `<div class="text-error-mono">${t('common.fetchFailed')}</div>`;
    }
}

async function submitCronTask() {
    const name = document.getElementById('cronName').value.trim();
    const content = document.getElementById('cronContent').value.trim();
    const workDir = document.getElementById('cronWorkspaceDisplay').title || '';
    const agentIdVal = document.getElementById('cronAgentId').value;
    const cronFieldValues = [
        document.getElementById('cronMin').value,
        document.getElementById('cronHour').value,
        document.getElementById('cronDay').value,
        document.getElementById('cronMonth').value,
        document.getElementById('cronWeek').value
    ];
    // 新增时后端忽略 enabled 字段，默认启用，但接口要求必传
    const payload = buildCronPayload(name, content, workDir, agentIdVal, cronFieldValues, '0', true);
    if (!payload) {
        return;
    }
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
        showToast(t('common.creationFault'), 'error');
    }
}

// 保存单个定时任务的编辑
async function updateSingleCron(taskId) {
    const name = document.getElementById(`cron-name-${taskId}`).value.trim();
    const content = document.getElementById(`cron-content-${taskId}`).value.trim();
    const workDir = document.getElementById(`cron-workdir-display-${taskId}`).title || '';
    const agentIdVal = document.getElementById(`cron-agent-${taskId}`).value;
    const cronFieldValues = [
        document.getElementById(`cron-min-${taskId}`).value,
        document.getElementById(`cron-hour-${taskId}`).value,
        document.getElementById(`cron-day-${taskId}`).value,
        document.getElementById(`cron-month-${taskId}`).value,
        document.getElementById(`cron-week-${taskId}`).value
    ];
    // 从缓存列表中获取 enabled 状态与秒字段（均不在编辑表单内），避免保存时被重置
    const existing = cronTasksCache.find(taskItem => taskItem.id === taskId);
    const enabled = existing ? existing.enabled : true;
    const second = existing ? parseCronExpr(existing.trigger).second : '0';
    const payload = buildCronPayload(name, content, workDir, agentIdVal, cronFieldValues, second, enabled);
    if (!payload) {
        return;
    }
    try {
        const response = await fetch(`/schedule/${taskId}`, {
            method: 'PUT',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify(payload)
        });
        if (response.ok) {
            showToast(t('cron.synced', {name: name}));
            await fetchCronTasks();
        }
    } catch (e) {
        showToast(t('common.syncCrashed'), 'error');
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
        showToast(t('common.syncCrashed'), 'error');
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
                showToast(t('common.purgeFailure'), 'error');
            }
        }
    });
}
