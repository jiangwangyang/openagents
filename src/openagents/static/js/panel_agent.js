// ==========================================
// AGENT 注册管理面板
// ==========================================
const agentListContainer = document.getElementById('agentListContainer');
const addAgentPanel = document.getElementById('addAgentPanel');

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
            agentListContainer.innerHTML = `<div style="padding:30px; text-align:center; border:1px dashed var(--border-hard); color:var(--slate-400)">${t('agent.empty')}</div>`;
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
                        <button class="btn btn-sm send-button" style="height:28px; padding:0 8px; font-size:10px;" onclick="updateSingleAgent(${agent.id})">${t('common.save')}</button>
                        <button class="delete-btn" style="opacity:1; padding:6px;">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
                        </button>
                    </div>
                </div>
                <div class="info-card-details" style="display: none;">
                    <div class="details-grid">
                        <div class="details-label">${t('agent.name')}</div>
                        <div class="details-value">
                            <input type="text" id="agent-name-${agent.id}" class="form-control" value="${escapeHtml(agent.name)}">
                        </div>

                        <div class="details-label">${t('common.description')}</div>
                        <div class="details-value">
                            <input type="text" id="agent-desc-${agent.id}" class="form-control" value="${escapeHtml(agent.description)}">
                        </div>

                        <div class="details-label">${t('agent.prompt')}</div>
                        <div class="details-value">
                            <textarea id="agent-prompt-${agent.id}" class="form-control mono" rows="6" style="resize: vertical; font-size:11px;">${escapeHtml(agent.prompt)}</textarea>
                        </div>
                    </div>
                </div>
            `;
            // 删除按钮通过闭包绑定，避免名称中的引号破坏内联 onclick 字符串
            card.querySelector('.delete-btn').onclick = () => removeAgent(agent.id, agent.name);
            agentListContainer.appendChild(card);
        });
    } catch (e) {
        agentListContainer.innerHTML = `<div style="padding:20px; color:var(--danger-color)">${t('agent.rosterCrashed')}</div>`;
    }
}

async function submitAgent() {
    const name = document.getElementById('agentName').value.trim();
    const description = document.getElementById('agentDesc').value.trim();
    const prompt = document.getElementById('agentPrompt').value.trim();
    if (!name) {
        alert(t('agent.nameRequired'));
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
        alert(t('agent.commitFailure'));
    }
}

async function updateSingleAgent(id) {
    const name = document.getElementById(`agent-name-${id}`).value.trim();
    const description = document.getElementById(`agent-desc-${id}`).value.trim();
    const prompt = document.getElementById(`agent-prompt-${id}`).value.trim();
    if (!name) {
        alert(t('agent.nameRequired'));
        return;
    }

    try {
        const response = await fetch(`/agent/${id}`, {
            method: 'PUT',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({name: name, description: description, prompt: prompt})
        });
        if (response.ok) {
            alert(t('agent.synced', {name: name}));
            await fetchAgentRegistry();
        }
    } catch (e) {
        alert(t('common.syncCrashed'));
    }
}

function removeAgent(id, name) {
    showConfirmDialog({
        title: t('agent.purgeTitle'),
        text: t('agent.purgeText', {name: name}),
        onConfirm: async () => {
            try {
                const response = await fetch(`/agent/${id}`, {method: 'DELETE'});
                if (response.ok) {
                    await fetchAgentRegistry();
                }
            } catch (e) {
                alert(t('common.purgeFailure'));
            }
        }
    });
}

