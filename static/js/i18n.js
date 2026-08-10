// ==========================================
// I18N 国际化引擎（中英文切换，与主题方案同构）
// ==========================================
const I18N_STORAGE_KEY = 'openagents_lang';

// 文案字典：en/zh 两组，key 按模块分组，支持 {name} 占位符替换
const I18N = {
    en: {
        lang: {toggleTitle: 'Switch Language'},
        sidebar: {newDialog: '+ New Dialog', history: 'History'},
        nav: {dialog: 'DIALOG', task: 'TASK', cron: 'CRON', agent: 'AGENT', skill: 'SKILL', mcp: 'MCP', config: 'CONFIG'},
        header: {newTrace: 'NEW CONVERSATION', switchTheme: 'Switch Theme', coreTask: 'TASK PIPELINE', coreCron: 'SCHEDULE', coreSkill: 'SKILLS', coreMcp: 'MCP SERVERS', coreAgent: 'AGENTS', coreConfig: 'CONFIGURATION'},
        empty: {title: 'Start a Conversation', text: 'Start a new conversation or select an existing one from the sidebar to continue.', hint: 'Press Enter to send / Shift+Enter for newline'},
        common: {save: 'Save', cancel: 'Cancel', abort: 'Cancel', purge: 'Delete', refresh: 'Refresh', start: 'Start', review: 'Review', edit: 'Edit', fetchFailed: 'LOAD FAILED', requiredMissing: 'REQUIRED FIELDS MISSING.', creationFault: 'CREATE FAILED', purgeFailure: 'DELETE FAILED', syncCrashed: 'OPERATION FAILED', none: 'NONE', inheritedEnv: 'Inherited Environment', description: 'Description'},
        task: {viewTitle: 'Task Pipeline Management', newTask: '+ New Task', title: 'Task Title', content: 'Task Content', workDir: 'Working Directory', candidateAgents: 'Candidate Agents', titlePlaceholder: 'e.g. Daily Code Review Pipeline', contentPlaceholder: '# Enter task description...', launch: 'Launch Pipeline', stageProgress: 'Stage Progress', empty: 'NO TASK PIPELINES FOUND', noAgents: 'NO REGISTERED AGENTS', notStarted: 'PIPELINE NOT STARTED. NO STAGES YET.', generating: '⚡ Generating... Click to view live stream', reviewPlaceholder: 'Enter review feedback...', reviewRequired: 'REVIEW FEEDBACK IS REQUIRED.', reviewFault: 'REVIEW FAILED', noMessages: 'NO MESSAGES', agentRequired: 'AGENT IS REQUIRED.', needCandidate: 'NO CANDIDATE AGENTS ON THIS TASK', launched: 'TASK STARTED.', alreadyRunning: 'TASK ALREADY RUNNING', startFault: 'START FAILED', purgeTitle: 'DELETE TASK', purgeText: 'Delete task [{name}]? All attached stage conversations will be permanently deleted.'},
        agent: {viewTitle: 'Registered Agent Roster', newAgent: '+ New Agent', name: 'Agent Name', prompt: 'System Prompt', namePlaceholder: 'e.g. Code Reviewer', descPlaceholder: 'e.g. Reviews pull requests and reports issues', promptPlaceholder: '# Enter agent system prompt...', empty: 'NO REGISTERED AGENTS FOUND', rosterCrashed: 'LOAD FAILED', nameRequired: 'AGENT NAME IS REQUIRED.', providerRequired: 'MODEL PROVIDER IS REQUIRED.', commitFailure: 'SAVE FAILED', synced: 'AGENT [{name}] SAVED.', purgeTitle: 'DELETE AGENT', purgeText: 'Delete agent [{name}]? Its linked conversations will be detached.'},
        cron: {viewTitle: 'Scheduled Cron Processes', newCron: '+ New Cron', name: 'Task Name', namePlaceholder: 'e.g. Daily Market Scraper', workDirPlaceholder: 'e.g. /home/project/miniclaw', agent: 'Agent', noAgent: '-- None --', minute: 'Min', hour: 'Hour', day: 'Day', month: 'Month', week: 'Week', contentLabel: 'Execution Content / Script Context', contentPlaceholder: '# Enter detailed script steps...', empty: 'NO SCHEDULED CRON PROCESSES FOUND', unnamed: 'Unnamed Task', workingDir: 'Working Dir', nextFire: 'Next Fire', triggerSpec: 'Trigger Spec', execContent: 'Execution Content', suspended: 'Suspended', enabled: 'ENABLED', disabled: 'DISABLED', enable: 'Enable', disable: 'Disable', synced: 'CRON TASK [{name}] SAVED.', purgeTitle: 'DELETE CRON TASK', purgeText: 'Delete scheduled task [{name}]? It will no longer run.', purgeFailed: 'DELETE FAILED', formatHint: 'Fields accept digits and * , - /  (e.g. 0 9 * * * = every day at 09:00)', invalidFormat: 'CRON FIELDS MAY ONLY CONTAIN DIGITS AND * , - /'},
        skill: {viewTitle: 'Registered Agent Skills', empty: 'NO SKILLS EXTRACTED', unnamed: 'Unnamed Skill', noDesc: 'No description assigned.', fsPath: 'FS Path', sourceManifest: 'Source Manifest'},
        mcp: {viewTitle: 'Model Context Protocol Servers', newServer: '+ New Server', keyName: 'Server Name', keyPlaceholder: 'e.g. vserver_wind_financial_data', descPlaceholder: 'e.g. Wind Financial Data Provider', protocolArch: 'Protocol Type', urlEndpoint: 'Connection URL Endpoint', headersLabel: 'HTTP Headers Context (JSON Format)', command: 'Execute Command', args: 'Arguments (Comma Separated)', empty: 'NO REGISTERED MCP CONTEXTS', localContext: 'Local Context', testProbe: 'Test Connection', protocolType: 'Protocol Type', targetUrl: 'Target URL', httpHeaders: 'HTTP Context Headers (JSON)', cmdExec: 'Command Execution', exposedCaps: 'Exposed Capabilities Registry', topologyCrashed: 'LOAD FAILED', nameRequired: 'SERVER NAME IS REQUIRED.', headersInvalid: 'HEADERS MUST BE A VALID JSON STRING.', regRejected: 'CREATE FAILED', synced: 'MCP SERVER [{name}] SAVED.', purgeTitle: 'DELETE MCP SERVER', purgeText: 'Delete MCP server [{name}]? The tools it provides will no longer be available.', connectedNoTools: 'CONNECTED. NO TOOLS EXPOSED.', noManifest: 'No description provided.', sessionCrashed: 'CONNECTION REFUSED'},
        config: {viewTitle: 'System Runtime Configurations', newProvider: '+ New Provider', newProviderCard: '+ New Model Provider', providerName: 'Provider Name', providerNamePlaceholder: 'e.g. deepseek', protocolType: 'Protocol Type', baseUrl: 'Base URL Endpoint', secretToken: 'Secret Token (API Key)', settingsFault: 'LOAD FAILED', endpointMissing: 'Endpoint missing', nameRequired: 'PROVIDER NAME IS REQUIRED.', injectionFault: 'CREATE FAILED', synced: 'PROVIDER [{name}] SAVED.', purgeTitle: 'DELETE MODEL PROVIDER', purgeText: 'Delete model provider [{name}]? Agents using this provider will stop working.'},
        input: {cwdTitle: 'Set Directory Context', unset: 'UNSET', agentTitle: 'Select Agent', providerTitle: 'Select Provider', modelTitle: 'Select Model', thinkingTitle: 'Thinking Mode', placeholder: 'Enter instructions...', execute: 'Execute', contextLocked: 'Conversation started. Directory and agent are locked.', cwdLabel: 'WORKDIR:', modelListUnavailable: 'MODEL LIST UNAVAILABLE. TYPE THE MODEL NAME MANUALLY.'},
        modal: {workspaceContext: 'Workspace Context', pathPlaceholder: 'Input path...', go: 'GO', recentWorkdirs: 'Recent Workdirs', confirmPath: 'Confirm Path', removeHistory: 'Remove from history'},
        stream: {purgeTitle: 'DELETE CONVERSATION', purgeText: 'Delete conversation [{name}]? This action cannot be undone.', startFailed: 'START FAILED', configMissing: 'PLEASE SELECT A PROVIDER AND ENTER A MODEL FIRST.', unknownError: 'Unknown Error', thoughtProcess: 'Thought Process', callPrefix: 'Call', tool: 'Tool', toolError: 'Error', toolResult: 'Result', usageIn: 'in', usageOut: 'out', usageCache: 'cache', usageTotal: 'this turn'},
        workspace: {upLevel: '.. (UP LEVEL)', connFailed: 'ERR: CONNECTION FAILED'}
    },
    zh: {
        lang: {toggleTitle: '切换语言'},
        sidebar: {newDialog: '+ 新建会话', history: '历史会话'},
        nav: {dialog: '会话', task: '任务', cron: '定时', agent: '智能体', skill: '技能', mcp: 'MCP', config: '配置'},
        header: {newTrace: '新会话', switchTheme: '切换主题', coreTask: '任务流水线', coreCron: '定时排程', coreSkill: '技能清单', coreMcp: 'MCP 服务', coreAgent: '智能体名册', coreConfig: '系统配置'},
        empty: {title: '开始新会话', text: '开始一个新会话，或从侧边栏选择已有会话继续。', hint: '按 Enter 发送 / Shift+Enter 换行'},
        common: {save: '保存', cancel: '取消', abort: '取消', purge: '删除', refresh: '刷新', start: '启动', review: '审核', edit: '编辑', fetchFailed: '加载失败', requiredMissing: '必填字段缺失。', creationFault: '创建失败', purgeFailure: '删除失败', syncCrashed: '操作失败', none: '无', inheritedEnv: '继承环境', description: '描述'},
        task: {viewTitle: '任务流水线管理', newTask: '+ 新建任务', title: '任务标题', content: '任务内容', workDir: '工作目录', candidateAgents: '候选智能体', titlePlaceholder: '例如：每日代码审查流水线', contentPlaceholder: '# 输入任务描述...', launch: '启动流水线', stageProgress: '阶段进展', empty: '暂无任务流水线', noAgents: '未注册智能体', notStarted: '流水线未启动，暂无阶段。', generating: '⚡ 生成中...点击查看实时流', reviewPlaceholder: '输入审核意见...', reviewRequired: '请填写审核意见。', reviewFault: '审核提交失败', noMessages: '暂无消息', agentRequired: '请选择智能体。', needCandidate: '该任务未配置候选智能体', launched: '任务已启动。', alreadyRunning: '任务已在运行中', startFault: '启动失败', purgeTitle: '删除任务', purgeText: '确定要删除任务 [{name}] 吗？其关联的所有阶段会话将被永久删除。'},
        agent: {viewTitle: '已注册智能体名册', newAgent: '+ 新建智能体', name: '智能体名称', prompt: '系统提示词', namePlaceholder: '例如：代码审查员', descPlaceholder: '例如：审查 PR 并报告问题', promptPlaceholder: '# 输入智能体系统提示词...', empty: '暂无已注册智能体', rosterCrashed: '加载失败', nameRequired: '智能体名称为必填项。', providerRequired: '必须选择模型供应商。', commitFailure: '保存失败', synced: '智能体 [{name}] 已保存。', purgeTitle: '删除智能体', purgeText: '确定要删除智能体 [{name}] 吗？其关联会话将被解绑。'},
        cron: {viewTitle: '定时任务管理', newCron: '+ 新建定时任务', name: '任务名称', namePlaceholder: '例如：每日行情抓取', workDirPlaceholder: '例如：/home/project/miniclaw', agent: '智能体', noAgent: '-- 无 --', minute: '分', hour: '时', day: '日', month: '月', week: '周', contentLabel: '执行内容 / 脚本上下文', contentPlaceholder: '# 输入详细脚本步骤...', empty: '暂无定时任务', unnamed: '未命名任务', workingDir: '工作目录', nextFire: '下次触发', triggerSpec: '触发规则', execContent: '执行内容', suspended: '已暂停', enabled: '已启用', disabled: '已禁用', enable: '启用', disable: '禁用', synced: '定时任务 [{name}] 已保存。', purgeTitle: '删除定时任务', purgeText: '确定要删除定时任务 [{name}] 吗？删除后将不再执行。', purgeFailed: '删除失败', formatHint: '支持数字及 * , - / 符号（如 0 9 * * * 表示每天 09:00）', invalidFormat: '定时字段只能包含数字和 * , - / 符号。'},
        skill: {viewTitle: '已注册智能体技能', empty: '未提取到技能', unnamed: '未命名技能', noDesc: '未分配描述。', fsPath: '文件路径', sourceManifest: '源清单'},
        mcp: {viewTitle: 'Model Context Protocol 服务', newServer: '+ 新建服务', keyName: '服务名称', keyPlaceholder: '例如：vserver_wind_financial_data', descPlaceholder: '例如：Wind 金融数据服务', protocolArch: '协议类型', urlEndpoint: '连接地址', headersLabel: 'HTTP 请求头（JSON 格式）', command: '执行命令', args: '参数（逗号分隔）', empty: '暂无已注册 MCP 服务', localContext: '本地上下文', testProbe: '测试连接', protocolType: '协议类型', targetUrl: '目标地址', httpHeaders: 'HTTP 请求头（JSON）', cmdExec: '执行命令', exposedCaps: '暴露的能力清单', topologyCrashed: '加载失败', nameRequired: '服务名称为必填项。', headersInvalid: '请求头必须是合法的 JSON 字符串。', regRejected: '创建失败', synced: 'MCP 服务 [{name}] 已保存。', purgeTitle: '删除 MCP 服务', purgeText: '确定要删除 MCP 服务 [{name}] 吗？其提供的工具将不再可用。', connectedNoTools: '连接成功，未暴露任何工具。', noManifest: '未提供说明。', sessionCrashed: '连接被拒绝'},
        config: {viewTitle: '系统运行配置', newProvider: '+ 新建供应商', newProviderCard: '+ 新建模型供应商', providerName: '供应商名称', providerNamePlaceholder: '例如：deepseek', protocolType: '协议类型', baseUrl: '基础地址', secretToken: '密钥（API Key）', settingsFault: '加载失败', endpointMissing: '缺少端点地址', nameRequired: '供应商名称为必填项。', injectionFault: '创建失败', synced: '供应商 [{name}] 已保存。', purgeTitle: '删除模型供应商', purgeText: '确定要删除供应商 [{name}] 吗？使用该供应商的智能体将无法正常工作。'},
        input: {cwdTitle: '设置目录上下文', unset: '未设置', agentTitle: '选择智能体', providerTitle: '选择供应商', modelTitle: '选择模型', thinkingTitle: '思考模式', placeholder: '输入指令...', execute: '执行', contextLocked: '会话已创建，工作目录与智能体不可修改。', cwdLabel: '工作目录：', modelListUnavailable: '无法获取模型列表，可直接输入模型名称。'},
        modal: {workspaceContext: '工作目录上下文', pathPlaceholder: '输入路径...', go: '跳转', recentWorkdirs: '最近工作目录', confirmPath: '确认路径', removeHistory: '从历史中移除'},
        stream: {purgeTitle: '删除会话', purgeText: '确定要删除会话 [{name}] 吗？该操作不可恢复。', startFailed: '启动失败', configMissing: '请先选择供应商并填写模型。', unknownError: '未知错误', thoughtProcess: '思考过程', callPrefix: '调用', tool: '工具', toolError: '错误', toolResult: '结果', usageIn: '输入', usageOut: '输出', usageCache: '缓存', usageTotal: '本轮'},
        workspace: {upLevel: '..（上一级）', connFailed: '错误：连接失败'}
    }
};

// 当前语言（初始化函数中赋值）
let currentLang = 'en';

// 按当前语言取文案：缺失时回退英文，再缺失回退 key 本身；params 用于替换 {name} 占位符
function t(key, params) {
    const segments = key.split('.');
    let node = I18N[currentLang];
    for (const segment of segments) {
        node = node ? node[segment] : undefined;
    }
    if (typeof node !== 'string') {
        node = I18N.en;
        for (const segment of segments) {
            node = node ? node[segment] : undefined;
        }
    }
    let text = typeof node === 'string' ? node : key;
    if (params) {
        Object.keys(params).forEach(name => {
            text = text.replaceAll(`{${name}}`, String(params[name]));
        });
    }
    return text;
}

// 批量刷新静态 DOM：data-i18n 文本 / data-i18n-placeholder 占位 / data-i18n-title 悬停提示
function applyStaticI18n() {
    document.querySelectorAll('[data-i18n]').forEach(el => {
        el.textContent = t(el.dataset.i18n);
    });
    document.querySelectorAll('[data-i18n-placeholder]').forEach(el => {
        el.placeholder = t(el.dataset.i18nPlaceholder);
    });
    document.querySelectorAll('[data-i18n-title]').forEach(el => {
        el.title = t(el.dataset.i18nTitle);
    });
}

// 同步语言切换按钮：显示目标语言（当前中文显示 EN，当前英文显示 中）
function syncLanguageButton() {
    const btn = document.getElementById('langToggleBtn');
    if (btn) {
        btn.textContent = currentLang === 'zh' ? 'EN' : '中';
    }
}

// 切换语言后重渲染当前视图的动态内容（会话页流式消息不重载，仅刷新 header 信息文本）
function refreshActiveViewI18n() {
    const activeView = document.querySelector('.view-container.active');
    if (!activeView) {
        return;
    }
    if (activeView.id === 'viewTask') {
        fetchTaskList();
    } else if (activeView.id === 'viewAgent') {
        fetchAgentRegistry();
    } else if (activeView.id === 'viewCron') {
        fetchCronTasks();
    } else if (activeView.id === 'viewSkill') {
        fetchSkillData();
    } else if (activeView.id === 'viewMcp') {
        fetchMcpRegistry();
    } else if (activeView.id === 'viewConfig') {
        fetchGlobalSettings();
    } else {
        conversationInfo.textContent = currentConversationId ? `ID: ${currentConversationId}` : t('header.newTrace');
    }
}

// 设置语言：持久化到后端 Web 存储并全量刷新页面文案
function setLanguage(lang) {
    if (lang !== 'zh' && lang !== 'en') {
        lang = 'en';
    }
    currentLang = lang;
    // 异步落库，不阻塞文案刷新
    setWebStorage(I18N_STORAGE_KEY, lang);
    document.documentElement.lang = lang === 'zh' ? 'zh-CN' : 'en';
    applyStaticI18n();
    syncLanguageButton();
    // 刷新 CWD 显示（空路径回退为本地化 UNSET 文案）
    updateWorkspaceUI(currentWorkdir);
    refreshActiveViewI18n();
}

// 语言切换按钮入口：中英直接互换
function toggleLanguage() {
    setLanguage(currentLang === 'zh' ? 'en' : 'zh');
}

// 初始化：先按浏览器语言应用（zh* 判中文，否则英文），再异步加载后端持久化语言，有记录时覆盖浏览器判断结果
(function initLanguage() {
    currentLang = (navigator.language || 'en').toLowerCase().startsWith('zh') ? 'zh' : 'en';
    document.documentElement.lang = currentLang === 'zh' ? 'zh-CN' : 'en';
    applyStaticI18n();
    syncLanguageButton();
    getWebStorage(I18N_STORAGE_KEY).then(savedLang => {
        if (savedLang && savedLang !== currentLang) {
            setLanguage(savedLang);
        }
    });
})();
