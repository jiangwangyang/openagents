// ==========================================
// MCP (Model Context Protocol) 服务注册面板
// ==========================================
const mcpListContainer = document.getElementById('mcpListContainer');
const addMcpPanel = document.getElementById('addMcpPanel');

// 全局 MCP 服务拓扑缓存（列表渲染与 Test Probe 探测共用）
let globalMcpCachedList = [];

function toggleAddMcpPanel() {
    addMcpPanel.style.display = addMcpPanel.style.display === 'none' ? 'flex' : 'none';
}

function adaptMcpFormFields() {
    const type = document.getElementById('mcpType').value;
    document.getElementById('mcpNetworkRow').style.display = (type === 'stdio') ? 'none' : 'flex';
    document.getElementById('mcpLocalRow').style.display = (type === 'stdio') ? 'flex' : 'none';
}

async function fetchMcpRegistry() {
    mcpListContainer.innerHTML = SKELETON_HTML;
    try {
        const response = await fetch('/mcp-server/list');
        globalMcpCachedList = await response.json();
        mcpListContainer.innerHTML = '';

        if (globalMcpCachedList.length === 0) {
            mcpListContainer.innerHTML = '<div style="padding:30px; text-align:center; border:1px dashed var(--border-hard); color:var(--slate-400)">NO REGISTERED MCP CONTEXTS</div>';
            return;
        }

        globalMcpCachedList.forEach((server, index) => {
            const name = server.name;
            const card = document.createElement('div');
            card.className = 'info-card';
            // 元素 id 使用列表索引生成（服务名可能包含引号等特殊字符）
            card.id = `mcp-card-${index}`;

            const snippet = server.url || (server.command ? `${server.command} ${server.args?.join(' ')}` : 'Local Context');
            const isStdio = server.type === 'stdio';

            card.innerHTML = `
                <div class="info-card-summary" onclick="toggleCardOpen(this.parentNode)">
                    <div class="info-card-main">
                        ${ARROW_SVG}
                        <span class="info-card-name" style="min-width:180px; max-width:280px;">${escapeHtml(name)}</span>
                        <span class="info-card-snippet">${escapeHtml(snippet)}</span>
                    </div>
                    <div style="display:flex; gap:8px; align-items:center;" onclick="event.stopPropagation();">
                        <button class="btn btn-sm send-button mcp-save-btn" style="height:28px; padding:0 8px; font-size:10px;">Save</button>
                        <button class="btn btn-sm send-button mcp-test-btn" style="height:28px;">Test Probe</button>
                        <button class="delete-btn" style="opacity:1; padding:6px;">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
                        </button>
                    </div>
                </div>
                <div class="info-card-details" style="display: none;">
                    <input type="hidden" id="mcp-type-${index}" value="${escapeHtml(server.type)}">
                    <div class="details-grid">
                        <div class="details-label">Protocol Type</div>
                        <div class="details-value"><code style="background:var(--inline-code-bg); padding:2px 6px; border-radius:2px;">${escapeHtml(server.type)}</code></div>

                        <div class="details-label">Description</div>
                        <div class="details-value">
                            <input type="text" id="mcp-desc-${index}" class="form-control" value="${escapeHtml(server.description || '')}">
                        </div>

                        ${!isStdio ? `
                            <div class="details-label">Target URL</div>
                            <div class="details-value">
                                <input type="text" id="mcp-url-${index}" class="form-control mono" value="${escapeHtml(server.url || '')}">
                            </div>
                            <div class="details-label">HTTP Context Headers (JSON)</div>
                            <div class="details-value">
                                <textarea id="mcp-headers-${index}" class="form-control mono" rows="3" style="resize: vertical; font-size:11px;">${escapeHtml(JSON.stringify(server.headers || {}, null, 2))}</textarea>
                            </div>
                        ` : `
                            <div class="details-label">Command Execution</div>
                            <div class="details-value">
                                <input type="text" id="mcp-command-${index}" class="form-control mono" value="${escapeHtml(server.command || '')}">
                            </div>
                            <div class="details-label">Arguments (Comma Separated)</div>
                            <div class="details-value">
                                <input type="text" id="mcp-args-${index}" class="form-control mono" value="${escapeHtml(server.args?.join(', ') || '')}">
                            </div>
                        `}

                        <div class="details-block-container" id="mcp-tools-zone-${index}" style="display:none;">
                            <div class="details-label" style="margin-bottom: 6px;">Exposed Capabilities Registry</div>
                            <div class="mcp-tool-badge-list" id="mcp-tools-list-${index}"></div>
                        </div>
                    </div>
                </div>
            `;
            // 按钮通过闭包绑定，避免服务名中的引号破坏内联 onclick 字符串
            card.querySelector('.mcp-save-btn').onclick = () => updateSingleMcp(name);
            card.querySelector('.mcp-test-btn').onclick = () => testMcpServerTools(name);
            card.querySelector('.delete-btn').onclick = () => removeMcpServer(name);
            mcpListContainer.appendChild(card);
        });
    } catch (e) {
        mcpListContainer.innerHTML = '<div style="padding:20px; color:var(--danger-color)">TOPOLOGY CAPTURE CRASHED</div>';
    }
}

async function submitMcpServer() {
    const name = document.getElementById('mcpKey').value.trim();
    const description = document.getElementById('mcpDesc').value.trim();
    const type = document.getElementById('mcpType').value;
    if (!name) {
        alert("SERVER UNIQUE NAME IS REQUIRED.");
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
                alert("HEADERS MUST BE A VALID JSON STRING.");
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
        alert("REGISTRATION REJECTED");
    }
}

async function updateSingleMcp(name) {
    // 元素 id 使用列表索引生成（服务名可能包含引号等特殊字符）
    const index = globalMcpCachedList.findIndex(s => s.name === name);
    if (index === -1) {
        return;
    }
    const type = document.getElementById(`mcp-type-${index}`).value;
    const description = document.getElementById(`mcp-desc-${index}`).value.trim();
    let bodyPayload = {description: description};

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
                alert("HEADERS MUST BE A VALID JSON STRING.");
                return;
            }
        } else {
            bodyPayload.headers = {};
        }
    }

    try {
        const response = await fetch(`/mcp-server/${encodeURIComponent(name)}/${encodeURIComponent(type)}`, {
            method: 'PUT',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify(bodyPayload)
        });
        if (response.ok) {
            alert(`MCP SERVER [${name}] SYNCHRONIZED.`);
            await fetchMcpRegistry();
        }
    } catch (e) {
        alert("SYNC CRASHED");
    }
}

function removeMcpServer(name) {
    showConfirmDialog({
        title: "PURGE MCP NODE",
        text: `Are you sure you want to completely eject MCP node [${name}] from system environment? Bridge tunnels attached will be disconnected instantly.`,
        onConfirm: async () => {
            try {
                const response = await fetch(`/mcp-server/${encodeURIComponent(name)}`, {method: 'DELETE'});
                if (response.ok) {
                    await fetchMcpRegistry();
                }
            } catch (e) {
                alert("PURGE FAILURE");
            }
        }
    });
}

async function testMcpServerTools(name) {
    const targetServer = globalMcpCachedList.find(s => s.name === name);
    if (!targetServer) {
        return;
    }

    const index = globalMcpCachedList.indexOf(targetServer);
    const card = document.getElementById(`mcp-card-${index}`);
    const detailsZone = document.getElementById(`mcp-tools-zone-${index}`);
    const listContainer = document.getElementById(`mcp-tools-list-${index}`);

    if (!card.hasAttribute('open')) {
        toggleCardOpen(card);
    }
    detailsZone.style.display = 'block';
    listContainer.innerHTML = SKELETON_HTML;

    try {
        const response = await fetch(`/mcp-server/${encodeURIComponent(targetServer.type)}/test`, {
            method: 'POST',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify(targetServer)
        });
        const tools = await response.json();
        listContainer.innerHTML = '';
        if (!tools || tools.length === 0) {
            listContainer.innerHTML = '<div style="font-size:12px; color:var(--slate-400); font-style:italic;">CONNECTED SUCCESSFULLY. ZERO TOOLS EXPOSED.</div>';
            return;
        }
        tools.forEach(tool => {
            const item = document.createElement('div');
            item.className = 'mcp-tool-item';
            item.innerHTML = `
                <div class="mcp-tool-name">/tools::${escapeHtml(tool.name)}</div>
                <div class="mcp-tool-desc">${escapeHtml(tool.description || 'No instruction manifest provided.')}</div>
            `;
            listContainer.appendChild(item);
        });
    } catch (e) {
        listContainer.innerHTML = `<div style="font-family:var(--font-mono); font-size:12px; color:var(--danger-color)">SESSION CRASHED: REFUSED CONNECTION</div>`;
    }
}

