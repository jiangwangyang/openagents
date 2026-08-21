// ==========================================
// CRON 定时任务面板
// ==========================================

// ===== 1. DOM 节点缓存 =====
const cronListContainer = document.getElementById('cronListContainer');
const addCronPanel = document.getElementById('addCronPanel');

// ===== 2. 新增面板与表单辅助 =====
async function toggleAddCronPanel() {
    const isOpening = addCronPanel.style.display === 'none';
    addCronPanel.style.display = isOpening ? 'flex' : 'none';
    if (isOpening) {
        await resetCronForm();
        loadCronAgentOptions();
    }
}

// 重置 cron 新增表单
async function resetCronForm() {
    document.getElementById('cronName').value = '';
    document.getElementById('cronContent').value = '';
    document.getElementById('cronAgentId').value = '';
    document.getElementById('cronMin').value = '0';
    document.getElementById('cronHour').value = '9';
    document.getElementById('cronDay').value = '*';
    document.getElementById('cronMonth').value = '*';
    document.getElementById('cronWeek').value = '*';
    // 工作目录默认填入对话页当前目录, 未设置时从后端拉取默认目录
    const workdir = await resolveDefaultWorkdir();
    const display = document.getElementById('cronWorkspaceDisplay');
    display.textContent = workdir || t('input.unset');
    display.title = workdir || '';
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
        day_of_week: parts[5] || '*'
    };
}

// 组装定时任务提交载荷: 校验必填项与 cron 字段格式, 失败时提示并返回 null
function buildCronPayload(name, content, workDir, agentIdVal, cronFieldValues, second, enabled) {
    if (!name || !content || !workDir || !agentIdVal) {
        showToast(t('common.requiredMissing'), 'error');
        return null;
    }
    // 简单格式校验: cron 字段仅允许数字与 * , - / 符号
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

// ===== 3. 任务列表 =====
async function fetchCronTasks() {
    cronListContainer.innerHTML = SKELETON_HTML;
    try {
        const [taskResponse, agentResponse] = await Promise.all([fetch('/schedule/list'), fetch('/agent/list')]);
        const tasks = await taskResponse.json();
        const agents = await agentResponse.json();
        cronListContainer.innerHTML = '';

        if (tasks.length === 0) {
            cronListContainer.innerHTML = emptyListHtml('cron.empty', 'cron.emptyHint');
            return;
        }

        tasks.forEach(task => {
            const cron = parseCronExpr(task.trigger);
            // 解析 Agent 名称用于只读展示, 花名册中缺失时回退为 common.none
            const boundAgent = agents.find(agent => agent.id === task.agent_id);
            const agentName = boundAgent ? boundAgent.name : '';
            const enabledText = task.enabled ? t('cron.enabled') : t('cron.disabled');
            const enabledColor = task.enabled ? 'var(--success-color)' : 'var(--slate-400)';
            const card = document.createElement('div');
            card.className = 'info-card';
            card.innerHTML = `
                <div class="info-card-summary" onclick="toggleCronCard(this.parentNode, ${task.id})">
                    <div class="info-card-main">
                        ${ARROW_SVG}
                        <span class="info-card-name card-name-fixed">${escapeHtml(task.name || t('cron.unnamed'))}</span>
                        <span class="info-card-snippet">${escapeHtml(task.content || '')}</span>
                        <span class="card-meta card-meta-gap" style="color: ${enabledColor};">${enabledText}</span>
                        <span class="card-meta card-meta-gap" title="${t('cron.triggerSpec')}">${escapeHtml(`${cron.minute} ${cron.hour} ${cron.day} ${cron.month} ${cron.day_of_week}`)}</span>
                        <span class="card-meta">${t('cron.nextFire')}: ${escapeHtml(task.next_fire_time || t('cron.suspended'))}</span>
                    </div>
                    <div class="card-actions" onclick="event.stopPropagation();">
                        <button class="btn btn-sm btn-secondary cron-run-btn btn-card-sm">${t('cron.triggerNow')}</button>
                        <button class="btn btn-sm btn-secondary cron-toggle-btn btn-card-sm">${task.enabled ? t('cron.disable') : t('cron.enable')}</button>
                        <button class="delete-btn always-visible">${DELETE_SVG}</button>
                    </div>
                </div>
                <div class="info-card-details" style="display: none;">
                    <div class="details-grid">
                        <div class="details-label">${t('cron.name')}</div>
                        <div class="details-value">${escapeHtml(task.name || t('cron.unnamed'))}</div>
                        <div class="details-label">${t('cron.workingDir')}</div>
                        <div class="details-value">${escapeHtml(task.work_dir || t('common.inheritedEnv'))}</div>
                        <div class="details-label">${t('cron.agent')}</div>
                        <div class="details-value">${escapeHtml(agentName) || t('common.none')}</div>
                        <div class="details-label">${t('cron.triggerSpec')}</div>
                        <div class="details-value">${escapeHtml(`${cron.minute} ${cron.hour} ${cron.day} ${cron.month} ${cron.day_of_week}`)}</div>
                        <div class="details-label">${t('cron.nextFire')}</div>
                        <div class="details-value">${escapeHtml(task.next_fire_time || t('cron.suspended'))}</div>
                        <div class="details-label">${t('cron.execContent')}</div>
                        <div class="details-value" style="white-space: pre-wrap;">${escapeHtml(task.content || '')}</div>
                    </div>
                    <div class="details-block-container" style="margin-top: 12px;">
                        <div class="details-block-header">
                            <div class="details-label">${t('cron.execHistory')}</div>
                        </div>
                        <div class="task-stage-list${stageSnippetExpanded ? ' snippet-expanded' : ''}" id="cron-stages-${task.id}"></div>
                    </div>
                </div>
            `;
            // 执行记录标题栏追加排序按钮(升/降序切换并持久化记忆)
            card.querySelector('.details-block-header').appendChild(createStageSortButton());
            card.querySelector('.details-block-header').appendChild(createStageExpandButton());
            // 手动触发按钮
            card.querySelector('.cron-run-btn').onclick = () => triggerCronTask(task);
            // 启用/禁用切换按钮
            card.querySelector('.cron-toggle-btn').onclick = () => toggleCronEnabled(task);
            // 删除按钮通过闭包绑定, 避免任务名中的引号破坏内联 onclick 字符串
            const deleteBtn = card.querySelector('.delete-btn');
            deleteBtn.title = t('common.purge');
            deleteBtn.onclick = (event) => {
                event.stopPropagation();
                removeCronTask(task.id, task.name);
            };
            cronListContainer.appendChild(card);
        });
    } catch (e) {
        cronListContainer.innerHTML = errorListHtml('common.fetchFailed');
    }
}

// ===== 4. 执行记录 =====
// 展开 cron 卡片时加载定时任务详情(执行对话记录)
function toggleCronCard(cardElement, scheduleId) {
    toggleCardOpen(cardElement);
    if (cardElement.hasAttribute('open')) {
        loadCronDetail(scheduleId);
    }
}

// 加载定时任务详情: 渲染各次执行对话的最后一条消息
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
        // 按当前排序方向渲染(升/降序由排序按钮切换并持久化记忆)
        sortedStageConversations(schedule.conversations).forEach(conversation => {
            stageList.appendChild(createStageRecordItem(conversation));
        });
    } catch (e) {
        stageList.innerHTML = `<div class="text-error-mono">${t('common.fetchFailed')}</div>`;
    }
}

// ===== 5. 新增任务 =====
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
    // 新增默认启用, enabled 固定传 true
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

// ===== 6. 启用/禁用切换 =====
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
        enabled: !task.enabled
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

// ===== 7. 手动触发 =====
// 手动触发定时任务: 立即执行一次, 不影响原调度计划
async function triggerCronTask(task) {
    try {
        const response = await fetch(`/schedule/${task.id}/trigger`, {method: 'POST'});
        if (response.ok) {
            showToast(t('cron.triggered', {name: task.name}));
            // 卡片处于展开状态时刷新执行记录
            const stageList = document.getElementById(`cron-stages-${task.id}`);
            if (stageList && stageList.closest('.info-card').hasAttribute('open')) {
                loadCronDetail(task.id);
            }
        }
    } catch (e) {
        showToast(t('common.syncCrashed'), 'error');
    }
}

// ===== 8. 删除任务 =====
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
