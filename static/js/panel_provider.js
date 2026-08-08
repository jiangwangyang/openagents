// ==========================================
// CONFIG 全局环境核心控制台面板（模型供应商）
// ==========================================
const addProviderPanel = document.getElementById('addProviderPanel');

function toggleAddProviderPanel() {
    addProviderPanel.style.display = addProviderPanel.style.display === 'none' ? 'flex' : 'none';
}

async function fetchGlobalSettings() {
    const providerList = document.getElementById('providerConfigList');
    providerList.innerHTML = SKELETON_HTML;

    try {
        const pResponse = await fetch('/model-provider/list');
        const providers = await pResponse.json();
        renderProvidersFormList(providers);
    } catch (e) {
        providerList.innerHTML = `<div style="padding:20px; color:var(--danger-color)">${t('config.settingsFault')}</div>`;
    }
}

function renderProvidersFormList(providers) {
    const providerList = document.getElementById('providerConfigList');
    providerList.innerHTML = '';

    providers.forEach((provider, index) => {
        const card = document.createElement('div');
        card.className = 'info-card';
        // 元素 id 使用列表索引生成（供应商名可能包含引号等特殊字符）
        card.id = `provider-card-${index}`;

        const snippet = provider.base_url || t('config.endpointMissing');

        card.innerHTML = `
            <div class="info-card-summary" onclick="toggleCardOpen(this.parentNode)">
                <div class="info-card-main">
                    ${ARROW_SVG}
                    <span class="info-card-name" style="min-width:180px; max-width:280px;">${escapeHtml(provider.name)}</span>
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
                    <div class="form-group" style="flex: 1;">
                        <label style="font-family: var(--font-display); font-size: 11px; font-weight: 700; text-transform: uppercase; color: var(--charcoal-700);">${t('config.providerName')}</label>
                        <input type="text" id="provider-name-${index}" class="form-control mono" value="${escapeHtml(provider.name)}">
                    </div>
                    <div class="form-group" style="flex: 1;">
                        <label style="font-family: var(--font-display); font-size: 11px; font-weight: 700; text-transform: uppercase; color: var(--charcoal-700);">${t('config.protocolType')}</label>
                        <select id="provider-type-${index}" class="form-control">
                            <option value="anthropic">anthropic</option>
                            ${provider.protocol_type && provider.protocol_type !== 'anthropic' ? `<option value="${escapeHtml(provider.protocol_type)}" selected>${escapeHtml(provider.protocol_type)}</option>` : ''}
                        </select>
                    </div>
                </div>
                <div class="form-row">
                    <div class="form-group" style="flex: 1;">
                        <label style="font-family: var(--font-display); font-size: 11px; font-weight: 700; text-transform: uppercase; color: var(--charcoal-700);">${t('config.baseUrl')}</label>
                        <input type="text" id="provider-url-${index}" class="form-control mono" value="${escapeHtml(provider.base_url || '')}">
                    </div>
                    <div class="form-group" style="flex: 1;">
                        <label style="font-family: var(--font-display); font-size: 11px; font-weight: 700; text-transform: uppercase; color: var(--charcoal-700);">${t('config.secretToken')}</label>
                        <div class="password-toggle-group">
                            <input type="password" id="provider-key-${index}" class="form-control mono" value="${escapeHtml(provider.api_key || '')}" placeholder="••••••••••••">
                            <button type="button" class="password-toggle-btn" onclick="toggleKeyVisibility('provider-key-${index}', this)" title="Show / Hide Secret">👁</button>
                        </div>
                    </div>
                </div>
            </div>
        `;
        // 按钮通过闭包绑定，避免供应商名中的引号破坏内联 onclick 字符串
        card.querySelector('.provider-save-btn').onclick = () => updateSingleProvider(provider.id, index);
        card.querySelector('.delete-btn').onclick = () => removeModelProvider(provider.id, provider.name);
        providerList.appendChild(card);
    });
}

async function submitNewProvider() {
    const name = document.getElementById('newProviderName').value.trim();
    if (!name) {
        alert(t('config.nameRequired'));
        return;
    }

    const bodyPayload = {
        name: name,
        protocol_type: document.getElementById('newProviderType').value,
        base_url: document.getElementById('newProviderUrl').value.trim(),
        api_key: document.getElementById('newProviderKey').value.trim()
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
            toggleAddProviderPanel();
            await fetchGlobalSettings();
        }
    } catch (e) {
        alert(t('config.injectionFault'));
    }
}

async function updateSingleProvider(id, index) {
    const name = document.getElementById(`provider-name-${index}`).value.trim();
    if (!name) {
        alert(t('config.nameRequired'));
        return;
    }
    const bodyPayload = {
        name: name,
        protocol_type: document.getElementById(`provider-type-${index}`).value,
        base_url: document.getElementById(`provider-url-${index}`).value.trim(),
        api_key: document.getElementById(`provider-key-${index}`).value.trim()
    };

    try {
        const response = await fetch(`/model-provider/${id}`, {
            method: 'PUT',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify(bodyPayload)
        });
        if (response.ok) {
            alert(t('config.synced', {name: name}));
            await fetchGlobalSettings();
        }
    } catch (e) {
        alert(t('common.syncCrashed'));
    }
}

function removeModelProvider(id, name) {
    showConfirmDialog({
        title: t('config.purgeTitle'),
        text: t('config.purgeText', {name: name}),
        onConfirm: async () => {
            try {
                const response = await fetch(`/model-provider/${id}`, {method: 'DELETE'});
                if (response.ok) {
                    await fetchGlobalSettings();
                }
            } catch (e) {
                alert(t('common.purgeFailure'));
            }
        }
    });
}

// Secret Token 显示/隐藏切换（增加卡片与修改卡片通用）
function toggleKeyVisibility(inputId, btn) {
    const input = document.getElementById(inputId);
    if (!input) return;
    const isVisible = input.type === 'text';
    input.type = isVisible ? 'password' : 'text';
    btn.textContent = isVisible ? '👁' : '🙈';
}
