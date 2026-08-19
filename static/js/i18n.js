// ==========================================
// I18N 国际化引擎（中英文切换，与主题方案同构）
// ==========================================

// ===== 1. 文案字典 =====
// en/zh 两组，key 按模块分组，支持 {name} 占位符替换
const I18N_STORAGE_KEY = 'openagents_lang';

const I18N = {
    en: {
        lang: {toggleTitle: 'Switch Language'},
        theme: {light: 'LIGHT', dark: 'DARK', ink: 'INK', sunset: 'SUNSET', aurora: 'AURORA', cyberpunk: 'CYBERPUNK', blackhole: 'BLACK HOLE'},
        sidebar: {newDialog: '+ New Dialog', history: 'History'},
        nav: {dialog: 'DIALOG', task: 'TASK', cron: 'CRON', agent: 'AGENT', skill: 'SKILL', mcp: 'MCP', config: 'CONFIG', dialogTip: 'Chat directly with an agent', taskTip: 'Multi-agent pipeline with staged execution and review', cronTip: 'Auto-run tasks on a cron schedule', agentTip: 'Register agents: model, prompt and behavior', skillTip: 'Skills distilled automatically from task runs', mcpTip: 'Register external tool servers for agents', configTip: 'Configure model providers (protocol, URL, API key)'},
        header: {newTrace: 'NEW CONVERSATION', switchTheme: 'Switch Theme', coreTask: 'TASK PIPELINE', coreCron: 'SCHEDULE', coreSkill: 'SKILLS', coreMcp: 'MCP SERVERS', coreAgent: 'AGENTS', coreConfig: 'CONFIGURATION'},
        empty: {title: 'Start a Conversation', text: 'Start a new conversation or select an existing one from the sidebar to continue.', hint: 'Press Enter to send / Shift+Enter for newline'},
        common: {save: 'Save', cancel: 'Cancel', abort: 'Cancel', purge: 'Delete', start: 'Start', sortAsc: '\u2191 ASC', sortDesc: '\u2193 DESC', sortToggle: 'Toggle sort order', expand: 'EXPAND', collapse: 'COLLAPSE', review: 'Review', fetchFailed: 'LOAD FAILED', requiredMissing: 'REQUIRED FIELDS MISSING.', creationFault: 'CREATE FAILED', purgeFailure: 'DELETE FAILED', syncCrashed: 'OPERATION FAILED', none: 'NONE', inheritedEnv: 'Inherited Environment', description: 'Description'},
        task: {viewTitle: 'Task Pipeline Management', newTask: '+ New Task', title: 'Task Title', content: 'Task Content', workDir: 'Working Directory', candidateAgents: 'Candidate Agents', titlePlaceholder: 'e.g. Daily Code Review Pipeline', contentPlaceholder: '# Enter task description...', stageProgress: 'Stage Progress', empty: 'NO TASK PIPELINES FOUND', noAgents: 'NO REGISTERED AGENTS', notStarted: 'PIPELINE NOT STARTED. NO STAGES YET.', generating: '⚡ Generating... Click to view live stream', reviewWaiting: '⏸ PIPELINE PAUSED. WAITING FOR YOUR REVIEW FEEDBACK:', reviewPlaceholder: 'Enter review feedback...', reviewRequired: 'REVIEW FEEDBACK IS REQUIRED.', reviewFault: 'REVIEW FAILED', noMessages: 'NO MESSAGES', agentRequired: 'AGENT IS REQUIRED.', needCandidate: 'NO CANDIDATE AGENTS ON THIS TASK', launched: 'TASK STARTED.', alreadyRunning: 'TASK ALREADY RUNNING', startFault: 'START FAILED', purgeTitle: 'DELETE TASK', purgeText: 'Delete task [{name}]? All attached stage conversations will be permanently deleted.', viewDesc: 'A task is a multi-agent pipeline: work is split into stages, dispatched to candidate agents, and can pause for your review. Register agents in the AGENT page first.', emptyHint: 'TIP: A task dispatches stages to candidate agents. Register at least one agent in the AGENT page, then click "+ New Task".', statusIdle: 'IDLE', statusRunning: 'RUNNING', statusReview: 'REVIEW', statusDone: 'DONE', statusFailed: 'INTERRUPTED', stageCount: '{count} STAGES', stop: 'Stop Task', stopped: 'TASK STOPPED.', stopFault: 'STOP FAILED', notRunning: 'TASK IS NOT RUNNING', submitRun: 'Submit & Run', launch: 'Start', launchPlaceholder: 'Start execution', resumePlaceholder: 'Continue execution...', failedPlaceholder: 'Task interrupted. Continue execution', complete: 'Complete', completePlaceholder: 'Done', resume: 'Continue', reviewNeeded: 'TASK [{name}] IS WAITING FOR YOUR REVIEW', runningNoPurge: 'CANNOT DELETE A RUNNING TASK', openInDialog: 'OPEN IN DIALOG'},
        agent: {viewTitle: 'Registered Agent Roster', newAgent: '+ New Agent', name: 'Agent Name', prompt: 'System Prompt', namePlaceholder: 'e.g. Code Reviewer', descPlaceholder: 'e.g. Reviews pull requests and reports issues', promptPlaceholder: '# Enter agent system prompt...', empty: 'NO REGISTERED AGENTS FOUND', nameRequired: 'AGENT NAME IS REQUIRED.', providerRequired: 'MODEL PROVIDER IS REQUIRED.', commitFailure: 'SAVE FAILED', synced: 'AGENT [{name}] SAVED.', purgeTitle: 'DELETE AGENT', purgeText: 'Delete agent [{name}]? Its linked conversations will be detached.', viewDesc: 'An agent binds a model provider, a system prompt and a behavior description, and can be used by DIALOG, TASK and CRON. Add a model provider in CONFIG first.', emptyHint: 'TIP: An agent needs a model provider. Add one in the CONFIG page first, then click "+ New Agent".'},
        cron: {viewTitle: 'Scheduled Cron Processes', newCron: '+ New Cron', name: 'Task Name', namePlaceholder: 'e.g. Daily Code Review', agent: 'Agent', minute: 'Min', hour: 'Hour', day: 'Day', month: 'Month', week: 'Weekday', contentLabel: 'Task Content', contentPlaceholder: '# Enter task description...', empty: 'NO SCHEDULED CRON PROCESSES FOUND', unnamed: 'Unnamed Task', workingDir: 'Working Dir', nextFire: 'Next Fire', triggerSpec: 'Trigger Spec', execContent: 'Task Content', suspended: 'Suspended', enabled: 'ENABLED', disabled: 'DISABLED', enable: 'Enable', disable: 'Disable', synced: 'CRON TASK [{name}] SAVED.', purgeTitle: 'DELETE CRON TASK', purgeText: 'Delete scheduled task [{name}]? It will no longer run.', formatHint: 'Field order: Min Hour Day Month Weekday. Each accepts digits and * , - /  (e.g. Min=0 Hour=9 = every day at 09:00)', invalidFormat: 'CRON FIELDS MAY ONLY CONTAIN DIGITS AND * , - /', execHistory: 'Execution History', notTriggered: 'NOT TRIGGERED YET. NO EXECUTION RECORDS.', viewDesc: 'Cron tasks fire automatically on a schedule (Min Hour Day Month Weekday) with no manual intervention. Execution history is kept on each card. Register an agent first.', emptyHint: 'TIP: Cron tasks run automatically on schedule. Register an agent in the AGENT page first, then click "+ New Cron".'},
        skill: {viewTitle: 'Registered Agent Skills', empty: 'NO SKILLS EXTRACTED', unnamed: 'Unnamed Skill', noDesc: 'No description assigned.', fsPath: 'FS Path', sourceManifest: 'Source Manifest', viewDesc: 'Skills are scanned and loaded automatically from ~/.openagents/skills and ~/.agents/skills under your home directory. Nothing to create here, this page is read-only.', emptyHint: 'No skills scanned. Place skill folders under ~/.openagents/skills or ~/.agents/skills in your home directory, then reopen this page.'},
        mcp: {viewTitle: 'Model Context Protocol Servers', newServer: '+ New Server', keyName: 'Server Name', keyPlaceholder: 'e.g. file_server', descPlaceholder: 'e.g. File Server', protocolArch: 'Protocol Type', urlEndpoint: 'Connection URL Endpoint', headersLabel: 'HTTP Headers Context (JSON Format)', command: 'Execute Command', args: 'Arguments (Comma Separated)', empty: 'NO REGISTERED MCP CONTEXTS', localContext: 'Local Context', testProbe: 'Test Connection', protocolType: 'Protocol Type', targetUrl: 'Target URL', httpHeaders: 'HTTP Context Headers (JSON)', cmdExec: 'Command Execution', exposedCaps: 'Exposed Capabilities Registry', nameRequired: 'SERVER NAME IS REQUIRED.', headersInvalid: 'HEADERS MUST BE A VALID JSON STRING.', regRejected: 'CREATE FAILED', synced: 'MCP SERVER [{name}] SAVED.', purgeTitle: 'DELETE MCP SERVER', purgeText: 'Delete MCP server [{name}]? The tools it provides will no longer be available.', connectedNoTools: 'CONNECTED. NO TOOLS EXPOSED.', noManifest: 'No description provided.', sessionCrashed: 'CONNECTION REFUSED', viewDesc: 'Register MCP tool servers (remote HTTP or local command). Once connected, the tools they expose become available to agents.', emptyHint: 'TIP: MCP servers plug external tools (files, databases, APIs) into agents. Click "+ New Server" to register the first one.'},
        config: {viewTitle: 'System Runtime Configurations', newProvider: '+ New Provider', newProviderCard: '+ New Model Provider', providerName: 'Provider Name', providerNamePlaceholder: 'e.g. deepseek', protocolType: 'Protocol Type', baseUrl: 'Base URL Endpoint', secretToken: 'Secret Token (API Key)', endpointMissing: 'Endpoint missing', nameRequired: 'PROVIDER NAME IS REQUIRED.', injectionFault: 'CREATE FAILED', synced: 'PROVIDER [{name}] SAVED.', purgeTitle: 'DELETE MODEL PROVIDER', purgeText: 'Delete model provider [{name}]? Agents using this provider will stop working.', viewDesc: 'Model providers (protocol / base URL / API key) power every agent. Adding a provider is step one before creating agents.', empty: 'NO MODEL PROVIDERS REGISTERED', emptyHint: 'STEP ONE: Click "+ New Provider" to add a model provider, required before creating agents or starting conversations.'},
        input: {cwdTitle: 'Set Directory Context', unset: 'UNSET', agentTitle: 'Select Agent', providerTitle: 'Select Provider', modelTitle: 'Select Model', thinkingTitle: 'Thinking Mode', placeholder: 'Enter instructions...', readonlyPlaceholder: 'Read-only conversation from task/cron. Sending is disabled.', execute: 'Send', stop: 'Stop', charLimit: 'INPUT LIMIT REACHED ({max} CHARS). EXCESS CONTENT WAS DISCARDED.', contextLocked: 'Conversation started. Directory and agent are locked.', cwdLabel: 'WORKDIR:', modelListUnavailable: 'MODEL LIST UNAVAILABLE. TYPE THE MODEL NAME MANUALLY.', promptSourceAgent: 'PROMPT: AGENT [{name}]', promptSourceFile: 'PROMPT: {path}', promptSourcePending: 'PROMPT: WILL LOAD FROM {path} ON START (IF EXISTS)', promptSourceNone: 'PROMPT: NONE'},
        modal: {workspaceContext: 'Workspace Context', pathPlaceholder: 'Input path...', go: 'GO', recentWorkdirs: 'Recent Workdirs', confirmPath: 'Confirm Path', removeHistory: 'Remove from history'},
        stream: {purgeTitle: 'DELETE CONVERSATION', purgeText: 'Delete conversation [{name}]? This action cannot be undone.', startFailed: 'START FAILED', configMissing: 'PLEASE SELECT A PROVIDER AND ENTER A MODEL FIRST.', workdirMissing: 'PLEASE SET A WORKING DIRECTORY (WORKDIR) FIRST.', unknownError: 'Unknown Error', thoughtProcess: 'Thought Process', systemPrompt: 'System Prompt', callPrefix: 'Call', tool: 'Tool', toolError: 'Error', toolResult: 'Result', usageIn: 'in', usageOut: 'out', usageCache: 'cache', usageTotal: 'this turn', stopped: '\u23f8 MANUALLY STOPPED', stopFailed: 'STOP FAILED'},
        workspace: {upLevel: '.. (UP LEVEL)', connFailed: 'ERR: CONNECTION FAILED'}
    },
    zh: {
        lang: {toggleTitle: '切换语言'},
        theme: {light: '浅色', dark: '深色', ink: '水墨', sunset: '落日', aurora: '极光', cyberpunk: '赛博朋克', blackhole: '黑洞'},
        sidebar: {newDialog: '+ 新建会话', history: '历史会话'},
        nav: {dialog: '会话', task: '任务', cron: '定时', agent: '智能体', skill: '技能', mcp: 'MCP', config: '配置', dialogTip: '与智能体直接对话', taskTip: '多智能体协作流水线，分阶段执行并可审核', cronTip: '按 cron 规则自动触发任务', agentTip: '注册智能体：模型、提示词与行为', skillTip: '任务执行中自动沉淀的技能', mcpTip: '注册外部工具服务，供智能体调用', configTip: '配置模型供应商（协议/地址/密钥）'},
        header: {newTrace: '新会话', switchTheme: '切换主题', coreTask: '任务流水线', coreCron: '定时排程', coreSkill: '技能清单', coreMcp: 'MCP 服务', coreAgent: '智能体名册', coreConfig: '系统配置'},
        empty: {title: '开始新会话', text: '开始一个新会话，或从侧边栏选择已有会话继续。', hint: '按 Enter 发送 / Shift+Enter 换行'},
        common: {save: '保存', cancel: '取消', abort: '取消', purge: '删除', start: '启动', sortAsc: '↑ 升序', sortDesc: '↓ 降序', sortToggle: '切换排序', expand: '展开', collapse: '收起', review: '审核', fetchFailed: '加载失败', requiredMissing: '必填字段缺失。', creationFault: '创建失败', purgeFailure: '删除失败', syncCrashed: '操作失败', none: '无', inheritedEnv: '继承环境', description: '描述'},
        task: {viewTitle: '任务流水线管理', newTask: '+ 新建任务', title: '任务标题', content: '任务内容', workDir: '工作目录', candidateAgents: '候选智能体', titlePlaceholder: '例如：每日代码审查流水线', contentPlaceholder: '# 输入任务描述...', stageProgress: '阶段进展', empty: '暂无任务流水线', noAgents: '未注册智能体', notStarted: '流水线未启动，暂无阶段。', generating: '⚡ 生成中...点击查看实时流', reviewWaiting: '⏸ 流水线已暂停，等待您的审核意见：', reviewPlaceholder: '输入审核意见...', reviewRequired: '请填写审核意见。', reviewFault: '审核提交失败', noMessages: '暂无消息', agentRequired: '请选择智能体。', needCandidate: '该任务未配置候选智能体', launched: '任务已启动。', alreadyRunning: '任务已在运行中', startFault: '启动失败', purgeTitle: '删除任务', purgeText: '确定要删除任务 [{name}] 吗？其关联的所有阶段会话将被永久删除。', viewDesc: '任务是多智能体协作的流水线：工作被拆分为阶段，分派给候选智能体执行，关键节点可暂停等待您审核。使用前请先在「智能体」页注册智能体。', emptyHint: '提示：任务会将各阶段分派给候选智能体执行。请先在「智能体」页注册至少一个智能体，再点击右上角「+ 新建任务」。', statusIdle: '待启动', statusRunning: '运行中', statusReview: '待审核', statusDone: '已完成', statusFailed: '异常中断', stageCount: '{count} 个阶段', stop: '停止任务', stopped: '任务已停止。', stopFault: '停止失败', notRunning: '任务未在运行', submitRun: '提交并执行', launch: '开始执行', launchPlaceholder: '开始执行', resumePlaceholder: '继续执行', failedPlaceholder: '任务异常中断，重新继续执行', complete: '完成任务', completePlaceholder: '完成', resume: '继续执行', reviewNeeded: '任务 [{name}] 等待您的审核', runningNoPurge: '运行中的任务不可删除', openInDialog: '在对话页打开'},
        agent: {viewTitle: '已注册智能体名册', newAgent: '+ 新建智能体', name: '智能体名称', prompt: '系统提示词', namePlaceholder: '例如：代码审查员', descPlaceholder: '例如：审查 PR 并报告问题', promptPlaceholder: '# 输入智能体系统提示词...', empty: '暂无已注册智能体', nameRequired: '智能体名称为必填项。', providerRequired: '必须选择模型供应商。', commitFailure: '保存失败', synced: '智能体 [{name}] 已保存。', purgeTitle: '删除智能体', purgeText: '确定要删除智能体 [{name}] 吗？其关联会话将被解绑。', viewDesc: '智能体绑定模型供应商、系统提示词与行为描述，可被会话、任务、定时调用。使用前请先在「配置」页添加模型供应商。', emptyHint: '提示：智能体依赖模型供应商。请先在「配置」页添加供应商，再点击「+ 新建智能体」。'},
        cron: {viewTitle: '定时任务管理', newCron: '+ 新建定时任务', name: '任务名称', namePlaceholder: '例如：每日代码审查', agent: '智能体', minute: '分', hour: '时', day: '日', month: '月', week: '星期', contentLabel: '任务内容', contentPlaceholder: '# 输入任务描述...', empty: '暂无定时任务', unnamed: '未命名任务', workingDir: '工作目录', nextFire: '下次触发', triggerSpec: '触发规则', execContent: '任务内容', suspended: '已暂停', enabled: '已启用', disabled: '已禁用', enable: '启用', disable: '禁用', synced: '定时任务 [{name}] 已保存。', purgeTitle: '删除定时任务', purgeText: '确定要删除定时任务 [{name}] 吗？删除后将不再执行。', formatHint: '字段顺序：分 时 日 月 星期。每个字段支持数字及 * , - / 符号（如 分=0 时=9 表示每天 09:00）', invalidFormat: '定时字段只能包含数字和 * , - / 符号。', execHistory: '执行记录', notTriggered: '尚未触发执行，暂无记录。', viewDesc: '定时任务按 cron 规则（分 时 日 月 星期）自动触发，无需人工值守，执行记录保留在任务卡片中。使用前请先在「智能体」页注册智能体。', emptyHint: '提示：定时任务按规则自动执行。请先在「智能体」页注册智能体，再点击「+ 新建定时任务」。'},
        skill: {viewTitle: '已注册智能体技能', empty: '未提取到技能', unnamed: '未命名技能', noDesc: '未分配描述。', fsPath: '文件路径', sourceManifest: '源清单', viewDesc: '自动扫描用户目录下的 ~/.openagents/skills 与 ~/.agents/skills 技能目录并加载，无需手动创建，本页仅作展示。', emptyHint: '未扫描到技能。请将技能目录放入用户目录下的 ~/.openagents/skills 或 ~/.agents/skills 后重新打开本页。'},
        mcp: {viewTitle: 'Model Context Protocol 服务', newServer: '+ 新建服务', keyName: '服务名称', keyPlaceholder: '例如：file_server', descPlaceholder: '例如：文件服务器', protocolArch: '协议类型', urlEndpoint: '连接地址', headersLabel: 'HTTP 请求头（JSON 格式）', command: '执行命令', args: '参数（逗号分隔）', empty: '暂无已注册 MCP 服务', localContext: '本地上下文', testProbe: '测试连接', protocolType: '协议类型', targetUrl: '目标地址', httpHeaders: 'HTTP 请求头（JSON）', cmdExec: '执行命令', exposedCaps: '暴露的能力清单', nameRequired: '服务名称为必填项。', headersInvalid: '请求头必须是合法的 JSON 字符串。', regRejected: '创建失败', synced: 'MCP 服务 [{name}] 已保存。', purgeTitle: '删除 MCP 服务', purgeText: '确定要删除 MCP 服务 [{name}] 吗？其提供的工具将不再可用。', connectedNoTools: '连接成功，未暴露任何工具。', noManifest: '未提供说明。', sessionCrashed: '连接被拒绝', viewDesc: '注册 MCP 工具服务（远程 HTTP 或本地命令），连接成功后其暴露的工具即可被智能体调用。', emptyHint: '提示：MCP 服务用于给智能体外接工具（文件、数据库、API 等）。点击「+ 新建服务」注册第一个服务。'},
        config: {viewTitle: '系统运行配置', newProvider: '+ 新建供应商', newProviderCard: '+ 新建模型供应商', providerName: '供应商名称', providerNamePlaceholder: '例如：deepseek', protocolType: '协议类型', baseUrl: '基础地址', secretToken: '密钥（API Key）', endpointMissing: '缺少端点地址', nameRequired: '供应商名称为必填项。', injectionFault: '创建失败', synced: '供应商 [{name}] 已保存。', purgeTitle: '删除模型供应商', purgeText: '确定要删除供应商 [{name}] 吗？使用该供应商的智能体将无法正常工作。', viewDesc: '模型供应商（协议 / 地址 / 密钥）是智能体运行的基础，添加供应商是使用本系统的第一步。', empty: '暂无模型供应商', emptyHint: '第一步：点击「+ 新建供应商」添加模型供应商，这是创建智能体、开始会话的前提。'},
        input: {cwdTitle: '设置目录上下文', unset: '未设置', agentTitle: '选择智能体', providerTitle: '选择供应商', modelTitle: '选择模型', thinkingTitle: '思考模式', placeholder: '输入指令...', readonlyPlaceholder: '该会话来自任务/定时执行，仅供查看，不可发送消息。', execute: '发送', stop: '停止', charLimit: '已达输入上限（{max} 字符），超出内容已被丢弃。', contextLocked: '会话已创建，工作目录与智能体不可修改。', cwdLabel: '工作目录：', modelListUnavailable: '无法获取模型列表，可直接输入模型名称。', promptSourceAgent: '提示词：智能体 [{name}]', promptSourceFile: '提示词：{path}', promptSourcePending: '提示词：启动时将读取 {path}（如存在）', promptSourceNone: '提示词：无'},
        modal: {workspaceContext: '工作目录上下文', pathPlaceholder: '输入路径...', go: '跳转', recentWorkdirs: '最近工作目录', confirmPath: '确认路径', removeHistory: '从历史中移除'},
        stream: {purgeTitle: '删除会话', purgeText: '确定要删除会话 [{name}] 吗？该操作不可恢复。', startFailed: '启动失败', configMissing: '请先选择供应商并填写模型。', workdirMissing: '请先设置工作目录（WORKDIR）。', unknownError: '未知错误', thoughtProcess: '思考过程', systemPrompt: '系统提示词', callPrefix: '调用', tool: '工具', toolError: '错误', toolResult: '结果', usageIn: '输入', usageOut: '输出', usageCache: '缓存', usageTotal: '本轮', stopped: '\u23f8 已手动暂停', stopFailed: '停止失败'},
        workspace: {upLevel: '..（上一级）', connFailed: '错误：连接失败'}
    }
};

// ===== 2. 语言状态与文案查询 =====
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

// ===== 3. 文案应用与刷新 =====
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

// 同步语言切换按钮：固定显示 中/EN 双语言标识，避免被误解为当前语言状态
function syncLanguageButton() {
    const btn = document.getElementById('langToggleBtn');
    if (btn) {
        btn.textContent = '中/EN';
    }
}

// 切换语言后重渲染当前视图的动态内容（会话页流式消息不重载，仅刷新 header 信息文本）
function refreshActiveViewI18n() {
    const activeView = document.querySelector('.view-container.active');
    if (!activeView) {
        return;
    }
    // 面板视图：按 VIEW_CONFIG 路由表（core.js）找到对应加载函数重新拉取渲染
    const cfg = Object.values(VIEW_CONFIG).find(item => item.view === activeView.id);
    if (cfg && cfg.load) {
        window[cfg.load]();
        return;
    }
    // 会话页：仅刷新 header 文案与提示词来源提示，不重载流式消息
    conversationInfo.textContent = currentConversationId ? `ID: ${currentConversationId}` : t('header.newTrace');
    updatePromptSourceHint();
    // 只读会话的占位文案会被 data-i18n-placeholder 批量刷新覆盖，需单独恢复
    if (currentConvReadonly) {
        messageInput.placeholder = t('input.readonlyPlaceholder');
    }
}

// ===== 4. 语言设置与初始化 =====
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
    // 刷新主题选择器文案（主题名随语言切换）
    syncThemePicker(document.documentElement.dataset.theme);
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
