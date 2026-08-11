// ==========================================
// AGENT 注册管理面板
// ==========================================
const agentListContainer = document.getElementById('agentListContainer');
const addAgentPanel = document.getElementById('addAgentPanel');

function toggleAddAgentPanel() {
    addAgentPanel.style.display = addAgentPanel.style.display === 'none' ? 'flex' : 'none';
    // 打开面板时刷新供应商下拉
    if (addAgentPanel.style.display !== 'none') {
        loadAgentProviderOptions('agentProvider', null);
    }
}

// 加载供应商下拉选项（必填，无空选项），selectedId 指定后选中
async function loadAgentProviderOptions(elementId, selectedId) {
    try {
        const response = await fetch('/model-provider/list');
        const providers = await response.json();
        fillSelectOptions(document.getElementById(elementId), providers, selectedId);
    } catch (e) {
        // 静默处理错误
    }
}

async function fetchAgentRegistry() {
    agentListContainer.innerHTML = SKELETON_HTML;
    try {
        const response = await fetch('/agent/list');
        const agents = await response.json();
        agentListContainer.innerHTML = '';

        if (agents.length === 0) {
            agentListContainer.innerHTML = emptyListHtml('agent.empty', 'agent.emptyHint');
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
                        <span class="info-card-name card-name-fixed">${escapeHtml(agent.name)}</span>
                        <span class="info-card-snippet">${escapeHtml(agent.description)}</span>
                    </div>
                    <div class="card-actions" onclick="event.stopPropagation();">
                        <button class="btn btn-sm send-button btn-card-sm" onclick="updateSingleAgent(${agent.id})">${t('common.save')}</button>
                        <button class="delete-btn always-visible">${DELETE_SVG}</button>
                    </div>
                </div>
                <div class="info-card-details" style="display: none;">
                    <div class="form-row">
                        <div class="form-group flex-1">
                            <label>${t('agent.name')}</label>
                            <input type="text" id="agent-name-${agent.id}" class="form-control" value="${escapeHtml(agent.name)}">
                        </div>
                        <div class="form-group flex-2">
                            <label>${t('common.description')}</label>
                            <input type="text" id="agent-desc-${agent.id}" class="form-control" value="${escapeHtml(agent.description)}">
                        </div>
                    </div>
                    <div class="form-row">
                        <div class="form-group">
                            <label>${t('agent.prompt')}</label>
                            <textarea id="agent-prompt-${agent.id}" class="form-control mono textarea-sm" rows="6">${escapeHtml(agent.prompt)}</textarea>
                        </div>
                    </div>
                    <div class="form-row">
                        <div class="form-group flex-1">
                            <label>${t('input.providerTitle')}</label>
                            <select id="agent-provider-${agent.id}" class="form-control"></select>
                        </div>
                        <div class="form-group flex-1">
                            <label>${t('input.modelTitle')}</label>
                            <input type="text" id="agent-model-${agent.id}" class="form-control mono" value="${escapeHtml(agent.model || '')}">
                        </div>
                        <div class="form-group flex-1">
                            <label>${t('input.thinkingTitle')}</label>
                            <select id="agent-thinking-${agent.id}" class="form-control">
                                <option value="true" ${agent.thinking ? 'selected' : ''}>THINK</option>
                                <option value="false" ${agent.thinking ? '' : 'selected'}>NO THINK</option>
                            </select>
                        </div>
                    </div>
                </div>
            `;
            // 删除按钮通过闭包绑定，避免名称中的引号破坏内联 onclick 字符串
            const deleteBtn = card.querySelector('.delete-btn');
            deleteBtn.title = t('common.purge');
            deleteBtn.onclick = () => removeAgent(agent.id, agent.name);
            agentListContainer.appendChild(card);
            // 异步填充供应商下拉
            loadAgentProviderOptions(`agent-provider-${agent.id}`, agent.model_provider_id);
        });
    } catch (e) {
        agentListContainer.innerHTML = errorListHtml('agent.rosterCrashed');
    }
}

async function submitAgent() {
    const name = document.getElementById('agentName').value.trim();
    const description = document.getElementById('agentDesc').value.trim();
    const prompt = document.getElementById('agentPrompt').value.trim();
    if (!name) {
        showToast(t('agent.nameRequired'), 'error');
        return;
    }
    const providerValue = document.getElementById('agentProvider').value;
    if (!providerValue) {
        showToast(t('agent.providerRequired'), 'error');
        return;
    }

    try {
        const response = await fetch('/agent', {
            method: 'POST',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({
                name: name,
                description: description,
                prompt: prompt,
                model_provider_id: parseInt(providerValue),
                model: document.getElementById('agentModel').value.trim(),
                thinking: document.getElementById('agentThinking').value === 'true'
            })
        });
        if (response.ok) {
            document.getElementById('agentName').value = '';
            document.getElementById('agentDesc').value = '';
            document.getElementById('agentPrompt').value = '';
            document.getElementById('agentModel').value = '';
            toggleAddAgentPanel();
            await fetchAgentRegistry();
        }
    } catch (e) {
        showToast(t('agent.commitFailure'), 'error');
    }
}

async function updateSingleAgent(id) {
    const name = document.getElementById(`agent-name-${id}`).value.trim();
    const description = document.getElementById(`agent-desc-${id}`).value.trim();
    const prompt = document.getElementById(`agent-prompt-${id}`).value.trim();
    if (!name) {
        showToast(t('agent.nameRequired'), 'error');
        return;
    }
    const providerValue = document.getElementById(`agent-provider-${id}`).value;
    if (!providerValue) {
        showToast(t('agent.providerRequired'), 'error');
        return;
    }

    try {
        const response = await fetch(`/agent/${id}`, {
            method: 'PUT',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({
                name: name,
                description: description,
                prompt: prompt,
                model_provider_id: parseInt(providerValue),
                model: document.getElementById(`agent-model-${id}`).value.trim(),
                thinking: document.getElementById(`agent-thinking-${id}`).value === 'true'
            })
        });
        if (response.ok) {
            showToast(t('agent.synced', {name: name}));
            await fetchAgentRegistry();
        }
    } catch (e) {
        showToast(t('common.syncCrashed'), 'error');
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
                showToast(t('common.purgeFailure'), 'error');
            }
        }
    });
}
