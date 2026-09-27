# 版本与发布

两包各自记版本、各自打标签，发布线不合并；仓库级 CHANGELOG 只做索引。

## 版本契约

- 破坏性变更指语义或模型的变更：四层结构、边界规则、蒸馏逻辑、文件含义改变，读者按原理解会得出错误结论，必须升 major 并在 CHANGELOG 写明影响面
- 内容新增升 minor，修复升 patch
- 纯路径迁移升 minor：目录迁移、文件重命名、归档规范变化，内容逐字节不变、语义与模型不变；CHANGELOG 必须附新旧路径映射表，且全部已知读者在同一次发布内同步改完，漏改一处即按破坏性变更处理
- 预发布同档递增：alpha → beta → rc → 正式

## CHANGELOG 分工

- 根 `CHANGELOG.md`：只做索引，列出各包 CHANGELOG 的路径
- `packages/dart/CHANGELOG.md`、`packages/rust/CHANGELOG.md`：版本记录正本，条目含 Added 与差异说明
- 大版本条目另需定位说明、破坏性变更与迁移指南、Removed 清单；minor 条目不套用此规格

## 发布流程

包在同一个仓库，按 scope 分别发。标签格式 `dart/vX.Y.Z` 与 `rust/vX.Y.Z`，先核对目标包 CHANGELOG 有对应版本条目：

~~~sh
cd packages/<dart|rust>
grep '^## \[0.1.0-alpha.1\]' CHANGELOG.md

# 预检：版本号、配置一致性、CHANGELOG、工作区、标签冲突、远程可达性
qtcloud-devops release audit -v <dart|rust>/v0.1.0-alpha.1

# 发布：校验 → 创建 tag → 推送 → 创建 GitHub Release
qtcloud-devops release publish -v <dart|rust>/v0.1.0-alpha.1 -y
~~~

发布后回主仓库更新子模块指针并推送：

~~~sh
cd <主仓库>
git add packages/quanttide-founder-toolkit
git commit -m "chore: update quanttide-founder-toolkit to <dart|rust>/vX.Y.Z"
git push
~~~

## 发布检查

1. 目标包 CHANGELOG 含 `## [X.Y.Z]` 条目，根索引指向正确
2. 工作区干净，门禁全绿（analyze/format/test 与 fmt/clippy/test）
3. 标签不存在，audit 七项全过
4. 发布顺序：先包、后主仓库；主仓库发布前确认子模块引用最新
