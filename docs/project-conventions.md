# 项目规范

## 词汇

本文档使用如下词汇描述规则的强制程度：

| 词汇                  | 含义                    |
|---------------------|-----------------------|
| MUST 必须             | 规范的绝对要求               |
| MUST NOT 禁止         | 规范的绝对禁止               |
| RECOMMENDED 推荐      | 可能存在正当理由以忽略条目，但必须权衡利害 |
| NOT RECOMMENDED 不推荐 | 可能存在正当理由以忽略条目，但必须权衡利害 |
| MAY 可以              | 某些场合下可以选用条目           |

参考 [RFC-2119](https://www.ietf.org/rfc/inline-errata/rfc2119.html)

## 价值观

- 任何项目规范，优先由自动化工具强制执行，其次由自动化检查程序提供保障与拦截，尽可能避免人工干预。
  - 强调注明需要人工维护的规范。

## 版本控制

默认分支：`develop`

- **受保护** Collaborators 不得直接向 `develop` 提交代码。
- 已启用 PR 自动检查。将保障：
  - 工程可编译。
  - formatting 正确。
- 已启用自动合并。
  - 通过自动检查后自动合并。

弃用分支：`main`

- 未来发布使用。

### 工作流

> ⚠️ MUST (human maintenance):

- **MUST** 采用 Git Flow 工作流。
  - 主开发分支：`develop`
  - 功能分支：`feature/xxx`
  - 修复分支：`fix/xxx`
- **MUST** 新分支使用明确且短生命周期的命名，且需匹配 `^[a-z0-9/-]+$`，例如 `feature/inventory-ui`、
  `feature/planet-bootstrap`、
  `fix/save-init`。

---

- **RECOMMENDED** 合作者按照功能创建自己的工作分支。
- **RECOMMENDED** 一个分支只承载一项明确变更，避免在同一分支里混入多个无关功能。
- **MUST** 开发完成后，通过 Pull Request 将工作分支合并到 `develop`。
  - 已启用 PR 自动检查与自动合并。如果需要 review 流程，请手动取消自动合并。
  - PR 创建后会自动运行基础检查；检查通过后，仓库会自动执行 auto-merge。
