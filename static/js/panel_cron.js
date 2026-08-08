// ==========================================
// CRON 核心排程自动化面板
// ==========================================
const cronListContainer = document.getElementById('cronListContainer');
const addCronPanel = document.getElementById('addCronPanel');

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

async function fetchCronTasks() {
    cronListContainer.innerHTML = SKELETON_HTML;
    try {
        const response = await fetch('/schedule/list');
        const tasks = await response.json();
        cronListContainer.innerHTML = '';

        if (tasks.length === 0) {
            cronListContainer.innerHTML = `<div style="padding:30px; text-align:center; border:1px dashed var(--border-hard); color:var(--slate-400)">${t('cron.empty')}</div>`;
            return;
        }

        tasks.forEach(task => {
            const card = document.createElement('div');
            card.className = 'info-card';
            card.innerHTML = `
                <div class="info-card-summary" onclick="toggleCardOpen(this.parentNode)">
                    <div class="info-card-main">
                        ${ARROW_SVG}
                        <span class="info-card-name">${escapeHtml(task.name || t('cron.unnamed'))}</span>
                        <span class="info-card-snippet">${escapeHtml(task.content || '')}</span>
                    </div>
                    <button class="delete-btn" style="opacity:1;">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
                    </button>
                </div>
                <div class="info-card-details" style="display: none;">
                    <div class="details-grid">
                        <div class="details-label">${t('cron.workingDir')}</div>
                        <div class="details-value">${escapeHtml(task.work_dir || t('common.inheritedEnv'))}</div>
                        <div class="details-label">${t('cron.nextFire')}</div>
                        <div class="details-value" style="font-family:var(--font-mono); font-size:12px; color:var(--charcoal-900)">${escapeHtml(task.next_fire_time || t('cron.suspended'))}</div>
                        <div class="details-label">${t('cron.triggerSpec')}</div>
                        <div class="details-value"><code>${escapeHtml(task.trigger || '* * * * *')}</code></div>
                        <div class="details-block-container">
                            <div class="details-label" style="margin-bottom: 6px;">${t('cron.execContent')}</div>
                            <div class="reply-content">${formatMarkdown(task.content || '')}</div>
                        </div>
                    </div>
                </div>
            `;
            // 删除按钮通过闭包绑定，避免任务名中的引号破坏内联 onclick 字符串
            card.querySelector('.delete-btn').onclick = (event) => {
                event.stopPropagation();
                removeCronTask(task.id, task.name);
            };
            cronListContainer.appendChild(card);
        });
    } catch (e) {
        cronListContainer.innerHTML = `<div style="padding:20px; color:var(--danger-color)">${t('common.fetchFailed')}</div>`;
    }
}

function toggleAddCronPanel() {
    addCronPanel.style.display = addCronPanel.style.display === 'none' ? 'flex' : 'none';
    if (addCronPanel.style.display === 'flex') {
        loadCronAgentOptions();
    }
}

async function submitCronTask() {
    const name = document.getElementById('cronName').value.trim();
    const content = document.getElementById('cronContent').value.trim();
    const work_dir = document.getElementById('cronWorkDir').value.trim();

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

