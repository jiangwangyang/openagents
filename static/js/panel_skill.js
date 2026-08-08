// ==========================================
// SKILLS 核心原子能力展示面板
// ==========================================
const skillListContainer = document.getElementById('skillListContainer');

async function fetchSkillData() {
    skillListContainer.innerHTML = SKELETON_HTML;
    try {
        const response = await fetch('/skill/list');
        const skills = await response.json();
        skillListContainer.innerHTML = '';

        if (skills.length === 0) {
            skillListContainer.innerHTML = `<div style="padding:30px; text-align:center; border:1px dashed var(--border-hard); color:var(--slate-400)">${t('skill.empty')}</div>`;
            return;
        }

        skills.forEach(skill => {
            const card = document.createElement('div');
            card.className = 'info-card';
            card.innerHTML = `
                <div class="info-card-summary" onclick="toggleCardOpen(this.parentNode)">
                    <div class="info-card-main">
                        ${ARROW_SVG}
                        <span class="info-card-name">${escapeHtml(skill.name || t('skill.unnamed'))}</span>
                        <span class="info-card-snippet">${escapeHtml(skill.description || '')}</span>
                    </div>
                </div>
                <div class="info-card-details" style="display: none;">
                    <div class="details-grid">
                        <div class="details-label">${t('common.description')}</div>
                        <div class="details-value">${escapeHtml(skill.description || t('skill.noDesc'))}</div>
                        <div class="details-label">${t('skill.fsPath')}</div>
                        <div class="details-value" style="font-family:var(--font-mono); font-size:12px; color:var(--slate-400)">${escapeHtml(skill.path || '')}</div>
                        <div class="details-block-container">
                            <div class="details-label" style="margin-bottom: 6px;">${t('skill.sourceManifest')}</div>
                            <div class="reply-content">${formatMarkdown(skill.content || '')}</div>
                        </div>
                    </div>
                </div>
            `;
            skillListContainer.appendChild(card);
        });
    } catch (e) {
        skillListContainer.innerHTML = `<div style="padding:20px; color:var(--danger-color)">${t('common.fetchFailed')}</div>`;
    }
}

