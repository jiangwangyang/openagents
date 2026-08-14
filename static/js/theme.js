// ==========================================
// 主题切换引擎（浅色/深色，未手动选择时默认浅色）
// ==========================================
const THEME_STORAGE_KEY = 'openagents_theme';

function setTheme(theme) {
    // 校验主题合法性（已删除主题/非法值回退浅色），THEME_EFFECTS 的 key 即有效主题白名单
    if (!Object.prototype.hasOwnProperty.call(THEME_EFFECTS, theme)) {
        theme = 'light';
    }
    // 持久化到后端 Web 存储（异步落库，不阻塞主题切换）
    setWebStorage(THEME_STORAGE_KEY, theme);
    document.documentElement.dataset.theme = theme;
    syncThemePicker(theme);
    // 同步切换动态粒子特效（静态背景由 CSS 变量随 data-theme 自动生效）
    startThemeEffects(theme);
}

// 主题选择器元数据：菜单顺序即数组顺序，分 3 组（基础 light/dark → 其它主题 → 黑洞），divider 标记该项前插入分组分割线，bg/accent 用于色板预览，名称取自 i18n theme 组
const THEME_META = [
    {id: 'light', bg: '#f0efe9', accent: '#121417'},
    {id: 'dark', bg: '#101214', accent: '#e6e8eb'},
    {id: 'ink', divider: true, bg: '#f2ecdf', accent: '#b03a2e'},
    {id: 'sunset', bg: '#ffd6bd', accent: '#e85d3d'},
    {id: 'botanica', bg: '#f4f2e4', accent: '#2e6b34'},
    {id: 'draft', bg: '#eef4fd', accent: '#1a56c4'},
    {id: 'aurora', bg: '#060a18', accent: '#5ff0c0'},
    {id: 'vaporwave', bg: '#1b1035', accent: '#ff71ce'},
    {id: 'cyberpunk', bg: '#04040c', accent: '#ff2a6d'},
    {id: 'matrix', bg: '#000000', accent: '#00ff41'},
    {id: 'blackhole', divider: true, bg: '#000000', accent: '#ff9a3d'}
];

// 同步主题选择器显示：按钮色板与标签 + 菜单项高亮
function syncThemePicker(theme) {
    const meta = THEME_META.find(t => t.id === theme) || THEME_META[0];
    const dot = document.getElementById('themePickerDot');
    const label = document.getElementById('themePickerLabel');
    if (dot) {
        dot.style.background = meta.bg;
        dot.style.borderColor = meta.accent;
    }
    if (label) {
        label.textContent = t('theme.' + meta.id);
    }
    const menu = document.getElementById('themePickerMenu');
    if (menu) {
        menu.querySelectorAll('.theme-picker-item').forEach(item => {
            item.classList.toggle('active', item.dataset.theme === theme);
            // 随语言切换刷新菜单项文案（i18n.js 的 setLanguage 会调用本函数）
            item.querySelector('.theme-picker-text').textContent = t('theme.' + item.dataset.theme);
        });
    }
}

// 初始化主题选择器：构建色板菜单、绑定开合、点击外部或按 Escape 关闭
function initThemePicker() {
    const menu = document.getElementById('themePickerMenu');
    const btn = document.getElementById('themePickerBtn');
    if (!menu || !btn || menu.childElementCount > 0) {
        return;
    }
    THEME_META.forEach(meta => {
        // 三组主题之间插入分割线（divider 标记的项为一组之首）
        if (meta.divider) {
            const divider = document.createElement('div');
            divider.className = 'theme-picker-divider';
            menu.appendChild(divider);
        }
        const item = document.createElement('button');
        item.type = 'button';
        item.className = 'theme-picker-item';
        item.dataset.theme = meta.id;
        const swatch = document.createElement('span');
        swatch.className = 'theme-picker-swatch';
        swatch.style.background = meta.bg;
        swatch.style.borderColor = meta.accent;
        const swatchDot = document.createElement('span');
        swatchDot.className = 'theme-picker-swatch-dot';
        swatchDot.style.background = meta.accent;
        swatch.appendChild(swatchDot);
        const text = document.createElement('span');
        text.className = 'theme-picker-text';
        text.textContent = t('theme.' + meta.id);
        item.appendChild(swatch);
        item.appendChild(text);
        item.addEventListener('click', () => {
            setTheme(meta.id);
            menu.classList.remove('open');
        });
        menu.appendChild(item);
    });
    btn.addEventListener('click', (e) => {
        e.stopPropagation();
        menu.classList.toggle('open');
    });
    document.addEventListener('click', () => menu.classList.remove('open'));
    document.addEventListener('keydown', (e) => {
        if (e.key === 'Escape') {
            menu.classList.remove('open');
        }
    });
    syncThemePicker(document.documentElement.dataset.theme);
}

// 初始化：先应用内联脚本预设的主题（不触发持久化），再异步加载后端持久化主题并应用
(function initTheme() {
    const preset = document.documentElement.dataset.theme || 'light';
    document.documentElement.dataset.theme = preset;
    syncThemePicker(preset);
    // 同步开启动态粒子特效（静态背景由 CSS 变量随 data-theme 自动生效）
    startThemeEffects(preset);
    getWebStorage(THEME_STORAGE_KEY).then(savedTheme => {
        if (savedTheme && savedTheme !== preset) {
            setTheme(savedTheme);
        }
    });
})();
initThemePicker();

