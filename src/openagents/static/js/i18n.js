// ==========================================
// I18N 国际化引擎（中英文切换，与主题方案同构）
// ==========================================
const I18N_STORAGE_KEY = 'openagents_lang';

// 文案字典：en/zh 两组，key 按模块分组，支持 {name} 占位符替换
const I18N = {
    en: {
        lang: {toggleTitle: 'Switch Language'},
        sidebar: {newDialog: '+ New Dialog', history: 'History Logs'},
        nav: {dialog: 'DIALOG', task: 'TASK', agent: 'AGENT', cron: 'CRON', skill: 'SKILL', mcp: 'MCP', config: 'CONFIG'},
        header: {newTrace: 'NEW TRACE', switchTheme: 'Switch Theme', coreTask: 'CORE::TASK_PIPELINE', coreCron: 'CORE::SCHEDULE', coreSkill: 'CORE::SKILLS', coreMcp: 'CORE::MCP_ECOSYSTEM', coreAgent: 'CORE::AGENT_ROSTER', coreConfig: 'CORE::CONFIG_CONSOLE'},
        empty: {title: 'Initialize Process', text: 'Start a new conversation or select an existing trace from the sidebar to continue.', hint: 'Press Enter to execute / Shift+Enter for newline'},
        common: {save: 'Save', cancel: 'Cancel', abort: 'Abort', purge: 'Purge', refresh: 'Refresh', start: 'Start', review: 'Review', fetchFailed: 'FETCH FAILED', requiredMissing: 'REQUIRED FIELDS MISSING.', creationFault: 'CREATION FAULT', purgeFailure: 'PURGE FAILURE', syncCrashed: 'SYNC CRASHED', none: 'NONE', inheritedEnv: 'Inherited Environment', description: 'Description'},
        task: {viewTitle: 'Task Pipeline Management', newTask: '+ New Task', title: 'Task Title', content: 'Task Content', workDir: 'Working Directory', candidateAgents: 'Candidate Agents', titlePlaceholder: 'e.g. Daily Code Review Pipeline', contentPlaceholder: '# Enter task description...', launch: 'Launch Pipeline', stageProgress: 'Stage Progress', empty: 'NO TASK PIPELINES FOUND', noAgents: 'NO REGISTERED AGENTS', notStarted: 'PIPELINE NOT STARTED. ZERO STAGES.', generating: '⚡ GENERATING... CLICK TO VIEW LIVE STREAM', reviewPlaceholder: 'Enter review feedback...', reviewRequired: 'REVIEW FEEDBACK IS REQUIRED.', reviewFault: 'REVIEW FAULT', noMessages: 'NO MESSAGES', agentRequired: 'AGENT IS REQUIRED.', launched: 'TASK PIPELINE LAUNCHED.', alreadyRunning: 'TASK ALREADY RUNNING', startFault: 'START FAULT', purgeTitle: 'PURGE TASK PIPELINE', purgeText: 'Are you sure you want to destroy task pipeline [{name}]? All stage conversations attached will be wiped out instantly.'},
        agent: {viewTitle: 'Registered Agent Roster', newAgent: '+ New Agent', name: 'Agent Name', prompt: 'System Prompt', namePlaceholder: 'e.g. Code Reviewer', descPlaceholder: 'e.g. Reviews pull requests and reports issues', promptPlaceholder: '# Enter agent system prompt...', empty: 'NO REGISTERED AGENTS FOUND', rosterCrashed: 'ROSTER CAPTURE CRASHED', nameRequired: 'AGENT NAME IS REQUIRED.', commitFailure: 'COMMIT FAILURE', synced: 'AGENT [{name}] SYNCHRONIZED.', purgeTitle: 'PURGE AGENT NODE', purgeText: 'Are you sure you want to completely eject agent [{name}] from system roster? Conversations attached will be detached instantly.'},
        cron: {viewTitle: 'Scheduled Cron Processes', newCron: '+ New Cron', name: 'Task Name', namePlaceholder: 'e.g. Daily Market Scraper', workDirPlaceholder: 'e.g. /home/project/miniclaw', minute: 'Min', hour: 'Hour', day: 'Day', month: 'Month', week: 'Week', contentLabel: 'Execution Content / Script Context', contentPlaceholder: '# Enter detailed script steps...', empty: 'NO SCHEDULED CRON PROCESSES FOUND', unnamed: 'Unnamed Task', workingDir: 'Working Dir', nextFire: 'Next Fire', triggerSpec: 'Trigger Spec', execContent: 'Execution Content', suspended: 'Suspended', purgeTitle: 'PURGE CRON PIPELINE', purgeText: 'Are you sure you want to destroy scheduled process [{name}]? This pipeline sequence will be wiped out from kernel queue.', purgeFailed: 'PURGE FAILED'},
        skill: {viewTitle: 'Registered Agent Skills', empty: 'NO SKILLS EXTRACTED', unnamed: 'Unnamed Skill', noDesc: 'No description assigned.', fsPath: 'FS Path', sourceManifest: 'Source Manifest'},
        mcp: {viewTitle: 'Model Context Protocol Servers', newServer: '+ New Server', keyName: 'Server Unique Name', keyPlaceholder: 'e.g. vserver_wind_financial_data', descPlaceholder: 'e.g. Wind Financial Data Provider', protocolArch: 'Protocol Architecture Type', urlEndpoint: 'Connection URL Endpoint', headersLabel: 'HTTP Headers Context (JSON Format)', command: 'Execute Command', args: 'Arguments (Comma Separated)', empty: 'NO REGISTERED MCP CONTEXTS', localContext: 'Local Context', testProbe: 'Test Probe', protocolType: 'Protocol Type', targetUrl: 'Target URL', httpHeaders: 'HTTP Context Headers (JSON)', cmdExec: 'Command Execution', exposedCaps: 'Exposed Capabilities Registry', topologyCrashed: 'TOPOLOGY CAPTURE CRASHED', nameRequired: 'SERVER UNIQUE NAME IS REQUIRED.', headersInvalid: 'HEADERS MUST BE A VALID JSON STRING.', regRejected: 'REGISTRATION REJECTED', synced: 'MCP SERVER [{name}] SYNCHRONIZED.', purgeTitle: 'PURGE MCP NODE', purgeText: 'Are you sure you want to completely eject MCP node [{name}] from system environment? Bridge tunnels attached will be disconnected instantly.', connectedNoTools: 'CONNECTED SUCCESSFULLY. ZERO TOOLS EXPOSED.', noManifest: 'No instruction manifest provided.', sessionCrashed: 'SESSION CRASHED: REFUSED CONNECTION'},
        config: {viewTitle: 'System Runtime Configurations', newProvider: '+ New Provider', activeRoute: 'Active Context Route', targetProvider: 'Target Provider (Name)', activeModel: 'Active Engine Model', thinkingMode: 'Thinking Mode', newProviderCard: '+ New Model Provider', providerName: 'Provider Unique Name', providerNamePlaceholder: 'e.g. deepseek', baseUrl: 'Base URL Endpoint', secretToken: 'Secret Token (API Key)', engineClusters: 'Engine Clusters (Comma Separated Models)', settingsFault: 'SETTINGS DESERIALIZATION FAULT', endpointMissing: 'Endpoint missing', nameRequired: 'PROVIDER NAME IS REQUIRED.', injectionFault: 'INJECTION FAULT', synced: 'PROVIDER CLUSTER [{name}] SYNCHRONIZED.', purgeTitle: 'PURGE MODEL PROVIDER', purgeText: 'Are you sure you want to completely remove infrastructure cluster [{name}]? Layer dependencies pointing here will collapse.', routingIncomplete: 'ROUTING PATH IS INCOMPLETE.', providerRoutingFailed: 'PROVIDER ROUTING FAILED', modelRoutingFailed: 'MODEL ROUTING FAILED', routeMounted: 'GLOBAL CONTEXT ACTIVE ROUTE MOUNTED.', routingCommitFault: 'ROUTING COMMIT FAULT'},
        input: {cwdTitle: 'Set Directory Context', unset: 'UNSET', agentTitle: 'Select Agent', providerTitle: 'Select Provider', modelTitle: 'Select Model', thinkingTitle: 'Thinking Mode', placeholder: 'Enter instructions...', execute: 'Execute'},
        modal: {workspaceContext: 'Workspace Context', pathPlaceholder: 'Input path...', go: 'GO', recentWorkdirs: 'Recent Workdirs', confirmPath: 'Confirm Path', removeHistory: 'Remove from history'},
        stream: {purgeTitle: 'PURGE RECORD', purgeText: 'Are you sure you want to delete conversation [{name}]? This trace log action is irreversible.', startFailed: 'START WORK FAILED', unknownError: 'Unknown Error', thoughtProcess: 'Thought Process', callPrefix: 'Call', tool: 'Tool', toolError: 'Error', toolResult: 'Result', usageIn: 'in', usageOut: 'out', usageCache: 'cache'},
        workspace: {upLevel: '.. (UP LEVEL)', connFailed: 'ERR: CONNECTION FAILED'}
    },
    zh: {
        lang: {toggleTitle: '切换语言'},
        sidebar: {newDialog: '+ 新建会话', history: '历史会话'},
        nav: {dialog: '会话', task: '任务', agent: '智能体', cron: '定时', skill: '技能', mcp: 'MCP', config: '配置'},
        header: {newTrace: '新会话', switchTheme: '切换主题', coreTask: '核心：任务流水线', coreCron: '核心：定时排程', coreSkill: '核心：技能清单', coreMcp: '核心：MCP 生态', coreAgent: '核心：智能体名册', coreConfig: '核心：配置控制台'},
        empty: {title: '初始化会话', text: '开始一个新会话，或从侧边栏选择已有会话继续。', hint: '按 Enter 发送 / Shift+Enter 换行'},
        common: {save: '保存', cancel: '取消', abort: '中止', purge: '删除', refresh: '刷新', start: '启动', review: '审核', fetchFailed: '加载失败', requiredMissing: '必填字段缺失。', creationFault: '创建失败', purgeFailure: '删除失败', syncCrashed: '同步失败', none: '无', inheritedEnv: '继承环境', description: '描述'},
        task: {viewTitle: '任务流水线管理', newTask: '+ 新建任务', title: '任务标题', content: '任务内容', workDir: '工作目录', candidateAgents: '候选智能体', titlePlaceholder: '例如：每日代码审查流水线', contentPlaceholder: '# 输入任务描述...', launch: '启动流水线', stageProgress: '阶段进展', empty: '暂无任务流水线', noAgents: '未注册智能体', notStarted: '流水线未启动，暂无阶段。', generating: '⚡ 生成中...点击查看实时流', reviewPlaceholder: '输入审核意见...', reviewRequired: '请填写审核意见。', reviewFault: '审核提交失败', noMessages: '暂无消息', agentRequired: '请选择智能体。', launched: '任务流水线已启动。', alreadyRunning: '任务已在运行中', startFault: '启动失败', purgeTitle: '删除任务流水线', purgeText: '确定要销毁任务流水线 [{name}] 吗？其关联的所有阶段会话将被立即清空。'},
        agent: {viewTitle: '已注册智能体名册', newAgent: '+ 新建智能体', name: '智能体名称', prompt: '系统提示词', namePlaceholder: '例如：代码审查员', descPlaceholder: '例如：审查 PR 并报告问题', promptPlaceholder: '# 输入智能体系统提示词...', empty: '暂无已注册智能体', rosterCrashed: '名册加载失败', nameRequired: '智能体名称为必填项。', commitFailure: '提交失败', synced: '智能体 [{name}] 已同步。', purgeTitle: '删除智能体', purgeText: '确定要将智能体 [{name}] 从系统名册中移除吗？其关联会话将被立即解绑。'},
        cron: {viewTitle: '定时任务管理', newCron: '+ 新建定时任务', name: '任务名称', namePlaceholder: '例如：每日行情抓取', workDirPlaceholder: '例如：/home/project/miniclaw', minute: '分', hour: '时', day: '日', month: '月', week: '周', contentLabel: '执行内容 / 脚本上下文', contentPlaceholder: '# 输入详细脚本步骤...', empty: '暂无定时任务', unnamed: '未命名任务', workingDir: '工作目录', nextFire: '下次触发', triggerSpec: '触发规则', execContent: '执行内容', suspended: '已暂停', purgeTitle: '删除定时任务', purgeText: '确定要销毁定时任务 [{name}] 吗？该任务序列将从内核队列中清除。', purgeFailed: '删除失败'},
        skill: {viewTitle: '已注册智能体技能', empty: '未提取到技能', unnamed: '未命名技能', noDesc: '未分配描述。', fsPath: '文件路径', sourceManifest: '源清单'},
        mcp: {viewTitle: 'Model Context Protocol 服务', newServer: '+ 新建服务', keyName: '服务唯一名称', keyPlaceholder: '例如：vserver_wind_financial_data', descPlaceholder: '例如：Wind 金融数据服务', protocolArch: '协议架构类型', urlEndpoint: '连接地址', headersLabel: 'HTTP 请求头（JSON 格式）', command: '执行命令', args: '参数（逗号分隔）', empty: '暂无已注册 MCP 服务', localContext: '本地上下文', testProbe: '测试探测', protocolType: '协议类型', targetUrl: '目标地址', httpHeaders: 'HTTP 请求头（JSON）', cmdExec: '执行命令', exposedCaps: '暴露的能力清单', topologyCrashed: '拓扑加载失败', nameRequired: '服务唯一名称为必填项。', headersInvalid: '请求头必须是合法的 JSON 字符串。', regRejected: '注册被拒绝', synced: 'MCP 服务 [{name}] 已同步。', purgeTitle: '删除 MCP 节点', purgeText: '确定要将 MCP 节点 [{name}] 从系统环境中移除吗？关联的桥接通道将被立即断开。', connectedNoTools: '连接成功，未暴露任何工具。', noManifest: '未提供说明。', sessionCrashed: '会话崩溃：连接被拒绝'},
        config: {viewTitle: '系统运行配置', newProvider: '+ 新建供应商', activeRoute: '当前路由', targetProvider: '目标供应商（名称）', activeModel: '当前引擎模型', thinkingMode: '思考模式', newProviderCard: '+ 新建模型供应商', providerName: '供应商唯一名称', providerNamePlaceholder: '例如：deepseek', baseUrl: '基础地址', secretToken: '密钥（API Key）', engineClusters: '模型集群（逗号分隔）', settingsFault: '配置解析失败', endpointMissing: '缺少端点地址', nameRequired: '供应商名称为必填项。', injectionFault: '注入失败', synced: '供应商集群 [{name}] 已同步。', purgeTitle: '删除模型供应商', purgeText: '确定要移除基础设施集群 [{name}] 吗？指向它的依赖层将失效。', routingIncomplete: '路由路径不完整。', providerRoutingFailed: '供应商路由失败', modelRoutingFailed: '模型路由失败', routeMounted: '全局当前路由已挂载。', routingCommitFault: '路由提交失败'},
        input: {cwdTitle: '设置目录上下文', unset: '未设置', agentTitle: '选择智能体', providerTitle: '选择供应商', modelTitle: '选择模型', thinkingTitle: '思考模式', placeholder: '输入指令...', execute: '执行'},
        modal: {workspaceContext: '工作目录上下文', pathPlaceholder: '输入路径...', go: '跳转', recentWorkdirs: '最近工作目录', confirmPath: '确认路径', removeHistory: '从历史中移除'},
        stream: {purgeTitle: '删除会话', purgeText: '确定要删除会话 [{name}] 吗？该记录操作不可恢复。', startFailed: '任务启动失败', unknownError: '未知错误', thoughtProcess: '思考过程', callPrefix: '调用', tool: '工具', toolError: '错误', toolResult: '结果', usageIn: '输入', usageOut: '输出', usageCache: '缓存'},
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

// 设置语言：持久化到 localStorage 并全量刷新页面文案
function setLanguage(lang) {
    if (lang !== 'zh' && lang !== 'en') {
        lang = 'en';
    }
    currentLang = lang;
    localStorage.setItem(I18N_STORAGE_KEY, lang);
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

// 初始化：优先本地存储，无记录时按浏览器语言判断（zh* 判中文，否则英文），检测值不落盘以保持跟随浏览器
(function initLanguage() {
    let lang = localStorage.getItem(I18N_STORAGE_KEY);
    if (!lang) {
        lang = (navigator.language || 'en').toLowerCase().startsWith('zh') ? 'zh' : 'en';
    }
    currentLang = lang === 'zh' ? 'zh' : 'en';
    document.documentElement.lang = currentLang === 'zh' ? 'zh-CN' : 'en';
    applyStaticI18n();
    syncLanguageButton();
})();
