// ==========================================
// MCP (Model Context Protocol) 服务注册面板
// ==========================================
const mcpListContainer = document.getElementById('mcpListContainer');
const addMcpPanel = document.getElementById('addMcpPanel');

function toggleAddMcpPanel() {
    addMcpPanel.style.display = addMcpPanel.style.display === 'none' ? 'flex' : 'none';
}

function adaptMcpFormFields() {
    const type = document.getElementById('mcpType').value;
    const isStdio = type === 'stdio';
    document.getElementById('mcpNetworkRow').style.display = isStdio ? 'none' : 'flex';
    document.getElementById('mcpNetworkRowHeaders').style.display = isStdio ? 'none' : 'flex';
    document.getElementById('mcpLocalRow').style.display = isStdio ? 'flex' : 'none';
    document.getElementById('mcpLocalRowArgs').style.display = isStdio ? 'flex' : 'none';
}

// 切换 MCP 修改卡片内 stdio / 网络 字段区域显示（协议类型下拉联动）
function adaptMcpCardFields(index) {
    const type = document.getElementById(`mcp-type-${index}`).value;
    const isStdio = type === 'stdio';
    document.getElementById(`mcp-network-zone-${index}`).style.display = isStdio ? 'none' : 'flex';
    document.getElementById(`mcp-local-zone-${index}`).style.display = isStdio ? 'flex' : 'none';
}

async function fetchMcpRegistry() {
    mcpListContainer.innerHTML = SKELETON_HTML;
    try {
        const response = await fetch('/mcp-server/list');
        const servers = await response.json();
        mcpListContainer.innerHTML = '';

        if (servers.length === 0) {
            mcpListContainer.innerHTML = emptyListHtml('mcp.empty', 'mcp.emptyHint');
            return;
        }

        servers.forEach((server, index) => {
            const name = server.name;
            const card = document.createElement('div');
            card.className = 'info-card';
            // 元素 id 使用列表索引生成（服务名可能包含引号等特殊字符）
            card.id = `mcp-card-${index}`;

            const snippet = server.url || (server.command ? `${server.command} ${server.args?.join(' ')}` : t('mcp.localContext'));
            const isStdio = server.protocol_type === 'stdio';

            card.innerHTML = `
                <div class="info-card-summary" onclick="toggleCardOpen(this.parentNode)">
                    <div class="info-card-main">
                        ${ARROW_SVG}
                        <span class="info-card-name card-name-fixed">${escapeHtml(name)}</span>
                        <span class="info-card-snippet">${escapeHtml(snippet)}</span>
                    </div>
                    <div class="card-actions" onclick="event.stopPropagation();">
                        <button class="btn btn-sm send-button mcp-save-btn btn-card-sm">${t('common.save')}</button>
                        <button class="btn btn-sm send-button mcp-test-btn" style="height:28px;">${t('mcp.testProbe')}</button>
                        <button class="delete-btn always-visible">${DELETE_SVG}</button>
                    </div>
                </div>
                <div class="info-card-details" style="display: none;">
                    <div class="form-row">
                        <div class="form-group flex-1">
                            <label>${t('mcp.keyName')}</label>
                            <input type="text" id="mcp-name-${index}" class="form-control mono" value="${escapeHtml(server.name || '')}">
                        </div>
                        <div class="form-group flex-1">
                            <label>${t('common.description')}</label>
                            <input type="text" id="mcp-desc-${index}" class="form-control" value="${escapeHtml(server.description || '')}">
                        </div>
                        <div class="form-group flex-1">
                            <label>${t('mcp.protocolType')}</label>
                            <select id="mcp-type-${index}" class="form-control" onchange="adaptMcpCardFields(${index})">
                                <option value="streamable-http" ${server.protocol_type !== 'stdio' ? 'selected' : ''}>streamable_http</option>
                                <option value="stdio" ${server.protocol_type === 'stdio' ? 'selected' : ''}>stdio</option>
                            </select>
                        </div>
                    </div>
                    <div id="mcp-network-zone-${index}" class="mcp-zone" style="display:${isStdio ? 'none' : 'flex'};">
                        <div class="form-row">
                            <div class="form-group">
                                <label>${t('mcp.targetUrl')}</label>
                                <input type="text" id="mcp-url-${index}" class="form-control mono" value="${escapeHtml(server.url || '')}">
                            </div>
                        </div>
                        <div class="form-row">
                            <div class="form-group">
                                <label>${t('mcp.httpHeaders')}</label>
                                <textarea id="mcp-headers-${index}" class="form-control mono textarea-sm" rows="3">${escapeHtml(JSON.stringify(server.headers || {}, null, 2))}</textarea>
                            </div>
                        </div>
                    </div>
                    <div id="mcp-local-zone-${index}" class="mcp-zone" style="display:${isStdio ? 'flex' : 'none'};">
                        <div class="form-row">
                            <div class="form-group">
                                <label>${t('mcp.cmdExec')}</label>
                                <input type="text" id="mcp-command-${index}" class="form-control mono" value="${escapeHtml(server.command || '')}">
                            </div>
                        </div>
                        <div class="form-row">
                            <div class="form-group">
                                <label>${t('mcp.args')}</label>
                                <input type="text" id="mcp-args-${index}" class="form-control mono" value="${escapeHtml(server.args?.join(', ') || '')}">
                            </div>
                        </div>
                    </div>

                    <div class="details-block-container" id="mcp-tools-zone-${index}" style="display:none;">
                        <div class="details-label" style="margin-bottom: 6px;">${t('mcp.exposedCaps')}</div>
                        <div class="mcp-tool-badge-list" id="mcp-tools-list-${index}"></div>
                    </div>
                </div>
            `;
            // 按钮通过闭包绑定，避免服务名中的引号破坏内联 onclick 字符串
            card.querySelector('.mcp-save-btn').onclick = () => updateSingleMcp(server.id, server.name, index);
            card.querySelector('.mcp-test-btn').onclick = () => testMcpServerTools(server, index);
            const deleteBtn = card.querySelector('.delete-btn');
            deleteBtn.title = t('common.purge');
            deleteBtn.onclick = () => removeMcpServer(server.id, name);
            mcpListContainer.appendChild(card);
        });
    } catch (e) {
        mcpListContainer.innerHTML = errorListHtml('common.fetchFailed');
    }
}

async function submitMcpServer() {
    const name = document.getElementById('mcpKey').value.trim();
    const description = document.getElementById('mcpDesc').value.trim();
    const type = document.getElementById('mcpType').value;
    if (!name) {
        showToast(t('mcp.nameRequired'), 'error');
        return;
    }

    let bodyPayload = {name: name, description: description};
    if (type === 'stdio') {
        bodyPayload.command = document.getElementById('mcpCommand').value.trim();
        const argsStr = document.getElementById('mcpArgs').value.trim();
        bodyPayload.args = argsStr ? argsStr.split(',').map(a => a.trim()) : [];
    } else {
        bodyPayload.url = document.getElementById('mcpUrl').value.trim();
        const headersStr = document.getElementById('mcpHeaders').value.trim();
        if (headersStr) {
            try {
                bodyPayload.headers = JSON.parse(headersStr);
            } catch (e) {
                showToast(t('mcp.headersInvalid'), 'error');
                return;
            }
        } else {
            bodyPayload.headers = {};
        }
    }

    try {
        const response = await fetch(`/mcp-server/${encodeURIComponent(type)}`, {
            method: 'POST',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify(bodyPayload)
        });
        if (response.ok) {
            toggleAddMcpPanel();
            document.getElementById('mcpKey').value = '';
            document.getElementById('mcpDesc').value = '';
            document.getElementById('mcpUrl').value = '';
            document.getElementById('mcpHeaders').value = '';
            document.getElementById('mcpCommand').value = '';
            document.getElementById('mcpArgs').value = '';
            await fetchMcpRegistry();
        }
    } catch (e) {
        showToast(t('mcp.regRejected'), 'error');
    }
}

async function updateSingleMcp(id, name, index) {
    const type = document.getElementById(`mcp-type-${index}`).value;
    const nameInput = document.getElementById(`mcp-name-${index}`);
    name = nameInput ? nameInput.value.trim() : name;
    if (!name) {
        showToast(t('mcp.nameRequired'), 'error');
        return;
    }
    const description = document.getElementById(`mcp-desc-${index}`).value.trim();
    let bodyPayload = {name: name, description: description};

    if (type === 'stdio') {
        bodyPayload.command = document.getElementById(`mcp-command-${index}`).value.trim();
        const argsStr = document.getElementById(`mcp-args-${index}`).value.trim();
        bodyPayload.args = argsStr ? argsStr.split(',').map(a => a.trim()).filter(a => a) : [];
    } else {
        bodyPayload.url = document.getElementById(`mcp-url-${index}`).value.trim();
        const headersStr = document.getElementById(`mcp-headers-${index}`).value.trim();
        if (headersStr) {
            try {
                bodyPayload.headers = JSON.parse(headersStr);
            } catch (e) {
                showToast(t('mcp.headersInvalid'), 'error');
                return;
            }
        } else {
            bodyPayload.headers = {};
        }
    }

    try {
        // server.protocol_type 为下划线形式（streamable_http），接口路径使用连字符形式（streamable-http）
        const response = await fetch(`/mcp-server/${id}/${encodeURIComponent(type.replace('_', '-'))}`, {
            method: 'PUT',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify(bodyPayload)
        });
        if (response.ok) {
            showToast(t('mcp.synced', {name: name}));
            await fetchMcpRegistry();
        }
    } catch (e) {
        showToast(t('common.syncCrashed'), 'error');
    }
}

function removeMcpServer(id, name) {
    showConfirmDialog({
        title: t('mcp.purgeTitle'),
        text: t('mcp.purgeText', {name: name}),
        onConfirm: async () => {
            try {
                const response = await fetch(`/mcp-server/${id}`, {method: 'DELETE'});
                if (response.ok) {
                    await fetchMcpRegistry();
                }
            } catch (e) {
                showToast(t('common.purgeFailure'), 'error');
            }
        }
    });
}

async function testMcpServerTools(targetServer, index) {
    const card = document.getElementById(`mcp-card-${index}`);
    const detailsZone = document.getElementById(`mcp-tools-zone-${index}`);
    const listContainer = document.getElementById(`mcp-tools-list-${index}`);

    if (!card.hasAttribute('open')) {
        toggleCardOpen(card);
    }
    detailsZone.style.display = 'block';
    listContainer.innerHTML = SKELETON_HTML;

    try {
        const response = await fetch(`/mcp-server/${targetServer.id}/tool/list`, {method: 'POST'});
        const tools = await response.json();
        listContainer.innerHTML = '';
        if (!tools || tools.length === 0) {
            listContainer.innerHTML = `<div class="text-hint">${t('mcp.connectedNoTools')}</div>`;
            return;
        }
        tools.forEach(tool => {
            const item = document.createElement('div');
            item.className = 'mcp-tool-item';
            item.innerHTML = `
                <div class="mcp-tool-name">/tools::${escapeHtml(tool.name)}</div>
                <div class="mcp-tool-desc">${escapeHtml(tool.description || t('mcp.noManifest'))}</div>
            `;
            listContainer.appendChild(item);
        });
    } catch (e) {
        listContainer.innerHTML = `<div class="text-error-mono">${t('mcp.sessionCrashed')}</div>`;
    }
}

