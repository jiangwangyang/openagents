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

// 主题选择器元数据：菜单顺序即数组顺序，bg/accent 用于色板预览
const THEME_META = [
    {id: 'light', label: 'LIGHT', bg: '#f0efe9', accent: '#121417'},
    {id: 'ink', label: 'INK', bg: '#f2ecdf', accent: '#b03a2e'},
    {id: 'sunset', label: 'SUNSET', bg: '#ffd6bd', accent: '#e85d3d'},
    {id: 'botanica', label: 'BOTANICA', bg: '#f4f2e4', accent: '#2e6b34'},
    {id: 'draft', label: 'DRAFT', bg: '#eef4fd', accent: '#1a56c4'},
    {id: 'dark', label: 'DARK', bg: '#101214', accent: '#e6e8eb'},
    {id: 'aurora', label: 'AURORA', bg: '#060a18', accent: '#5ff0c0'},
    {id: 'vaporwave', label: 'VAPORWAVE', bg: '#1b1035', accent: '#ff71ce'},
    {id: 'cyberpunk', label: 'CYBERPUNK', bg: '#04040c', accent: '#ff2a6d'},
    {id: 'matrix', label: 'MATRIX', bg: '#000000', accent: '#00ff41'},
    {id: 'blackhole', label: 'BLACK HOLE', bg: '#000000', accent: '#ff9a3d'}
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
        label.textContent = meta.label;
    }
    const menu = document.getElementById('themePickerMenu');
    if (menu) {
        menu.querySelectorAll('.theme-picker-item').forEach(item => {
            item.classList.toggle('active', item.dataset.theme === theme);
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
    THEME_META.forEach(t => {
        // 明暗两组之间加分隔线（dark 为暗色组首项）
        if (t.id === 'dark') {
            const divider = document.createElement('div');
            divider.className = 'theme-picker-divider';
            menu.appendChild(divider);
        }
        const item = document.createElement('button');
        item.type = 'button';
        item.className = 'theme-picker-item';
        item.dataset.theme = t.id;
        const swatch = document.createElement('span');
        swatch.className = 'theme-picker-swatch';
        swatch.style.background = t.bg;
        swatch.style.borderColor = t.accent;
        const swatchDot = document.createElement('span');
        swatchDot.className = 'theme-picker-swatch-dot';
        swatchDot.style.background = t.accent;
        swatch.appendChild(swatchDot);
        const text = document.createElement('span');
        text.textContent = t.label;
        item.appendChild(swatch);
        item.appendChild(text);
        item.addEventListener('click', () => {
            setTheme(t.id);
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

