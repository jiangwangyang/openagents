// JSON 修复与流式解析(移植自 pi/packages/ai/src/utils/json-parse.ts)
// 对齐说明: pi 依赖 partial-json 包做不完整 JSON 的容错解析, 本项目以本地实现 partial_parse 替代(不引入新依赖)
use serde_json::Value;

// 合法 JSON 转义字符集(对齐 pi VALID_JSON_ESCAPES; u 在 match 中单独处理)
const VALID_JSON_ESCAPES: [char; 8] = ['"', '\\', '/', 'b', 'f', 'n', 'r', 't'];

// 是否为控制字符
fn is_control_character(c: char) -> bool {
    (c as u32) <= 0x1f
}

// 控制字符转义
fn escape_control_character(c: char) -> String {
    match c {
        '\u{8}' => "\\b".to_string(),
        '\u{c}' => "\\f".to_string(),
        '\n' => "\\n".to_string(),
        '\r' => "\\r".to_string(),
        '\t' => "\\t".to_string(),
        _ => format!("\\u{:04x}", c as u32),
    }
}

// 修复格式错误的 JSON 字符串字面量: 转义字符串内的原始控制字符, 非法转义字符前的反斜杠翻倍
pub fn repair_json(json: &str) -> String {
    let chars: Vec<char> = json.chars().collect();
    let mut repaired = String::new();
    let mut in_string = false;
    let mut index = 0;
    while index < chars.len() {
        let c = chars[index];
        if !in_string {
            repaired.push(c);
            if c == '"' {
                in_string = true;
            }
            index += 1;
            continue;
        }
        if c == '"' {
            repaired.push(c);
            in_string = false;
            index += 1;
            continue;
        }
        if c == '\\' {
            match chars.get(index + 1).copied() {
                None => {
                    repaired.push_str("\\\\");
                    index += 1;
                    continue;
                }
                Some('u') => {
                    let digits: String = chars.iter().skip(index + 2).take(4).collect();
                    if digits.len() == 4 && digits.chars().all(|d| d.is_ascii_hexdigit()) {
                        repaired.push_str("\\u");
                        repaired.push_str(&digits);
                        index += 6;
                        continue;
                    }
                    // hex 非法/不足 4 位时按合法转义原样保留 \u(对齐 pi: VALID_JSON_ESCAPES 含 u)
                    repaired.push_str("\\u");
                    index += 2;
                    continue;
                }
                Some(next) if VALID_JSON_ESCAPES.contains(&next) => {
                    repaired.push('\\');
                    repaired.push(next);
                    index += 2;
                    continue;
                }
                _ => {}
            }
            repaired.push_str("\\\\");
            index += 1;
            continue;
        }
        if is_control_character(c) {
            repaired.push_str(&escape_control_character(c));
        } else {
            repaired.push(c);
        }
        index += 1;
    }
    repaired
}

// 解析 JSON, 失败时先修复字符串字面量再重试
pub fn parse_json_with_repair(json: &str) -> Result<Value, serde_json::Error> {
    match serde_json::from_str(json) {
        Ok(v) => Ok(v),
        Err(e) => {
            let repaired = repair_json(json);
            if repaired != json {
                if let Ok(v) = serde_json::from_str(&repaired) {
                    return Ok(v);
                }
            }
            Err(e)
        }
    }
}

// 不完整 JSON 容错解析(partial-json 的本地实现): 单趟扫描记录可安全截断的位置,
// 结尾在字符串中则补引号保留部分内容, 否则截断到最近完整 token, 再补全未闭合的对象/数组
fn partial_parse(json: &str) -> Option<Value> {
    // (截断字节位置, 该位置处理后未闭合的结构栈)
    let mut clean_points: Vec<(usize, Vec<char>)> = Vec::new();
    let mut stack: Vec<char> = Vec::new();
    let mut in_string = false;
    let mut escaped = false;
    let mut in_token = false; // 数字/字面量等裸 token 中
    for (i, c) in json.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            match c {
                '\\' => escaped = true,
                '"' => {
                    in_string = false;
                    clean_points.push((i + 1, stack.clone()));
                }
                _ => {}
            }
            continue;
        }
        if in_token {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+') {
                continue;
            }
            in_token = false;
        }
        match c {
            '"' => in_string = true,
            '{' | '[' => {
                stack.push(c);
                clean_points.push((i + 1, stack.clone()));
            }
            '}' | ']' => {
                // 失配的闭合字符直接放弃
                match (stack.pop(), c) {
                    (Some('{'), '}') | (Some('['), ']') => {}
                    _ => return None,
                }
                clean_points.push((i + 1, stack.clone()));
            }
            ',' | ':' => clean_points.push((i + 1, stack.clone())),
            c if c.is_whitespace() => {}
            _ => in_token = true,
        }
    }
    // 候选 1: 结尾在字符串中, 补引号保留部分字符串内容
    if in_string {
        let mut candidate = json.to_string();
        candidate.push('"');
        for open in stack.iter().rev() {
            candidate.push(if *open == '{' { '}' } else { ']' });
        }
        if let Ok(v) = serde_json::from_str(&candidate) {
            return Some(v);
        }
    }
    // 候选 2: 从最近完整 token 逆序尝试, 截断后补全未闭合结构
    for (point, snapshot) in clean_points.iter().rev() {
        let mut candidate = json[..*point].to_string();
        while candidate.ends_with(',') || candidate.ends_with(':') {
            candidate.pop();
        }
        if candidate.trim().is_empty() {
            continue;
        }
        for open in snapshot.iter().rev() {
            candidate.push(if *open == '{' { '}' } else { ']' });
        }
        if let Ok(v) = serde_json::from_str(&candidate) {
            return Some(v);
        }
    }
    None
}

// 解析流式传输中的不完整 JSON, 始终返回合法值(失败回退空对象)
pub fn parse_streaming_json(partial_json: &str) -> Value {
    if partial_json.trim().is_empty() {
        return Value::Object(serde_json::Map::new());
    }
    if let Ok(v) = parse_json_with_repair(partial_json) {
        return v;
    }
    if let Some(v) = partial_parse(partial_json) {
        return v;
    }
    let repaired = repair_json(partial_json);
    if repaired != partial_json {
        if let Some(v) = partial_parse(&repaired) {
            return v;
        }
    }
    Value::Object(serde_json::Map::new())
}
