// Unicode 代理对清理(移植自 pi/packages/ai/src/utils/sanitize-unicode.ts)
// 对齐说明: pi 移除字符串中未配对的 UTF-16 代理项(部分 API 因此序列化报错);
// Rust String 保证合法 UTF-8, 不可能存在未配对代理, 因此恒等返回(保留函数以对齐 pi 调用点)

// 移除未配对代理字符后的字符串
pub fn sanitize_surrogates(text: &str) -> String {
    text.to_string()
}
