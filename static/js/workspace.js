// ==========================================
// 工作空间 (CWD) 及目录层级导航引擎
// ==========================================
// 工作目录历史记忆（后端 Web 存储）
const WORKDIR_HISTORY_KEY = 'openagents_recent_workdirs';
const WORKDIR_HISTORY_LIMIT = 10;

// 工作目录历史内存缓存（打开目录弹窗时从后端拉取）
let workdirHistoryCache = [];

// 从后端加载工作目录历史到内存缓存
async function loadWorkdirHistory() {
    const raw = await getWebStorage(WORKDIR_HISTORY_KEY);
    try {
        const list = raw ? JSON.parse(raw) : [];
        workdirHistoryCache = Array.isArray(list) ? list : [];
    } catch (e) {
        workdirHistoryCache = [];
    }
}

async function loadDirList(path) {
    const dirListContainer = document.getElementById('dirList');
    const pathDisplay = document.getElementById('currentPathDisplay');
    dirListContainer.innerHTML = SKELETON_HTML;
    try {
        const response = await fetch(`/dir/list?path=${encodeURIComponent(path)}`);
        const data = await response.json();
        tempSelectedPath = data.current_path;
        pathDisplay.textContent = `> ${data.current_path}`;
        dirListContainer.innerHTML = "";
        if (data.parent_path) {
            dirListContainer.appendChild(createDirItem(t('workspace.upLevel'), data.parent_path));
        }
        data.directories.forEach(dir => {
            dirListContainer.appendChild(createDirItem(dir.name, dir.path));
        });
    } catch (e) {
        dirListContainer.innerHTML = errorListHtml('workspace.connFailed');
    }
}

async function initDefaultWorkspace() {
    try {
        const response = await fetch(`/dir/list?path=`);
        const data = await response.json();
        if (data.current_path) {
            updateWorkspaceUI(data.current_path);
        }
    } catch (e) {
        // 默认静默降级逻辑
    }
}

function getWorkdirHistory() {
    return workdirHistoryCache;
}

function saveWorkdirToHistory(path) {
    if (!path) {
        return;
    }
    // 去重后置顶，超出上限截断
    const list = getWorkdirHistory().filter(item => item !== path);
    list.unshift(path);
    workdirHistoryCache = list.slice(0, WORKDIR_HISTORY_LIMIT);
    setWebStorage(WORKDIR_HISTORY_KEY, JSON.stringify(workdirHistoryCache));
}

function removeWorkdirFromHistory(path) {
    workdirHistoryCache = getWorkdirHistory().filter(item => item !== path);
    setWebStorage(WORKDIR_HISTORY_KEY, JSON.stringify(workdirHistoryCache));
    renderWorkdirHistory();
}

function renderWorkdirHistory() {
    const section = document.getElementById('dirHistorySection');
    const listContainer = document.getElementById('dirHistoryList');
    const list = getWorkdirHistory();
    listContainer.innerHTML = '';
    if (list.length === 0) {
        section.style.display = 'none';
        return;
    }
    section.style.display = 'block';
    list.forEach(path => {
        const item = document.createElement('div');
        item.className = 'dir-history-item';
        item.title = path;
        const pathSpan = document.createElement('span');
        pathSpan.className = 'dir-history-path';
        pathSpan.textContent = path;
        pathSpan.onclick = () => confirmHistorySelection(path);
        const removeBtn = document.createElement('button');
        removeBtn.className = 'delete-btn';
        removeBtn.style.opacity = '0.5';
        removeBtn.title = t('modal.removeHistory');
        removeBtn.innerHTML = DELETE_SVG;
        removeBtn.onclick = (e) => {
            e.stopPropagation();
            removeWorkdirFromHistory(path);
        };
        item.appendChild(pathSpan);
        item.appendChild(removeBtn);
        listContainer.appendChild(item);
    });
}

// 点击历史记录：直接选中该目录并走确认流程
function confirmHistorySelection(path) {
    saveWorkdirToHistory(path);
    if (dirConfirmCallback) {
        dirConfirmCallback(path);
    } else {
        updateWorkspaceUI(path);
    }
    closeDirModal();
}

// 打开目录选择弹窗：callback 为空时确认后写入对话工作目录（默认行为），否则回调处理选中路径
async function openDirModal(callback, currentPath) {
    dirConfirmCallback = callback || null;
    document.getElementById('dirModalOverlay').style.display = 'block';
    // 从后端加载工作目录历史后再渲染
    await loadWorkdirHistory();
    renderWorkdirHistory();
    await loadDirList(currentPath || "");
}

async function selectWorkspace() {
    await openDirModal(null, currentWorkdir);
}

function handleManualJump() {
    const targetPath = manualPathInput.value.trim();
    if (targetPath) {
        loadDirList(targetPath);
    }
}

function createDirItem(name, path) {
    const div = document.createElement('div');
    div.className = 'dir-item';
    div.innerHTML = `<span>[DIR]</span><span>${escapeHtml(name)}</span>`;
    div.onclick = () => loadDirList(path);
    return div;
}

function confirmDirSelection() {
    saveWorkdirToHistory(tempSelectedPath);
    if (dirConfirmCallback) {
        dirConfirmCallback(tempSelectedPath);
    } else {
        updateWorkspaceUI(tempSelectedPath);
    }
    closeDirModal();
}

function updateWorkspaceUI(path) {
    currentWorkdir = path;
    // 同步刷新对话页与任务创建面板的工作目录显示（复用同一组件状态）
    ['workspaceDisplay', 'taskWorkspaceDisplay', 'cronWorkspaceDisplay'].forEach(id => {
        const display = document.getElementById(id);
        display.textContent = path || t('input.unset');
        display.title = path;
    });
}

function closeDirModal() {
    document.getElementById('dirModalOverlay').style.display = 'none';
}
