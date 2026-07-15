# Yune

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![MSRV](https://img.shields.io/badge/rust-1.76%2B-orange.svg)](https://www.rust-lang.org)

**语言：** [English](README.md) | 简体中文 | [粵語](README.yue.md)

> 把你打的拼音变成汉字。
> 输入 `nihao`，得到 你好。输入 `nei5 hou2`，得到粤拼的 你好。
> 使用 Rust 构建——提供原生 ABI、浏览器 WASM 和 CLI 路径。

## 目录

- [Yune 做什么](#yune-做什么)
- [为什么要有 Yune](#为什么要有-yune)
- [原理简介](#原理简介)
- [当前状态](#当前状态)
- [兼容性](#兼容性)
- [性能](#性能)
- [快速开始](#快速开始)
- [质量检查](#质量检查)
- [仓库结构](#仓库结构)
- [文档](#文档)
- [非目标](#非目标)
- [参与贡献](#参与贡献)
- [许可证](#许可证)

## Yune 做什么

你在键盘上打拼音（普通话）或粤拼（粤语），Yune 实时把它转成正确的中文字。

Yune 可以读取 [RIME](https://rime.im) 风格的字典和配置文件。对于有 oracle 证据的
命名目标和已支持的 common-schema 行为，Yune 追求可预期的兼容性；这并不表示社区
所有 RIME 输入方案和词库都已经受支持。

**[yune-web.pages.dev](https://yune-web.pages.dev)** ——在浏览器里试试。

### 已有能力

- RIME schema 与 config 处理：`__include`、`__patch`、custom patch、部署时效检查、
  schema 安装，以及 schema 切换。
- 完整输入管线：speller、selector、navigator、key binder、editor、ASCII composer、
  chord composer、punctuation、recognizer、translators、filters。
- 字典支持：`.dict.yaml` 源文件、imports、Yune 原生编译的 table/prism/reverse 产物、
  rebuild 执行，以及面向命名目标、经参照引擎验证的 fixture-backed ranking。
- C ABI 兼容：与上游一致的默认 `RimeApi` 和 `RimeLeversApi`、
  config/context/candidate/session/deploy API、动态加载测试、前端风格的生命周期测试。
- TypeDuck profile 行为：通过 `rime_get_typeduck_profile_api()` 暴露 fork-only ABI
  接口、丰富的粤拼字典注释，以及 TypeDuck-Web/Windows 兼容性证据。
- 浏览器运行时：`@yune-ime/yune-web-runtime`、`yune-web` Vite 应用、多 schema 浏览器
  测试框架（jyut6ping3、cangjie5、luna_pinyin 等）、UI 语言切换、输出标准选择、
  公开 demo、Playwright 证据。
- AI 基础：provider trait、本地/mock provider、staged AI rows、隐私政策、独立 AI
  内存、浏览器端默认关闭。

## 为什么要有 Yune

RIME 作为开源中文输入法的基石已经超过十年，它很好用。但它是一个庞大的 C++ 项目，
难以改动、难以测试，也难以嵌入到浏览器、手机应用这类现代环境中。

Yune 从零开始，用 Rust 重写整个引擎，为了三个目标：

**跨平台形态。** 同一份核心引擎可以暴露为 librime-shaped 原生共享库，用于桌面
host 实验，也可以编译成 WebAssembly 或命令行工具。现有桌面证据不表示 Squirrel、
Weasel 或 ibus-rime 已经可以无缝替换。

**可以验证。** 已覆盖的行为会和相关 RIME-family oracle 逐字节比对。Yune 不照搬
C++ 源码，而是针对命名输入捕获 oracle 输出，再断言 Yune 产生相同结果。这些证据只
覆盖已捕获的目标，不表示“每一个 RIME 行为”都已经完全一致。

**为 AI 原生输入做准备。** 引擎内置了一个默认关闭的 AI 层。将来可以在设备本地跑
一个小语言模型，在传统字典候选项旁边给出智能补全或纠错建议——不影响传统路径的
速度，也不需要把你的打字内容发到云端。

## 原理简介

```
按键  ──►  拼写规则  ──►  字典查询  ──►  排序和过滤  ──►  输出汉字
           (规范化)      (找到候选)      (排序、去重)      (提交)
```

整条处理管线用的是可替换的 Rust trait——translator、filter、ranker——而不是一个
庞大的类继承体系。想接入自定义排序模型？实现一个 trait。想换一种字典格式？换一个
translator。

deterministic core 使用 safe Rust。workspace 默认禁止 unsafe code，但
`yune-rime-api` 和 `yune-cli` 有明确的 ABI/FFI 例外。

## 当前状态

Yune 是一个活跃的引擎项目。

- **兼容性基线：** Phase 1 已为捕获的命名目标完成。M59 进一步关闭了命名的
  Lane A、Lane B、Cangjie 和 37/59 页面形状；这是目标驱动的捕获范围，不是所有
  RIME 方案的通用一致性声明。`yune-web` 有真实浏览器验证；TypeDuck-Windows
  已验证 backend/profile/IPC smoke，但不等于完整前端可无缝替换。
- **当前工作：** 里程碑 M38-M59 均已完成。最终 M59 Windows 行为源
  `443cc636` 通过已签名的 consolidated threshold registry（`32/32` aggregate、
  `160/160` individual）。较新的 `0111cf47` Mac 五轮诊断中，17 条 Track A
  中位数和 pooled worst 均低于 `1.0x`，完整输入页面为 `16/17` exact，包括
  37/59；但它与 Windows `443cc636` 不是同一源，因此不声称当前平台速度差异。
  已签名的 Windows 基线和 ceilings 未变；native/browser 内存与更细粒度的
  跨平台归因仍是独立问题。
- **公开 demo：** `yune-web` 部署在 <https://yune-web.pages.dev>。最新测量的
  pre-hardening `0111cf47` 47 键正常输入 canary 以 `51 ms` p95 / `55 ms`
  最大值、零 worker 排队通过；`68df2d16` hardening 后的 current-main web
  performance 尚未测量，浏览器启动和 footprint 也仍是独立差距。
- **AI 姿态：** AI 层已经存在，但在 web harness 中默认关闭、仅本地运行，并且不进入
  classic deterministic input path。

详见 [docs/roadmap.md](docs/roadmap.md)。

## 兼容性

Yune 的兼容性是目标驱动的，而非清单驱动的。

**参考引擎**（定义正确行为的 "oracle"）：

- 默认 core oracle：上游 `rime/librime 1.17.0`
  (`33e78140250125871856cdc5b42ddc6a5fcd3cd4`)。
- Canonical 粤拼候选 oracle：上游 `rime/librime 1.17.0` 加固定版本的
  `rime/rime-cantonese`。
- TypeDuck profile oracle：TypeDuck-HK/librime `v1.1.2`
  (`74cb52b78fb2411137a7643f6c8bc6517acfde69`)，仅用于 profile/display/comment、
  multilingual payload、fork-only ABI 和历史 fixture-backed profile guard。

**规则：**

- 保留命名目标的上游可观察行为。
- 把 TypeDuck fork 的行为隔离在 TypeDuck profile 接口后面；不得用 fork v1.1.2
  的排序定义 canonical `jyut6ping3` 候选。
- 仅在命名目标需要时才添加对应的 librime 功能。
- 期望字节不得自行推导：必须从相关 oracle 捕获，不能从 Yune 自身推出。

默认 `rime_get_api()` 保持与上游一致。TypeDuck fork-only ABI 接口仅在
`rime_get_typeduck_profile_api()` 后面暴露。

## 性能

性能结论必须按平台和证据链分别阅读。最终 M59 Windows `443cc636` 与最新
Mac `0111cf47` 的五轮 Track A 中，17 条延迟中位数和最差值均低于 `1.0x`；
但两边的 Yune 源不同，因此不能据此推断平台速度差异。Windows 签名阈值完全
不变。Track A 内存仍是明显差距：Windows 约为同轮 librime 的 `8.9x`，Mac
RSS 约为 `11.8x`。最新公平浏览器对照中，WASM 内存为 `4.000x`，payload 为
`3.471x`；pre-hardening `0111cf47` 线上 47 键正常输入以 `51 ms` p95 /
`55 ms` 最大值、零 worker 排队通过，current-main 尚未重跑。

当前单一仪表盘：

- [所有平台结果、可视化和瓶颈分析](docs/reports/yune-vs-librime-performance.md)

## 快速开始

前置条件：

- Rust 1.76 或更新版本
- Node.js 和 npm（用于浏览器 demo 和 TypeScript 运行时）
- Emscripten（仅本地构建 WASM 时需要）

构建和测试：

```bash
cargo build
cargo test --workspace
```

直接向核心引擎输入按键序列：

```bash
cargo run -p yune-cli -- run "nihao "
```

对接真实 RIME 数据，走完整 ABI 路径：

```bash
cargo run -p yune-cli -- frontend \
  --shared-data-dir ./path/to/rime-data \
  --user-data-dir ./tmp/yune-user \
  --schema luna_pinyin \
  --sequence "nihao "
```

本地运行浏览器 demo：

```bash
npm --prefix apps/yune-web install
npm --prefix apps/yune-web run build
npm --prefix apps/yune-web run start
```

浏览器验证工作请先阅读
[apps/yune-web/e2e/yune-browser-smoke.md](apps/yune-web/e2e/yune-browser-smoke.md)。

## 质量检查

重要改动合并前请运行：

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm --prefix packages/yune-web-runtime test
npm --prefix packages/yune-web-runtime run build
```

浏览器层的声明需要 Playwright 或等价的真浏览器验证。

## 仓库结构

| 路径 | 内容 |
| --- | --- |
| `crates/yune-core` | 核心引擎：字典查询、拼写规则、候选排序、过滤器、用户词典、AI 暂存。 |
| `crates/yune-rime-api` | C ABI 适配层：把引擎打包成可以替换 RIME 共享库的格式。 |
| `crates/yune-cli` | 开发者命令行：喂按键序列，出 JSON，用来测试和调试。 |
| `packages/yune-web-runtime` | WASM 构建的 TypeScript 封装。 |
| `apps/yune-web` | 浏览器 demo 应用——项目的对外展示。 |
| `docs` | 路线图、架构决策、规范、报告。 |
| `fixtures` | 确定性测试 fixture（给定输入的预期引擎输出）。 |
| `scripts` | 构建辅助、性能测试、行为抓取工具。 |

## 文档

- [docs/conventions.md](docs/conventions.md) — 架构、技术栈、代码规范、测试约定、
  ABI 规则、集成方式以及当前风险。
- [docs/roadmap.md](docs/roadmap.md) — 活跃路线图与里程碑关卡。
- [docs/decisions.md](docs/decisions.md) — 决策记录与长期原则。
- [docs/requirements.md](docs/requirements.md) — 需求 ID 与状态。
- [docs/ledgers/fork-parity-ledger.md](docs/ledgers/fork-parity-ledger.md) —
  Cantoboard 与 TypeDuck fork 相对上游的差异。
- [docs/plans/](docs/plans/) — 活跃、参考及已完成的执行记录。

## 非目标

与目标同等重要——以下是 Yune 刻意不做的：

- 逐位一致的 librime 内部实现，或完整的 C++ plugin ABI 对等。
- 没有命名目标支持的宽泛 librime 功能清单。
- 为 TypeDuck-only 行为扩宽默认上游 `RimeApi`。
- 把云端推理作为硬性依赖。
- 没有明确隐私和产品把关的 remote AI provider。
- 用 native engine 证据声称应用/浏览器性能胜出。

## 参与贡献

欢迎提交 bug report、功能提案和 pull request。任何涉及行为兼容性的改动请附带
oracle 捕获证据（相同输入下真实 RIME 的输出——期望值不能从 Yune 自身推导）。
参与前请先阅读 [docs/conventions.md](docs/conventions.md) 了解架构和编码规范。

## 许可证

原创代码使用 [MIT 许可证](LICENSE)。第三方输入方案、字典、fixtures、生成数据和
provenance 材料保留各自上游许可证——详见
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。
