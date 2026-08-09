// ==========================================
// Web 存储引擎（后端 KV 存储，替代 localStorage）
// ==========================================

// 读取存储值：不存在或请求失败返回 null
async function getWebStorage(key) {
    try {
        const response = await fetch(`/web-storage/${encodeURIComponent(key)}`);
        const data = await response.json();
        return data.value ?? null;
    } catch (e) {
        return null;
    }
}

// 写入存储值：失败静默
async function setWebStorage(key, value) {
    try {
        await fetch(`/web-storage/${encodeURIComponent(key)}`, {
            method: 'PUT',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({value: String(value)})
        });
    } catch (e) {
        // 静默处理错误
    }
}
