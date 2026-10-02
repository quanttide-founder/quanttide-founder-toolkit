//! 测试共用件：演示 LLM 客户端 + 临时仓库夹具。
#![allow(dead_code)]

use std::fs;
use std::path::Path;

use quanttide_agent::LLMError;
use quanttide_agent::llm::{HttpClient, LLM};
use serde_json::Value;

/// 演示用 LLM 客户端——不发请求，直接回固定文本。
pub struct DemoLlm(pub &'static str);

impl HttpClient for DemoLlm {
    fn post_json(&self, _url: &str, _auth: &str, _body: &Value) -> Result<Value, LLMError> {
        Ok(serde_json::json!({
            "model": "demo",
            "choices": [{
                "message": { "role": "assistant", "content": self.0 },
                "finish_reason": "stop"
            }]
        }))
    }
}

/// 演示客户端：回一句固定文本（同 Dart 示例的 ConsoleLlm）。
///
/// 回的是纯文本而非 JSON——判据/填表都解析不了，因此它走的是规则降级路径。
pub fn demo_llm() -> LLM {
    LLM::with_client(
        "demo",
        "http://localhost",
        "",
        Box::new(DemoLlm("（演示模式，未调用 LLM）")),
    )
}

/// 演示客户端：回一段固定 JSON（给 LlmExtractor 填表用）。
pub fn json_llm(json: &'static str) -> LLM {
    LLM::with_client("demo", "http://localhost", "", Box::new(DemoLlm(json)))
}

// ---------------------------------------------------------------------------
// 语义演示客户端：按提示词里的标记回 JSON——覆盖「LLM 首选」路径
// ---------------------------------------------------------------------------

/// 语义客户端：读提示词里的判据标记，回对应的 JSON 判断。
pub struct SemanticLlm;

impl HttpClient for SemanticLlm {
    fn post_json(&self, _url: &str, _auth: &str, body: &Value) -> Result<Value, LLMError> {
        let prompt = body["messages"][0]["content"].as_str().unwrap_or_default();
        Ok(serde_json::json!({
            "model": "demo",
            "choices": [{
                "message": { "role": "assistant", "content": semantic_response(prompt) },
                "finish_reason": "stop"
            }]
        }))
    }
}

/// 语义客户端：测试用固定判断（不发网络请求）。
pub fn semantic_llm() -> LLM {
    LLM::with_client("demo", "http://localhost", "", Box::new(SemanticLlm))
}

fn semantic_response(prompt: &str) -> String {
    if prompt.starts_with("你是文档解析器") {
        return extract_response(prompt);
    }

    let criteria = judge_section(prompt, "## 判据", "## 条目");
    let items: Value = judge_section(prompt, "## 条目", "## 输出格式")
        .parse()
        .unwrap_or(Value::Array(Vec::new()));
    let allowed = allowed_decisions(criteria);

    let decision = if criteria.contains("类别判据") {
        let text = items[0].as_str().unwrap_or_default();
        // 「永远分类」：回在册的第一个类别——名字来自 YAML，代码里没有
        if text.contains("永远分类") {
            allowed.first().copied().unwrap_or("journal").to_string()
        } else if text.contains("乱码") {
            "不存在的类别".to_string()
        } else {
            test_route(text).to_string()
        }
    } else if criteria.contains("refuted、not_refuted") {
        let incoming = items[1]["statement"].as_str().unwrap_or_default();
        if incoming.contains("不是") {
            "refuted"
        } else {
            "not_refuted"
        }
        .to_string()
    } else if criteria.contains("same、different") {
        let (a, b) = (
            items[0]["statement"].as_str().unwrap_or_default(),
            items[1]["statement"].as_str().unwrap_or_default(),
        );
        if a.contains("TTL") && b.contains("TTL") {
            "same"
        } else {
            "different"
        }
        .to_string()
    } else if criteria.contains("取样判据") {
        pick_sample(&items)
    } else {
        // 分级 / 晋升 / 分流：回在册的第一个决策（取自 YAML）
        allowed.first().copied().unwrap_or("keep").to_string()
    };

    serde_json::json!({
        "decision": decision,
        "reason": "测试判断：按提示词里的判据标记回固定结果"
    })
    .to_string()
}

/// 测试侧的去向映射：只服务测试，库的判据在 YAML 里。
fn test_route(text: &str) -> &'static str {
    if text.contains("我总是") || text.contains("我习惯") {
        "profile"
    } else if text.contains("澄清") || text.contains("先做") {
        "intention"
    } else if text.contains("决定") || text.contains("优先") || text.contains("不动") {
        "roadmap"
    } else if text.contains("发现") || text.contains("原来") {
        "insight"
    } else {
        "journal"
    }
}

/// 提示词里两个标题之间的片段。
fn judge_section<'a>(prompt: &'a str, from: &str, to: &str) -> &'a str {
    prompt
        .split_once(&format!("\n{from}\n"))
        .and_then(|(_, rest)| rest.split_once(&format!("\n\n{to}\n")))
        .map(|(head, _)| head)
        .unwrap_or_default()
}

/// 判据末尾的决策表（「decision 只能取：a、b」）。
fn allowed_decisions(criteria: &str) -> Vec<&str> {
    criteria
        .lines()
        .find_map(|line| line.strip_prefix("decision 只能取："))
        .map(|rest| rest.split('、').collect())
        .unwrap_or_default()
}

/// 取样：选含「手机」的句子（测试素材的约定），否则取首句。
fn pick_sample(items: &Value) -> String {
    let lines: Vec<&str> = items
        .as_array()
        .map(|items| items.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    lines
        .iter()
        .find(|line| line.contains("手机"))
        .or_else(|| lines.first())
        .copied()
        .unwrap_or_default()
        .to_string()
}

/// 填表提示：按规则类型回对应的字段。
fn extract_response(prompt: &str) -> String {
    let document_type = prompt
        .split_once("文档类型: ")
        .and_then(|(_, rest)| rest.split('\n').next())
        .unwrap_or_default();
    let content = prompt
        .split_once("## 文档内容\n")
        .and_then(|(_, rest)| rest.split_once("\n## 输出格式"))
        .map(|(head, _)| head.trim_end())
        .unwrap_or_default();

    let value = match document_type {
        // 观察展开：回原样本（外化观察以样本为底）
        "observation" => serde_json::json!({
            "description": content,
            "sections": [],
            "fields": { "observation": content }
        }),
        "fragment" => serde_json::json!({
            "sections": [],
            "fields": { "motif": "赶末班车的角色", "scene": "她把票塞进他手里" }
        }),
        "packaging" => serde_json::json!({
            "sections": [],
            "fields": {
                "title": "她把票塞进他手里",
                "tagline": "夜班地铁的灯忽明忽暗",
                "theme": "错过与递出"
            }
        }),
        other => serde_json::json!({
            "title": other,
            "description": content,
            "sections": [],
            "fields": {}
        }),
    };
    value.to_string()
}

/// 写一个文件（含建父目录）。
pub fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("建夹具目录");
    }
    fs::write(path, content).expect("写夹具文件");
}

/// 临时 memory 仓库：
///
/// ```text
/// root/
/// ├── default/            记忆集
/// │   ├── 2026-09-01.md   集根当天日志
/// │   ├── journal/        归档日志 + README（README 不收）
/// │   ├── profile/ insight/ roadmap/
/// ├── 杂物/                不含层目录 → 不是记忆集
/// └── .hidden/            隐藏目录 → 跳过
/// ```
pub fn memory_fixture(root: &Path) {
    write_file(
        &root.join("default/2026-09-01.md"),
        "# 2026-09-01\n\n今天发现原来的缓存策略有个漏洞\n---\n\n我决定先做登录页\n",
    );
    write_file(
        &root.join("default/journal/2026-08-20.md"),
        "归档：我总是把事情拖到最后\n",
    );
    write_file(&root.join("default/journal/README.md"), "# 说明\n");
    write_file(&root.join("default/journal/not-a-date.md"), "不认\n");
    write_file(
        &root.join("default/profile/profile.md"),
        "# 张三\n\n来自长期观察的档案。\n\n## 工作方式\n\n- **决策快**：信息不全也先拍板\n- 习惯晚睡\n\n## 沟通\n\n说重点，不绕弯。\n",
    );
    write_file(
        &root.join("default/insight/insight.md"),
        "# 洞察\n\n## 已确认\n\n- **缓存失效的规律是 TTL 太长**：读多写少场景适用，依据：连续三周复现\n\n## 假说\n\n- 单点登录可能是瓶颈：待验证\n",
    );
    write_file(
        &root.join("default/roadmap/roadmap.md"),
        "# 路线图\n\n## 目标\n\n今年把发布流程跑顺。\n\n### 元目标\n\n- **自动化发布**：一条命令上线\n\n## 核心问题\n\n- **发布太慢**：手工步骤多\n\n## 已决策\n\n- 统一用一个 CI\n\n## 待决策\n\n- 是否引入灰度\n\n## 写作\n\n主题式的路线，不按方向层组织。\n",
    );
    write_file(&root.join("杂物/notes.md"), "没有层目录，不是记忆集\n");
    write_file(
        &root.join(".hidden/journal/2026-01-01.md"),
        "隐藏目录跳过\n",
    );
}

/// 临时 fiction 仓库：
///
/// ```text
/// root/
/// ├── 测试小说/           index.md + 1_灵感/ + 2_初稿/
/// ├── 观察站/             1_情绪日记/ + 2_社会观察/
/// ├── 实验室/             跳过
/// └── 杂物/                既无 index.md 也无阶段目录 → 跳过
/// ```
pub fn fiction_fixture(root: &Path) {
    write_file(
        &root.join("测试小说/index.md"),
        "# 测试小说\n\n晋江发文资料。\n",
    );
    write_file(
        &root.join("测试小说/1_灵感/0_前言.md"),
        "前言：写在正文之前。\n",
    );
    write_file(&root.join("测试小说/1_灵感/1_开头.md"), "第一章开头。\n");
    write_file(&root.join("测试小说/1_灵感/3_转折.md"), "第三章转折。\n");
    write_file(&root.join("测试小说/2_初稿/1_开头.md"), "开头初稿。\n");
    write_file(
        &root.join("测试小说/2_初稿/地摊火锅.md"),
        "未编号的替代草稿。\n",
    );
    write_file(&root.join("测试小说/README.md"), "# 说明\n");
    write_file(
        &root.join("观察站/1_情绪日记/深夜地铁.md"),
        "在地铁上看着拿手机的人，忽然想写一个赶末班车的角色。\n",
    );
    write_file(&root.join("观察站/1_情绪日记/README.md"), "# 说明\n");
    write_file(
        &root.join("观察站/2_社会观察/排队.md"),
        "排队时前面两个人换了三次位置。\n",
    );
    write_file(&root.join("实验室/notes.md"), "AI 产物，单独处理\n");
    write_file(&root.join("杂物/notes.md"), "不是小说目录\n");
}
