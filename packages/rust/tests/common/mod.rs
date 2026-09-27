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
