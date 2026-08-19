// ==========================================
// TASK 任务流水线管理面板
// ==========================================

// ===== 1. DOM 节点缓存 =====
const taskListContainer = document.getElementById('taskListContainer');
const addTaskPanel = document.getElementById('addTaskPanel');

// ===== 2. 运行时状态 =====
// 任务状态常量：待启动/运行中/待审核/已完成/运行失败
const TASK_STATUS = {IDLE: 'idle', RUNNING: 'running', REVIEW: 'review', DONE: 'done', FAILED: 'failed'};
// 每个任务的跟随控制器：状态推导、双模式更新（收起轮询/展开 SSE）与渲染载体
const taskControllers = {};
// Agent 花名册缓存：候选 Agent 下拉与名称解析共用
let taskAgentRoster = [];
// 收起态轮询间隔（毫秒）
const TASK_POLL_INTERVAL = 5000;
// 流关闭后的单次复核延迟（毫秒）：覆盖任务循环交接的毫秒级间隙
const TASK_RECHECK_DELAY = 1500;
// 启动后的补拉延迟（毫秒）：阶段对话由后台异步创建，立即拉取可能尚未出现
const TASK_START_REFETCH_DELAY = 1200;

// ===== 3. 新增面板与表单辅助 =====
async function toggleAddTaskPanel() {
    const isOpening = addTaskPanel.style.display === 'none';
    addTaskPanel.style.display = isOpening ? 'flex' : 'none';
    // 打开面板时将任务目录默认填入对话页当前目录，未设置时从后端拉取默认目录（仅作初值，确认后独立保存，不回写对话页）
    if (isOpening) {
        const workdir = await resolveDefaultWorkdir();
        const display = document.getElementById('taskWorkspaceDisplay');
        display.textContent = workdir || t('input.unset');
        display.title = workdir || '';
    }
}

// 渲染候选 Agent 多选列表，agents 为 Agent 列表，checkedIds 为已选中的 Agent id
function renderAgentCheckList(container, agents, checkedIds) {
    container.innerHTML = '';
    if (agents.length === 0) {
        container.innerHTML = `<div class="text-hint-mono">${t('task.noAgents')}</div>`;
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

// ===== 4. 跟随控制器基础 =====
// 获取（或惰性创建）任务控制器
function getTaskController(taskId) {
    if (!taskControllers[taskId]) {
        taskControllers[taskId] = {
            status: null,
            detail: null,
            timer: null,
            eventSource: null,
            streamConvId: null,
            streamLive: false,
            liveRaw: '',
            liveText: '',
            expanded: false,
            rechecking: false
        };
    }
    return taskControllers[taskId];
}

// 取任务的最新阶段对话（对话按 id 升序，末尾即最新）
function latestConversation(detail) {
    const conversations = (detail && detail.conversations) || [];
    return conversations.length > 0 ? conversations[conversations.length - 1] : null;
}


// 关闭任务的 SSE 跟随流
function closeTaskStream(controller) {
    if (controller.eventSource) {
        controller.eventSource.close();
        controller.eventSource = null;
    }
    controller.streamConvId = null;
    controller.streamLive = false;
}

// 停止任务的轮询定时器
function stopTaskPolling(controller) {
    if (controller.timer) {
        clearInterval(controller.timer);
        controller.timer = null;
    }
}

// 停止任务的全部跟随资源（轮询 + SSE）
function stopTaskController(taskId) {
    const controller = taskControllers[taskId];
    if (controller) {
        stopTaskPolling(controller);
        closeTaskStream(controller);
    }
}

// 视图清理钩子：离开任务视图时停止全部轮询与 SSE（由 switchView 按 VIEW_CONFIG.unload 调用）
function cleanupTaskView() {
    Object.keys(taskControllers).forEach(taskId => stopTaskController(parseInt(taskId)));
}

// ===== 5. 任务列表 =====
async function fetchTaskList() {
    taskListContainer.innerHTML = SKELETON_HTML;
    try {
        // 并行拉取任务清单与 Agent 花名册（候选 Agent 多选与操作区下拉共用）
        const [taskResponse, agentResponse] = await Promise.all([fetch('/task/list'), fetch('/agent/list')]);
        const tasks = await taskResponse.json();
        taskAgentRoster = await agentResponse.json();
        taskListContainer.innerHTML = '';
        renderAgentCheckList(document.getElementById('addTaskAgentList'), taskAgentRoster, []);
        // 清理已删除任务的控制器
        Object.keys(taskControllers).forEach(taskId => {
            if (!tasks.some(task => String(task.id) === taskId)) {
                stopTaskController(parseInt(taskId));
                delete taskControllers[taskId];
            }
        });
        if (tasks.length === 0) {
            taskListContainer.innerHTML = emptyListHtml('task.empty', 'task.emptyHint');
            return;
        }
        tasks.forEach(task => {
            taskListContainer.appendChild(createTaskCard(task));
            // 初始状态推导：进入视图时对每个任务一次性拉取详情（非周期轮询）
            loadTaskState(task.id);
        });
    } catch (e) {
        taskListContainer.innerHTML = errorListHtml('common.fetchFailed');
    }
}

// 创建任务卡片：摘要行（标题/状态徽章/实况行/阶段计数/删除）+ 展开详情容器
function createTaskCard(task) {
    const card = document.createElement('div');
    card.className = 'info-card task-card';
    card.id = `task-card-${task.id}`;
    card.innerHTML = `
        <div class="info-card-summary" onclick="toggleTaskCard(this.parentNode, ${task.id})">
            <div class="info-card-main">
                ${ARROW_SVG}
                <span class="info-card-name card-name-fixed">${escapeHtml(task.title)}</span>
                <span class="task-status-badge" id="task-status-${task.id}"></span>
                <span class="info-card-snippet task-live-line" id="task-live-${task.id}">${escapeHtml(task.content)}</span>
            </div>
            <div class="card-actions" onclick="event.stopPropagation();">
                <span class="task-stage-count" id="task-meta-${task.id}"></span>
                <button class="delete-btn always-visible" id="task-delete-${task.id}"></button>
            </div>
        </div>
        <div class="info-card-details" style="display: none;" id="task-details-${task.id}"></div>
    `;
    // 删除按钮通过闭包绑定，避免标题中的引号破坏内联 onclick 字符串
    const deleteBtn = card.querySelector(`#task-delete-${task.id}`);
    deleteBtn.innerHTML = DELETE_SVG;
    deleteBtn.title = t('common.purge');
    deleteBtn.onclick = () => removeTask(task.id, task.title);
    return card;
}

// ===== 6. 状态推导与摘要渲染 =====
// 由控制器数据推导五状态：执行循环存活标记（detail.running）优先，其余以后端持久化的 status 字段为准
function deriveTaskStatus(controller) {
    const detail = controller.detail;
    // 执行循环存活一律视为运行中（覆盖启动间隙/长轮次执行/阶段交接间隙）
    if (detail && detail.running) {
        return TASK_STATUS.RUNNING;
    }
    // 后端持久化状态为权威来源（由后端循环退出分支与审核提交处维护）
    if (detail && Object.values(TASK_STATUS).includes(detail.status)) {
        // 持久化状态残留运行中但执行循环已消亡（如应用被异常杀掉）：视为运行失败
        if (detail.status === TASK_STATUS.RUNNING) {
            return TASK_STATUS.FAILED;
        }
        return detail.status;
    }
    // status 缺失或非法时按待启动处理
    return TASK_STATUS.IDLE;
}

// 候选 Agent 对象列表（按任务 agent_ids 顺序解析花名册）
function candidateAgents(detail) {
    return (detail.agent_ids || []).map(agentId => taskAgentRoster.find(agent => agent.id === agentId)).filter(agent => agent);
}

// 候选 Agent 名称列表（只读展示）
function candidateAgentNames(detail) {
    return candidateAgents(detail).map(agent => agent.name).join(', ');
}

// 上一个执行者：阶段历史中最后一个非空的 agent_id（继续/审核下拉的默认选中项）
function lastExecutedAgentId(detail) {
    const conversations = (detail && detail.conversations) || [];
    for (let i = conversations.length - 1; i >= 0; i--) {
        if (conversations[i].agent_id != null) {
            return conversations[i].agent_id;
        }
    }
    return null;
}

// 刷新卡片摘要行：状态徽章、实况行、阶段计数、删除可用性与待审核高亮
function updateTaskSummary(taskId) {
    const controller = getTaskController(taskId);
    const detail = controller.detail;
    const card = document.getElementById(`task-card-${taskId}`);
    if (!detail || !card) {
        return;
    }
    const statusTextKey = {
        idle: 'task.statusIdle',
        running: 'task.statusRunning',
        review: 'task.statusReview',
        done: 'task.statusDone',
        failed: 'task.statusFailed'
    }[controller.status];
    const badge = document.getElementById(`task-status-${taskId}`);
    badge.textContent = t(statusTextKey);
    badge.className = `task-status-badge status-${controller.status}`;
    // 待审核卡片高亮提示人工介入
    card.classList.toggle('task-card-review', controller.status === TASK_STATUS.REVIEW);
    // 实况行：运行中优先流式文本，待审核显示等待提示，其余显示最新对话最后一条消息
    const liveLine = document.getElementById(`task-live-${taskId}`);
    const latest = latestConversation(detail);
    if (controller.status === TASK_STATUS.RUNNING && controller.liveText) {
        liveLine.textContent = controller.liveText;
    } else if (controller.status === TASK_STATUS.REVIEW) {
        liveLine.textContent = t('task.reviewWaiting');
    } else if (latest) {
        liveLine.textContent = getLastMessageText(latest.messages);
    } else {
        liveLine.textContent = detail.content || '';
    }
    // 阶段计数与最近更新时间
    const meta = document.getElementById(`task-meta-${taskId}`);
    const stageCount = (detail.conversations || []).length;
    meta.textContent = stageCount > 0
        ? t('task.stageCount', {count: stageCount}) + (latest && latest.update_time ? ` · ${latest.update_time}` : '')
        : '';
    // 运行中禁止删除（与后端 409 语义对齐）
    const deleteBtn = document.getElementById(`task-delete-${taskId}`);
    deleteBtn.disabled = controller.status === TASK_STATUS.RUNNING;
    deleteBtn.title = controller.status === TASK_STATUS.RUNNING ? t('task.runningNoPurge') : t('common.purge');
}

// ===== 7. 数据加载与双模式跟随 =====
// 拉取任务详情并应用（初始推导、轮询、流结束复核、动作后刷新共用入口）
async function loadTaskState(taskId) {
    try {
        const response = await fetch(`/task/${taskId}`);
        if (!response.ok) {
            return;
        }
        const detail = await response.json();
        applyTaskDetail(taskId, detail);
    } catch (e) {
        // 静默处理错误
    }
}

// 应用任务详情：推导状态、刷新摘要与展开区、同步跟随模式（轮询/SSE 两条路径的汇聚点）
function applyTaskDetail(taskId, detail) {
    const controller = getTaskController(taskId);
    const previousStatus = controller.status;
    controller.detail = detail;
    controller.status = deriveTaskStatus(controller);
    updateTaskSummary(taskId);
    // 运行中转入待审核：提醒用户人工介入
    if (controller.status === TASK_STATUS.REVIEW && previousStatus === TASK_STATUS.RUNNING) {
        showToast(t('task.reviewNeeded', {name: detail.title}));
    }
    if (controller.expanded) {
        renderTaskDetails(taskId);
        // 展开且运行中但无跟随流：补齐 SSE（复核窗口内除外，避免流刚关闭被立即重开）
        if (controller.status === TASK_STATUS.RUNNING && !controller.eventSource && !controller.rechecking) {
            const latest = latestConversation(detail);
            if (latest && latest.agent_id != null) {
                openTaskStream(taskId, latest.id);
            } else {
                // 运行中但最新仍是用户对话：阶段对话异步创建中，延迟补拉
                scheduleTaskRefetch(taskId, TASK_START_REFETCH_DELAY);
            }
        }
    }
    syncTaskFollowMode(taskId);
}

// 同步跟随模式：展开且运行中由 SSE 驱动（此处只管计时器），收起且运行中启动 5 秒轮询，其余状态静止
function syncTaskFollowMode(taskId) {
    const controller = getTaskController(taskId);
    const running = controller.status === TASK_STATUS.RUNNING;
    if (!running || !controller.expanded) {
        closeTaskStream(controller);
    }
    if (running && !controller.expanded && !controller.timer) {
        controller.timer = setInterval(() => loadTaskState(taskId), TASK_POLL_INTERVAL);
    }
    if ((!running || controller.expanded) && controller.timer) {
        stopTaskPolling(controller);
    }
}

// 延迟单次补拉：用于阶段对话异步创建等短暂间隙（非周期轮询）
function scheduleTaskRefetch(taskId, delay) {
    setTimeout(() => {
        if (taskControllers[taskId]) {
            loadTaskState(taskId);
        }
    }, delay);
}


// 打开阶段对话 SSE：实况行与阶段摘要实时更新
function openTaskStream(taskId, conversationId) {
    const controller = getTaskController(taskId);
    closeTaskStream(controller);
    controller.liveRaw = '';
    controller.liveText = '';
    controller.streamConvId = conversationId;
    const source = new EventSource(`/conversation/${conversationId}/stream`);
    controller.eventSource = source;
    source.onopen = () => {
        controller.streamLive = true;
    };
    source.onmessage = (event) => {
        const data = JSON.parse(event.data);
        controller.streamLive = true;
        // 提取最新文本类内容作为实况行与阶段摘要
        if (data.type === 'text' || data.type === 'thinking') {
            controller.liveRaw = data.text || '';
        } else if (data.type === 'delta') {
            controller.liveRaw += data.text || '';
        } else if (data.type === 'tool_use') {
            controller.liveRaw = `${t('stream.callPrefix')}: ${data.name || ''}`;
        }
        if (controller.liveRaw) {
            controller.liveText = controller.liveRaw;
            const liveLine = document.getElementById(`task-live-${taskId}`);
            if (liveLine) {
                liveLine.textContent = controller.liveText;
            }
            const stageItem = document.getElementById(`stage-item-${conversationId}`);
            if (stageItem) {
                const snippet = stageItem.querySelector('.task-stage-snippet');
                if (snippet) {
                    snippet.textContent = controller.liveText;
                    snippet.classList.add('stage-running');
                }
            }
        }
    };
    source.onerror = async () => {
        // 已被新连接替换时忽略
        if (controller.eventSource !== source) {
            return;
        }
        // 关闭连接并阻止浏览器自动重连
        source.close();
        controller.eventSource = null;
        controller.streamLive = false;
        // 流结束 = 当前对话完成：重新拉取任务，复核窗口内阻塞立即重开
        controller.rechecking = true;
        await loadTaskState(taskId);
        setTimeout(async () => {
            const current = getTaskController(taskId);
            if (!current.expanded || current.eventSource) {
                current.rechecking = false;
                return;
            }
            await loadTaskState(taskId);
            current.rechecking = false;
            if (current.status !== TASK_STATUS.RUNNING) {
                return;
            }
            const latest = latestConversation(current.detail);
            if (latest && latest.agent_id != null) {
                // 复核后仍运行中：继续跟随最新阶段对话
                openTaskStream(taskId, latest.id);
            } else {
                // 运行中但最新仍是用户对话：阶段对话异步创建中，延迟补拉
                scheduleTaskRefetch(taskId, TASK_START_REFETCH_DELAY);
            }
        }, TASK_RECHECK_DELAY);
    };
}

// ===== 8. 展开详情与统一操作区 =====
// 展开/收起任务卡片：展开渲染详情并切换 SSE 模式，收起回到轮询模式
function toggleTaskCard(cardElement, taskId) {
    toggleCardOpen(cardElement);
    const controller = getTaskController(taskId);
    controller.expanded = cardElement.hasAttribute('open');
    if (controller.expanded) {
        if (controller.detail) {
            renderTaskDetails(taskId);
            // 展开运行中的任务：开启 SSE 实时跟随最新阶段对话
            if (controller.status === TASK_STATUS.RUNNING && !controller.eventSource) {
                const latest = latestConversation(controller.detail);
                if (latest && latest.agent_id != null) {
                    openTaskStream(taskId, latest.id);
                }
            }
        } else {
            document.getElementById(`task-details-${taskId}`).innerHTML = SKELETON_HTML;
            loadTaskState(taskId);
        }
    }
    syncTaskFollowMode(taskId);
}

// 渲染展开区：统一操作区（顶部）+ 任务字段 + 阶段列表
function renderTaskDetails(taskId) {
    const controller = getTaskController(taskId);
    const detail = controller.detail;
    const container = document.getElementById(`task-details-${taskId}`);
    if (!container) {
        return;
    }
    if (!detail) {
        container.innerHTML = SKELETON_HTML;
        return;
    }
    // 审核输入框内容在重绘后恢复（状态切换触发重绘时保留用户输入）
    const oldInput = document.getElementById(`task-review-input-${taskId}`);
    const preservedInput = oldInput ? oldInput.value : null;
    container.innerHTML = '';
    // 统一操作区：启动/停止/审核/继续动作均在此处
    const actionArea = document.createElement('div');
    actionArea.className = 'task-action-area';
    container.appendChild(actionArea);
    renderTaskActionArea(taskId, actionArea);
    if (preservedInput) {
        const input = document.getElementById(`task-review-input-${taskId}`);
        if (input) {
            input.value = preservedInput;
        }
    }
    // 任务字段
    const grid = document.createElement('div');
    grid.className = 'details-grid';
    grid.innerHTML = `
        <div class="details-label">${t('task.title')}</div>
        <div class="details-value">${escapeHtml(detail.title)}</div>
        <div class="details-label">${t('task.workDir')}</div>
        <div class="details-value">${escapeHtml(detail.work_dir || t('common.inheritedEnv'))}</div>
        <div class="details-label">${t('task.candidateAgents')}</div>
        <div class="details-value">${escapeHtml(candidateAgentNames(detail)) || t('common.none')}</div>
        <div class="details-label">${t('task.content')}</div>
        <div class="details-value" style="white-space: pre-wrap;">${escapeHtml(detail.content)}</div>
    `;
    container.appendChild(grid);
    // 阶段列表
    const stageBlock = document.createElement('div');
    stageBlock.className = 'details-block-container';
    stageBlock.innerHTML = `<div class="details-block-header"><div class="details-label">${t('task.stageProgress')}</div></div>`;
    stageBlock.querySelector('.details-block-header').appendChild(createStageSortButton());
    stageBlock.querySelector('.details-block-header').appendChild(createStageExpandButton());
    const stageList = document.createElement('div');
    // 重绘时按当前展开状态补类，避免轮询/SSE 重绘丢失展开效果
    stageList.className = stageSnippetExpanded ? 'task-stage-list snippet-expanded' : 'task-stage-list';
    stageList.id = `task-stages-${taskId}`;
    stageBlock.appendChild(stageList);
    container.appendChild(stageBlock);
    renderTaskStages(taskId);
}

// 渲染统一操作区：按状态呈现 开始执行/停止任务/审核/继续执行 动作
function renderTaskActionArea(taskId, container) {
    const controller = getTaskController(taskId);
    const detail = controller.detail;
    const status = controller.status;
    // 运行中：停止任务按钮
    if (status === TASK_STATUS.RUNNING) {
        container.innerHTML = `<button class="btn btn-sm btn-secondary" onclick="stopTask(${taskId})">${t('task.stop')}</button>`;
        return;
    }
    // 其余状态：意见输入框 + 候选 Agent 下拉 + 状态主按钮；输入框占位文本即空输入时的默认提交内容
    const placeholderKey = {
        idle: 'task.launchPlaceholder',
        review: 'task.completePlaceholder',
        done: 'task.resumePlaceholder',
        failed: 'task.failedPlaceholder'
    }[status];
    const selectId = `task-agent-${taskId}`;
    let html = `<textarea id="task-review-input-${taskId}" class="form-control review-input" rows="2" placeholder="${t(placeholderKey)}"></textarea>`;
    html += '<div class="task-action-row">';
    html += `<select id="${selectId}" class="form-control task-start-select"></select>`;
    if (status === TASK_STATUS.IDLE) {
        html += `<button class="btn btn-sm send-button" onclick="runTask(${taskId}, t('task.launchPlaceholder'))">${t('task.launch')}</button>`;
    } else if (status === TASK_STATUS.REVIEW) {
        html += `<button class="btn btn-sm send-button" onclick="runTask(${taskId}, '')">${t('task.submitRun')}</button>`;
        html += `<button class="btn btn-sm send-button" onclick="submitTaskComplete(${taskId})">${t('task.complete')}</button>`;
    } else if (status === TASK_STATUS.DONE) {
        html += `<button class="btn btn-sm send-button" onclick="runTask(${taskId}, t('task.resumePlaceholder'))">${t('task.resume')}</button>`;
    } else if (status === TASK_STATUS.FAILED) {
        html += `<button class="btn btn-sm send-button" onclick="runTask(${taskId}, t('task.failedPlaceholder'))">${t('task.resume')}</button>`;
    }
    html += '</div>';
    container.innerHTML = html;
    // 填充候选 Agent 下拉：默认选中上一个执行者；无候选时禁用主按钮并提示原因
    const select = document.getElementById(selectId);
    const candidates = candidateAgents(detail);
    fillSelectOptions(select, candidates, lastExecutedAgentId(detail));
    if (candidates.length === 0) {
        const hintOpt = document.createElement('option');
        hintOpt.textContent = t('task.needCandidate');
        select.appendChild(hintOpt);
        select.disabled = true;
        const primaryBtn = container.querySelector('.send-button');
        if (primaryBtn) {
            primaryBtn.disabled = true;
            primaryBtn.title = t('task.needCandidate');
        }
    }
}
// 渲染阶段列表：点击阶段项打开覆盖式弹窗查看对话内容
function renderTaskStages(taskId) {
    const controller = getTaskController(taskId);
    const stageList = document.getElementById(`task-stages-${taskId}`);
    if (!stageList) {
        return;
    }
    const conversations = (controller.detail && controller.detail.conversations) || [];
    stageList.innerHTML = '';
    if (conversations.length === 0) {
        stageList.innerHTML = `<div class="text-hint">${t('task.notStarted')}</div>`;
        return;
    }
    // 按当前排序方向渲染（升/降序由排序按钮切换并持久化记忆）
    sortedStageConversations(conversations).forEach(conversation => {
        stageList.appendChild(createStageRecordItem(conversation));
    });
}

// ===== 9. 动作：启动/停止/审核/继续/重启/新增/删除 =====
// 启动任务并附带用户消息（开始执行/提交并执行/继续执行统一入口）：消息随启动请求提交，由后端落入用户对话后再启动流水线
async function runTask(taskId, defaultText) {
    const select = document.getElementById(`task-agent-${taskId}`);
    const agentId = select ? select.value : '';
    if (!agentId) {
        showToast(t('task.agentRequired'), 'error');
        return;
    }
    const input = document.getElementById(`task-review-input-${taskId}`);
    const typed = input ? input.value.trim() : '';
    // defaultText 为空串时表示必须填写意见（提交并执行），否则空输入提交占位文本
    const content = typed || defaultText;
    if (!content) {
        showToast(t('task.reviewRequired'), 'error');
        return;
    }
    try {
        const response = await fetch(`/task/${taskId}/start`, {
            method: 'POST',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({agent_id: parseInt(agentId), message: content})
        });
        if (response.ok) {
            showToast(t('task.launched'));
            // 阶段对话由后台异步创建：立即拉取一次，再延迟补拉一次兜底
            await loadTaskState(taskId);
            scheduleTaskRefetch(taskId, TASK_START_REFETCH_DELAY);
        } else if (response.status === 409) {
            showToast(t('task.alreadyRunning'), 'error');
        } else {
            showToast(t('task.startFault'), 'error');
        }
    } catch (e) {
        showToast(t('task.startFault'), 'error');
    }
}

// 停止任务：通知后端停止执行循环（当前对话优雅收尾后循环退出），状态经延迟补拉收敛
async function stopTask(taskId) {
    try {
        const response = await fetch(`/task/${taskId}/stop`, {method: 'POST'});
        if (response.ok) {
            showToast(t('task.stopped'));
            // 循环退出与对话收尾为异步过程：立即拉取一次，再延迟补拉一次兜底
            await loadTaskState(taskId);
            scheduleTaskRefetch(taskId, TASK_START_REFETCH_DELAY);
        } else if (response.status === 409) {
            showToast(t('task.notRunning'), 'error');
        } else {
            showToast(t('task.stopFault'), 'error');
        }
    } catch (e) {
        showToast(t('task.stopFault'), 'error');
    }
}

// 完成任务：调用任务完成接口（后端向待审核对话追加用户消息并将状态置为已完成，空输入时提交占位文本「完成」），不启动流水线
async function submitTaskComplete(taskId) {
    const input = document.getElementById(`task-review-input-${taskId}`);
    const typed = input ? input.value.trim() : '';
    const content = typed || t('task.completePlaceholder');
    try {
        const response = await fetch(`/task/${taskId}/complete`, {
            method: 'POST',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({message: content})
        });
        if (response.ok) {
            await loadTaskState(taskId);
        } else {
            showToast(t('task.reviewFault'), 'error');
        }
    } catch (e) {
        showToast(t('task.reviewFault'), 'error');
    }
}
// 新增任务
async function submitTask() {
    const title = document.getElementById('taskTitle').value.trim();
    const content = document.getElementById('taskContent').value.trim();
    // 任务面板独立维护工作目录，从面板显示元素读取而非全局对话目录
    const workDir = document.getElementById('taskWorkspaceDisplay').title || '';
    const agentIds = getCheckedAgentIds(document.getElementById('addTaskAgentList'));
    if (!title || !content || !workDir) {
        showToast(t('common.requiredMissing'), 'error');
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
        showToast(t('common.creationFault'), 'error');
    }
}

// 删除任务：清理跟随控制器后刷新列表
function removeTask(taskId, taskTitle) {
    showConfirmDialog({
        title: t('task.purgeTitle'),
        text: t('task.purgeText', {name: taskTitle}),
        onConfirm: async () => {
            try {
                const response = await fetch(`/task/${taskId}`, {method: 'DELETE'});
                if (response.ok) {
                    stopTaskController(taskId);
                    delete taskControllers[taskId];
                    await fetchTaskList();
                }
            } catch (e) {
                showToast(t('common.purgeFailure'), 'error');
            }
        }
    });
}
