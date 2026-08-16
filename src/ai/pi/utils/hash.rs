// 快速确定性哈希(移植自 pi/packages/ai/src/utils/hash.ts), 用于缩短长字符串
// 对齐说明: 按 UTF-16 码元迭代, 乘法为 32 位环绕乘法(对齐 JS 的 Math.imul)

// 无符号 32 位整数转 base36 字符串(对齐 JS 的 (n >>> 0).toString(36))
fn to_base36(mut n: u32) -> String {
    const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if n == 0 {
        return "0".to_string();
    }
    let mut out = Vec::new();
    while n > 0 {
        out.push(DIGITS[(n % 36) as usize]);
        n /= 36;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

// 快速确定性哈希, 用于缩短长字符串
pub fn short_hash(s: &str) -> String {
    let mut h1: u32 = 0xdeadbeef;
    let mut h2: u32 = 0x41c6ce57;
    for unit in s.encode_utf16() {
        let ch = unit as u32;
        h1 = (h1 ^ ch).wrapping_mul(2654435761);
        h2 = (h2 ^ ch).wrapping_mul(1597334677);
    }
    h1 = (h1 ^ (h1 >> 16)).wrapping_mul(2246822507) ^ (h2 ^ (h2 >> 13)).wrapping_mul(3266489909);
    h2 = (h2 ^ (h2 >> 16)).wrapping_mul(2246822507) ^ (h1 ^ (h1 >> 13)).wrapping_mul(3266489909);
    format!("{}{}", to_base36(h2), to_base36(h1))
}
