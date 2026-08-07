// ==========================================
// CONFIG 全局环境核心控制台面板（模型供应商与路由）
// ==========================================
const addProviderPanel = document.getElementById('addProviderPanel');

// 全局模型供应商缓存（配置表单渲染与模型下拉联动共用）
let globalProvidersCachedList = [];

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
        providerList.innerHTML = `<div style="padding:20px; color:var(--danger-color)">${t('config.settingsFault')}</div>`;
    }
}

function renderProvidersFormList() {
    const providerList = document.getElementById('providerConfigList');
    providerList.innerHTML = '';

    globalProvidersCachedList.forEach((provider, index) => {
        const name = provider.name;
        const card = document.createElement('div');
        card.className = 'info-card';
        // 元素 id 使用列表索引生成（供应商名可能包含引号等特殊字符）
        card.id = `provider-card-${index}`;

        const snippet = provider.base_url || t('config.endpointMissing');

        card.innerHTML = `
            <div class="info-card-summary" onclick="toggleCardOpen(this.parentNode)">
                <div class="info-card-main">
                    ${ARROW_SVG}
                    <span class="info-card-name" style="min-width:180px; max-width:280px;">${escapeHtml(name)}</span>
                    <span class="info-card-snippet">${escapeHtml(snippet)}</span>
                </div>
                <div style="display:flex; gap:12px; align-items:center;" onclick="event.stopPropagation();">
                    <button class="btn btn-sm send-button provider-save-btn" style="height:28px; padding:0 8px; font-size:10px;">${t('common.save')}</button>
                    <button class="delete-btn" style="opacity:1; color:var(--danger-color); font-size:11px; font-family:var(--font-mono); font-weight:700; padding:6px;">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
                    </button>
                </div>
            </div>
            <div class="info-card-details" style="display: none; background: var(--bg-surface); border-top: 1px solid var(--border-hard);">
                <div class="form-row" style="margin-bottom: 12px;">
                    <div class="form-group" style="flex: 2;">
                        <label style="font-family: var(--font-display); font-size: 11px; font-weight: 700; text-transform: uppercase; color: var(--charcoal-700);">${t('config.baseUrl')}</label>
                        <input type="text" id="provider-url-${index}" class="form-control mono" value="${escapeHtml(provider.base_url || '')}">
                    </div>
                    <div class="form-group" style="flex: 1;">
                        <label style="font-family: var(--font-display); font-size: 11px; font-weight: 700; text-transform: uppercase; color: var(--charcoal-700);">${t('config.secretToken')}</label>
                        <input type="password" id="provider-key-${index}" class="form-control mono" value="${escapeHtml(provider.api_key || '')}" placeholder="••••••••••••">
                    </div>
                </div>
                <div class="form-row">
                    <div class="form-group">
                        <label style="font-family: var(--font-display); font-size: 11px; font-weight: 700; text-transform: uppercase; color: var(--charcoal-700);">${t('config.engineClusters')}</label>
                        <input type="text" id="provider-models-${index}" class="form-control mono" value="${escapeHtml(provider.models?.join(', ') || '')}">
                    </div>
                </div>
            </div>
        `;
        // 按钮通过闭包绑定，避免供应商名中的引号破坏内联 onclick 字符串
        card.querySelector('.provider-save-btn').onclick = () => updateSingleProvider(name);
        card.querySelector('.delete-btn').onclick = () => removeModelProvider(name);
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
        alert(t('config.nameRequired'));
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
        alert(t('config.injectionFault'));
    }
}

async function updateSingleProvider(name) {
    // 元素 id 使用列表索引生成（供应商名可能包含引号等特殊字符）
    const index = globalProvidersCachedList.findIndex(p => p.name === name);
    if (index === -1) {
        return;
    }
    const urlValue = document.getElementById(`provider-url-${index}`).value.trim();
    const keyValue = document.getElementById(`provider-key-${index}`).value.trim();
    const modelsValue = document.getElementById(`provider-models-${index}`).value.trim();

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
            alert(t('config.synced', {name: name}));
            const pResponse = await fetch('/model-provider/list');
            globalProvidersCachedList = await pResponse.json();
            syncModelDropdown(document.getElementById('configActiveModel').value);
        }
    } catch (e) {
        alert(t('common.syncCrashed'));
    }
}

function removeModelProvider(name) {
    showConfirmDialog({
        title: t('config.purgeTitle'),
        text: t('config.purgeText', {name: name}),
        onConfirm: async () => {
            try {
                const response = await fetch(`/model-provider/${encodeURIComponent(name)}`, {method: 'DELETE'});
                if (response.ok) {
                    await fetchGlobalSettings();
                }
            } catch (e) {
                alert(t('common.purgeFailure'));
            }
        }
    });
}

async function saveActiveModelRoute() {
    const providerSelect = document.getElementById('configActiveProvider');
    const modelSelect = document.getElementById('configActiveModel');

    if (!providerSelect.value || !modelSelect.value) {
        alert(t('config.routingIncomplete'));
        return;
    }

    try {
        let response = await fetch('/model-provider/current', {
            method: 'PUT',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({name: providerSelect.value})
        });
        if (!response.ok) {
            alert(t('config.providerRoutingFailed'));
            return;
        }
        response = await fetch('/model/current', {
            method: 'PUT',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({model: modelSelect.value})
        });
        if (!response.ok) {
            alert(t('config.modelRoutingFailed'));
            return;
        }
        response = await fetch('/thinking', {
            method: 'PUT',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({thinking: document.getElementById('configThinking').value === 'true'})
        });
        if (response.ok) {
            alert(t('config.routeMounted'));
        }
    } catch (e) {
        alert(t('config.routingCommitFault'));
    }
}

