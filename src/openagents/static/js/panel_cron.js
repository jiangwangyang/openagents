// ==========================================
// CRON 核心排程自动化面板
// ==========================================
const cronListContainer = document.getElementById('cronListContainer');
const addCronPanel = document.getElementById('addCronPanel');

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

