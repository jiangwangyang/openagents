// ==========================================
// 主题动态粒子特效引擎（背景装饰由 CSS 变量驱动，此引擎负责飘叶/光尘/星尘/霓虹雨等）
// ==========================================
// 主题特效配置表：key 为 data-theme 值，null 表示该主题无动态特效
const THEME_EFFECTS = {
    'light': null,
    'dark': null,
    'ink': {kind: 'ink', count: 14},
    'sunset': {kind: 'sunset', count: 22},
    'aurora': {kind: 'aurora', count: 90},
    'cyberpunk': {kind: 'neon-rain', count: 42},
    // 黑洞主题为独立 WebGL 渲染层（theme_blackhole.js），不走下方 2D 粒子循环
    'blackhole': {kind: 'blackhole', count: 0}
};

// 运行时状态（纯数据对象）
let fx = {running: false, raf: null, parts: [], kind: null, canvas: null, ctx: null, scenery: null, sctx: null, last: 0, t: 0, meteors: [], gridOff: 0};
// 系统要求减少动态效果时自动关闭粒子
const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
// 帧间隔上限：全部主题特效限制 30fps，降低 GPU/CPU 常驻开销（dt 按真实流逝时间计算，运动速度不受影响）
const FX_FRAME_MS = 1000 / 30;
// 初始化标记：防止 script 顶部 setTheme 与 DOMContentLoaded 重复绑定
let fxBound = false;

// 初始化特效层：绑定画布、响应窗口缩放、启动当前主题特效
function initThemeEffects() {
    const canvas = document.getElementById('themeEffectsCanvas');
    if (!canvas || fxBound) {
        return;
    }
    fxBound = true;
    fx.canvas = canvas;
    fx.ctx = canvas.getContext('2d');
    canvas.width = window.innerWidth;
    canvas.height = window.innerHeight;
    // 布景层画布（z-index 0，内容之下）：城市剪影等实体背景在此绘制，避免遮挡阅读区
    fx.scenery = document.getElementById('themeSceneryCanvas');
    fx.sctx = fx.scenery ? fx.scenery.getContext('2d') : null;
    if (fx.scenery) {
        fx.scenery.width = window.innerWidth;
        fx.scenery.height = window.innerHeight;
    }
    window.addEventListener('resize', () => {
        canvas.width = window.innerWidth;
        canvas.height = window.innerHeight;
        if (fx.scenery) {
            fx.scenery.width = window.innerWidth;
            fx.scenery.height = window.innerHeight;
        }
    });
    startThemeEffects(document.documentElement.dataset.theme);
}

// 启动/切换主题特效：先销毁旧循环并清空画面，再按配置生成粒子
function startThemeEffects(theme) {
    if (fx.raf) {
        cancelAnimationFrame(fx.raf);
    }
    fx.running = false;
    fx.parts = [];
    fx.t = 0;
    fx.meteors = [];
    fx.gridOff = 0;
    // 布景层实体背景只属特定主题，切换任何主题时先擦除残留
    if (fx.sctx && fx.scenery) {
        fx.sctx.clearRect(0, 0, fx.scenery.width, fx.scenery.height);
    }
    // 黑洞主题为独立 WebGL 渲染层：切换任何主题时先停止其渲染循环
    stopBlackhole();
    const cfg = THEME_EFFECTS[theme] || null;
    fx.kind = cfg ? cfg.kind : null;
    // 黑洞主题：清空粒子画布残留后交由 WebGL 渲染层（reducedMotion 由其内部降级为静态单帧）
    if (fx.kind === 'blackhole') {
        if (fx.ctx && fx.canvas) {
            fx.ctx.clearRect(0, 0, fx.canvas.width, fx.canvas.height);
        }
        startBlackhole();
        return;
    }
    // 无特效/系统要求减弱动态：先清空画布残留，再直接退出
    if (!cfg || reducedMotion) {
        if (fx.ctx && fx.canvas) {
            fx.ctx.clearRect(0, 0, fx.canvas.width, fx.canvas.height);
        }
        return;
    }
    if (!fx.canvas) {
        return;
    }
    const w = fx.canvas.width;
    const h = fx.canvas.height;
    for (let i = 0; i < cfg.count; i++) {
        fx.parts.push(makeParticle(fx.kind, w, h, i, theme));
    }
    fx.running = true;
    fx.last = performance.now();
    fx.raf = requestAnimationFrame(tick);
}

// 生成单个粒子对象（飘叶/光尘/星尘/霓虹雨等类型，按主题特效 kind 分发）
function makeParticle(kind, w, h, seed, theme) {
    const rand = (min, max) => min + Math.random() * (max - min);
    const p = {x: rand(0, w), y: rand(0, h), r: rand(1, 6), vx: 0, vy: 0, sway: 0, swaySpeed: rand(0.4, 1.6), phase: rand(0, Math.PI * 2), rot: rand(0, Math.PI * 2), rotSpeed: 0, color: '#ffffff', opacity: 0.4};
    if (kind === 'sunset') {
        // 黄金时刻：3/4 为上升的暖光尘，1/4 为掠过天空的海鸥剪影
        if (seed % 4 === 3) {
            p.shape = 'bird';
            p.r = rand(6, 13);
            p.dir = seed % 2 === 0 ? 1 : -1;
            p.vx = rand(18, 42) * p.dir;
            p.swaySpeed = rand(4, 7);
            p.y = rand(h * 0.08, h * 0.42);
            p.color = '#4e2333';
            p.opacity = rand(0.3, 0.55);
        } else {
            p.shape = 'mote';
            p.r = rand(1, 2.8);
            p.vx = rand(-6, 6);
            p.vy = rand(-9, -4);
            p.sway = rand(4, 12);
            p.swaySpeed = rand(0.3, 0.9);
            p.color = '#f7c873';
            p.opacity = rand(0.3, 0.65);
        }
    } else if (kind === 'neon-rain') {
        // 赛博朋克：高速坠落的霓虹雨 streak
        p.x = rand(0, w);
        p.y = rand(-h, h);
        p.len = rand(40, 160);
        p.vy = rand(320, 720);
        p.r = rand(0.8, 1.8);
        p.color = ['#00f0ff', '#ff2a6d', '#b967ff', '#01cdfe'][seed % 4];
        p.opacity = rand(0.25, 0.7);
    } else if (kind === 'aurora') {
        // 极光：闪烁星尘（极光带与流星在主循环中程序化绘制）
        p.r = rand(0.4, 1.6);
        p.twinkleSpeed = rand(0.5, 2.2);
        p.baseOpacity = rand(0.15, 0.75);
        p.opacity = p.baseOpacity;
        p.color = '#ffffff';
    } else if (kind === 'ink') {
        // 水墨：偶数位为晕染墨滴（生长-消退-重生），奇数位为飘落竹叶
        if (seed % 2 === 0) {
            p.shape = 'drop';
            p.r = 0;
            p.maxR = rand(24, 90);
            p.growSpeed = rand(10, 26);
            p.opacity = rand(0.1, 0.22);
            p.life = rand(4, 9);
            p.age = 0;
            // 墨滴只落在宣纸四缘（左右竖边/上下横边），避免出现在正文区形成污渍感
            const edge = Math.floor(Math.random() * 4);
            if (edge === 0) { p.x = rand(0, w * 0.14); }
            else if (edge === 1) { p.x = rand(w * 0.86, w); }
            else if (edge === 2) { p.y = rand(0, h * 0.18); }
            else { p.y = rand(h * 0.84, h); }
        } else {
            p.shape = 'leaf';
            p.r = rand(4, 7);
            p.vy = rand(14, 30);
            p.sway = rand(20, 40);
            p.swaySpeed = rand(0.5, 1.1);
            p.rotSpeed = rand(-0.8, 0.8);
            p.color = '#3a4a3a';
            p.opacity = rand(0.25, 0.5);
            p.y = rand(-40, h);
        }
    }
    return p;
}

// 粒子主循环：按类型更新位置并绘制，使用 requestAnimationFrame
function tick(now) {
    if (!fx.running) {
        return;
    }
    fx.raf = requestAnimationFrame(tick);
    // 30fps 限速：未到帧间隔直接跳帧，仅跳绘制不跳计时
    if (now - fx.last < FX_FRAME_MS) {
        return;
    }
    const dt = Math.min((now - fx.last) / 1000, 0.05);
    fx.last = now;
    const ctx = fx.ctx;
    const w = fx.canvas.width;
    const h = fx.canvas.height;
    ctx.clearRect(0, 0, w, h);
    // 程序化整屏背景层（非粒子）：极光带/流星、赛博透视网格每帧先行绘制
    if (fx.kind === 'aurora') {
        fx.t += dt;
        drawAuroraBands(ctx, w, h, fx.t);
        updateMeteors(ctx, w, h, dt);
    } else if (fx.kind === 'neon-rain') {
        fx.gridOff = (fx.gridOff + dt * 60) % 560;
        // 实体城市剪影绘制到内容之下的布景层；半透明光带/透视网格与霓虹雨粒子留在上层画布
        if (fx.sctx && fx.scenery) {
            fx.sctx.clearRect(0, 0, fx.scenery.width, fx.scenery.height);
        }
        drawCyberGrid(ctx, fx.sctx, w, h);
    }
    for (let i = 0; i < fx.parts.length; i++) {
        const p = fx.parts[i];
        p.phase += p.swaySpeed * dt;
        const swayX = Math.sin(p.phase) * p.sway;
        if (fx.kind === 'sunset') {
            if (p.shape === 'bird') {
                // 海鸥：水平滑翔掠过天空，出屏后从另一侧重新入场
                p.x += p.vx * dt;
                if (p.dir > 0 && p.x > w + 40) {
                    p.x = -40;
                    p.y = h * (0.08 + Math.random() * 0.34);
                } else if (p.dir < 0 && p.x < -40) {
                    p.x = w + 40;
                    p.y = h * (0.08 + Math.random() * 0.34);
                }
                drawBird(ctx, p);
            } else {
                // 暖光尘：缓升漂移，出界后循环
                p.x += (p.vx + swayX * 2) * dt;
                p.y += p.vy * dt;
                if (p.x < -10) {
                    p.x = w + 10;
                }
                if (p.x > w + 10) {
                    p.x = -10;
                }
                if (p.y < -10) {
                    p.y = h + 10;
                }
                drawDot(ctx, p);
            }
        } else if (fx.kind === 'neon-rain') {
            // 霓虹雨：垂直高速坠落，出屏后回到顶部重生
            p.y += p.vy * dt;
            if (p.y - p.len > h) {
                p.y = -p.len;
                p.x = Math.random() * w;
            }
            drawRainStreak(ctx, p);
        } else if (fx.kind === 'aurora') {
            // 极光盘星尘：静止闪烁
            p.opacity = p.baseOpacity * (0.5 + 0.5 * Math.sin(p.phase * p.twinkleSpeed));
            p.phase += dt;
            drawDot(ctx, p);
        } else if (fx.kind === 'ink') {
            if (p.shape === 'drop') {
                // 墨滴：生长扩散随年龄消退，寿尽后在新位置重生
                p.age += dt;
                p.r = Math.min(p.r + p.growSpeed * dt, p.maxR);
                if (p.age >= p.life) {
                    p.age = 0;
                    p.r = 0;
                    // 重生位置同样约束在屏幕四缘
                    p.x = Math.random() * w;
                    p.y = Math.random() * h;
                    const edge = Math.floor(Math.random() * 4);
                    if (edge === 0) { p.x = Math.random() * w * 0.14; }
                    else if (edge === 1) { p.x = w * 0.86 + Math.random() * w * 0.14; }
                    else if (edge === 2) { p.y = Math.random() * h * 0.18; }
                    else { p.y = h * 0.84 + Math.random() * h * 0.16; }
                    p.maxR = 24 + Math.random() * 66;
                }
                drawInkDrop(ctx, p);
            } else {
                // 竹叶：飘落摆动旋转，出底后回到顶部
                p.x += Math.sin(p.phase) * p.sway * dt;
                p.y += p.vy * dt;
                p.rot += p.rotSpeed * dt;
                if (p.y > h + 30) {
                    p.y = -30;
                    p.x = Math.random() * w;
                }
                drawLeaf(ctx, p);
            }
        }
    }
}

// 绘制光点（光尘/星光通用）
function drawDot(ctx, p) {
    ctx.save();
    ctx.globalAlpha = p.opacity;
    ctx.fillStyle = p.color;
    ctx.beginPath();
    ctx.arc(p.x, p.y, p.r, 0, Math.PI * 2);
    ctx.fill();
    ctx.restore();
}

// 绘制海鸥：v 形双翼剪影，翼尖随相位上下扇动，按飞行方向镜像
function drawBird(ctx, p) {
    const wing = Math.sin(p.phase) * p.r * 0.55;
    ctx.save();
    ctx.translate(p.x, p.y);
    if (p.dir < 0) {
        ctx.scale(-1, 1);
    }
    ctx.globalAlpha = p.opacity;
    ctx.strokeStyle = p.color;
    ctx.lineWidth = 1.8;
    ctx.lineCap = 'round';
    ctx.beginPath();
    ctx.moveTo(-p.r, wing);
    ctx.quadraticCurveTo(-p.r * 0.45, -p.r * 0.3, 0, 0);
    ctx.quadraticCurveTo(p.r * 0.45, -p.r * 0.3, p.r, wing);
    ctx.stroke();
    ctx.restore();
}

// 绘制赛博背景层：城市剪影（实体色块，画到内容之下的布景层 sctx）+ 霓虹光带与透视网格（半透明，画到上层 ctx，垂直线汇聚于消失点，水平线加速向观察者流动）
function drawCyberGrid(ctx, sctx, w, h) {
    const horizon = h * 0.6;
    const cx = w / 2;
    // 城市剪影：建筑轮廓与亮窗位置由索引哈希生成，逐帧稳定不闪烁
    if (sctx) {
        sctx.save();
        const bw = 46;
        const count = Math.ceil(w / bw) + 1;
        for (let i = 0; i < count; i++) {
            const seed = Math.abs(Math.sin(i * 12.9898) * 43758.5453) % 1;
            const bh = h * (0.05 + seed * 0.17);
            const bx = i * bw;
            sctx.fillStyle = 'rgba(5, 7, 18, 0.92)';
            sctx.fillRect(bx, horizon - bh, bw - 4, bh);
            sctx.strokeStyle = 'rgba(0, 240, 255, 0.1)';
            sctx.strokeRect(bx + 0.5, horizon - bh + 0.5, bw - 5, bh - 1);
            for (let r = 0; r < Math.floor(bh / 18); r++) {
                for (let c = 0; c < 3; c++) {
                    const wseed = Math.abs(Math.sin((i * 31 + r * 7 + c * 13) * 5.4321) * 12543.123) % 1;
                    // 仅约 1/6 窗格点亮，青/品红/暖黄三色霓虹
                    if (wseed < 0.16) {
                        sctx.fillStyle = ['rgba(0, 240, 255, 0.5)', 'rgba(255, 42, 109, 0.45)', 'rgba(255, 220, 120, 0.4)'][Math.floor(wseed * 97) % 3];
                        sctx.fillRect(bx + 7 + c * 12, horizon - bh + 6 + r * 18, 5, 8);
                    }
                }
            }
        }
        sctx.restore();
    }
    ctx.save();
    // 地平线霓虹光带：中央青色向两侧品红渐隐
    const band = ctx.createLinearGradient(0, 0, w, 0);
    band.addColorStop(0, 'rgba(255, 42, 109, 0)');
    band.addColorStop(0.5, 'rgba(0, 240, 255, 0.35)');
    band.addColorStop(1, 'rgba(255, 42, 109, 0)');
    ctx.fillStyle = band;
    ctx.fillRect(0, horizon - 2, w, 4);
    ctx.lineWidth = 1;
    const spacing = w / 24;
    for (let i = -12; i <= 12; i++) {
        ctx.strokeStyle = 'rgba(255, 42, 109, 0.16)';
        ctx.beginPath();
        ctx.moveTo(cx + i * spacing * 0.12, horizon);
        ctx.lineTo(cx + i * spacing * 2.4, h);
        ctx.stroke();
    }
    for (let j = 0; j < 14; j++) {
        const t = ((j * 40 + fx.gridOff) % 560) / 560;
        const y = horizon + t * t * (h - horizon);
        ctx.strokeStyle = `rgba(0, 240, 255, ${0.05 + t * 0.18})`;
        ctx.beginPath();
        ctx.moveTo(0, y);
        ctx.lineTo(w, y);
        ctx.stroke();
    }
    ctx.restore();
}

// 绘制霓虹雨 streak：带辉光的发光垂直线段
function drawRainStreak(ctx, p) {
    ctx.save();
    ctx.globalAlpha = p.opacity;
    ctx.strokeStyle = p.color;
    ctx.lineWidth = p.r;
    ctx.shadowColor = p.color;
    ctx.shadowBlur = 8;
    ctx.beginPath();
    ctx.moveTo(p.x, p.y - p.len);
    ctx.lineTo(p.x, p.y);
    ctx.stroke();
    ctx.restore();
}

// 绘制极光带：三条正弦扭曲的渐变光带，lighter 混合叠加发光
function drawAuroraBands(ctx, w, h, t) {
    const bands = [
        {y: 0.16, amp: 60, speed: 0.25, thick: 90, color: [67, 232, 160], alpha: 0.1},
        {y: 0.3, amp: 80, speed: 0.18, thick: 120, color: [123, 108, 240], alpha: 0.08},
        {y: 0.1, amp: 45, speed: 0.32, thick: 60, color: [255, 106, 213], alpha: 0.05}
    ];
    ctx.save();
    ctx.globalCompositeOperation = 'lighter';
    bands.forEach((b, bi) => {
        const baseY = h * b.y;
        ctx.beginPath();
        for (let x = 0; x <= w; x += 24) {
            const y = baseY + Math.sin(x * 0.004 + t * b.speed + bi * 2.1) * b.amp + Math.sin(x * 0.011 - t * b.speed * 0.6 + bi) * b.amp * 0.4;
            if (x === 0) {
                ctx.moveTo(x, y);
            } else {
                ctx.lineTo(x, y);
            }
        }
        for (let x = w; x >= 0; x -= 24) {
            const y = baseY + Math.sin(x * 0.004 + t * b.speed + bi * 2.1) * b.amp + Math.sin(x * 0.011 - t * b.speed * 0.6 + bi) * b.amp * 0.4 + b.thick * (1 + 0.3 * Math.sin(x * 0.006 + t * 0.4));
            ctx.lineTo(x, y);
        }
        ctx.closePath();
        const g = ctx.createLinearGradient(0, baseY - b.amp, 0, baseY + b.thick + b.amp);
        g.addColorStop(0, `rgba(${b.color[0]}, ${b.color[1]}, ${b.color[2]}, 0)`);
        g.addColorStop(0.45, `rgba(${b.color[0]}, ${b.color[1]}, ${b.color[2]}, ${b.alpha})`);
        g.addColorStop(1, `rgba(${b.color[0]}, ${b.color[1]}, ${b.color[2]}, 0)`);
        ctx.fillStyle = g;
        ctx.fill();
    });
    ctx.restore();
}

// 更新并绘制流星：随机生成，拖尾渐隐，寿尽移除
function updateMeteors(ctx, w, h, dt) {
    if (fx.meteors.length < 2 && Math.random() < 0.004) {
        fx.meteors.push({x: Math.random() * w * 0.8 + w * 0.1, y: Math.random() * h * 0.25, vx: -(300 + Math.random() * 300), vy: 160 + Math.random() * 120, life: 1});
    }
    ctx.save();
    for (let i = fx.meteors.length - 1; i >= 0; i--) {
        const m = fx.meteors[i];
        m.x += m.vx * dt;
        m.y += m.vy * dt;
        m.life -= dt * 0.9;
        if (m.life <= 0 || m.x < -100 || m.y > h + 100) {
            fx.meteors.splice(i, 1);
            continue;
        }
        const g = ctx.createLinearGradient(m.x, m.y, m.x - m.vx * 0.25, m.y - m.vy * 0.25);
        g.addColorStop(0, `rgba(255, 255, 255, ${0.8 * m.life})`);
        g.addColorStop(1, 'rgba(255, 255, 255, 0)');
        ctx.strokeStyle = g;
        ctx.lineWidth = 1.6;
        ctx.beginPath();
        ctx.moveTo(m.x, m.y);
        ctx.lineTo(m.x - m.vx * 0.25, m.y - m.vy * 0.25);
        ctx.stroke();
    }
    ctx.restore();
}

// 绘制墨滴：径向渐变晕染，带偏移飞白晕圈
function drawInkDrop(ctx, p) {
    ctx.save();
    const fade = Math.max(1 - p.age / p.life, 0);
    const g = ctx.createRadialGradient(p.x, p.y, 0, p.x, p.y, Math.max(p.r, 1));
    g.addColorStop(0, `rgba(28, 26, 23, ${(p.opacity * fade).toFixed(3)})`);
    g.addColorStop(0.7, `rgba(28, 26, 23, ${(p.opacity * fade * 0.5).toFixed(3)})`);
    g.addColorStop(1, 'rgba(28, 26, 23, 0)');
    ctx.fillStyle = g;
    ctx.beginPath();
    ctx.arc(p.x, p.y, Math.max(p.r, 1), 0, Math.PI * 2);
    ctx.fill();
    ctx.beginPath();
    ctx.arc(p.x + p.r * 0.3, p.y - p.r * 0.2, Math.max(p.r * 0.55, 1), 0, Math.PI * 2);
    ctx.fill();
    ctx.restore();
}

// 绘制竹叶：尖细椭圆叶形，随风摆动
function drawLeaf(ctx, p) {
    ctx.save();
    ctx.translate(p.x, p.y);
    ctx.rotate(p.rot + Math.sin(p.phase) * 0.4);
    ctx.globalAlpha = p.opacity;
    ctx.fillStyle = p.color;
    ctx.beginPath();
    ctx.moveTo(0, -p.r * 1.8);
    ctx.quadraticCurveTo(p.r * 0.5, 0, 0, p.r * 1.8);
    ctx.quadraticCurveTo(-p.r * 0.5, 0, 0, -p.r * 1.8);
    ctx.fill();
    ctx.restore();
}

