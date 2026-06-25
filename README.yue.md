# Yune

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![MSRV](https://img.shields.io/badge/rust-1.76%2B-orange.svg)](https://www.rust-lang.org)

**語言：** [English](README.md) | [简体中文](README.zh-CN.md) | 粵語

> 將你打嘅拼音變成漢字。
> 打 `nihao`，出 你好。打 `nei5 hou2`，出粵拼嘅 你好。
> 用 Rust 由頭寫過——桌面、瀏覽器、邊度都行得。

## 目錄

- [Yune 做乜嘢](#yune-做乜嘢)
- [點解要有 Yune](#點解要有-yune)
- [原理簡介](#原理簡介)
- [而家狀態](#而家狀態)
- [兼容性](#兼容性)
- [性能](#性能)
- [快速開始](#快速開始)
- [品質檢查](#品質檢查)
- [倉庫結構](#倉庫結構)
- [文檔](#文檔)
- [非目標](#非目標)
- [參與貢獻](#參與貢獻)
- [許可證](#許可證)

## Yune 做乜嘢

你喺鍵盤度打拼音（普通話）或者粵拼（廣東話），Yune 即時將佢轉做啱嘅中文字。

Yune 讀嘅字典同設定檔，同 [RIME](https://rime.im) 係一樣嘅——RIME 係開源中文
輸入法入面用得最多嘅引擎。即係話，社區咁多年累積落嚟嘅幾千種 RIME 輸入方案同
詞庫，Yune 都用到。

**[yune-web.pages.dev](https://yune-web.pages.dev)** ——喺瀏覽器度即刻試下。

### 做到嘅嘢

- RIME schema 同 config 處理：`__include`、`__patch`、custom patch、deploy 時效
  檢查、schema 安裝，同埋 schema 切換。
- 完整輸入管線：speller、selector、navigator、key binder、editor、ASCII composer、
  chord composer、punctuation、recognizer、translators、filters。
- 字典支援：`.dict.yaml` 來源檔、imports、Yune 原生編譯嘅 table/prism/reverse 產物、
  rebuild 執行，同埋面向命名目標、經參照引擎驗證嘅 fixture-backed ranking。
- C ABI 兼容：同上游一致嘅預設 `RimeApi` 同 `RimeLeversApi`、
  config/context/candidate/session/deploy API、動態載入測試、前端風格生命週期測試。
- TypeDuck profile 行為：經 `rime_get_typeduck_profile_api()` 曝露 fork-only ABI
  接口、豐富粵拼字典註釋，同埋 TypeDuck-Web/Windows 兼容性證據。
- 瀏覽器 runtime：`@yune-ime/yune-web-runtime`、`yune-web` Vite app、多 schema
  瀏覽器測試框架（jyut6ping3、cangjie5、luna_pinyin 等）、UI 語言切換、輸出標準
  選擇、公開 demo、Playwright 證據。
- AI 基礎：provider trait、本地/mock provider、staged AI rows、私隱政策、獨立 AI
  記憶體、瀏覽器端預設關閉。

## 點解要有 Yune

RIME 做開源中文輸入法嘅基石已經超過十年，佢好好用。但佢係一個好大嘅 C++ 項目，
好難改、好難測試，亦好難嵌入到瀏覽器、手機 app 呢類現代環境入面。

Yune 由零開始，用 Rust 重寫成個引擎，為咗三個目標：

**邊度都行得。** 同一份核心引擎可以編譯做原生 shared library（畀 Squirrel、
Weasel、ibus-rime 呢啲桌面輸入法用），可以編譯做 WebAssembly（喺瀏覽器度行），
亦可以編譯做 command-line 工具（用嚟測試同做 performance 分析）。

**可以驗證。** 每一個行為都同真嘅 RIME 引擎逐 byte 比對。Yune 唔抄 C++ source
code——抄 code 即係抄埋 bug 同舊架構。Yune 嘅做法係將 RIME 當做「行為參考」：
餵一樣嘅輸入，capture RIME 嘅輸出，然後確保 Yune 嘅輸出一模一樣。咁樣既保住咗
兼容性，又唔使孭住一套十五年前嘅 C++ 架構。

**為 AI 原生輸入做準備。** 引擎入面有個預設熄咗嘅 AI layer。將來可以喺 device 度
行個細 language model，喺傳統字典候選字隔籬畀智能補全或者糾錯建議——唔會影響
傳統路徑嘅速度，亦唔會將你打嘅嘢 send 上 cloud。

## 原理簡介

```
撳掣  ──►  拼音規則  ──►  字典查詢  ──►  排序同過濾  ──►  出漢字
           (規範化)      (搵候選字)      (排位、去重)      (提交)
```

成條處理管線係用可以替換嘅 Rust trait 砌嘅——translator、filter、ranker——而唔係
一個大到嚇人嘅 class 繼承樹。想接入自訂排序 model？實現一個 trait。想換另一種
字典格式？換一個 translator。

全部 code 係 safe Rust，workspace 強制 `unsafe_code = "forbid"`。

## 而家狀態

Yune 係一個活躍緊嘅引擎項目。

- **兼容性基線：** Phase 1 已完成。喺普通話（`luna_pinyin`）同廣東話（`jyut6ping3`，
  經 TypeDuck profile）方案之下，Yune 輸出同 RIME 1.17.0 完全一致。已經喺真實
  frontend（TypeDuck-Web、TypeDuck-Windows）度驗證過可以無縫替換。
- **而家做緊：** milestone M38 聚焦引擎性能追平——收窄同原生 RIME 嘅剩餘速度差距。
  工作重點：native engine 啟動成本、mmap-backed `rsmarisa` table lookup、
  lazy/page-bounded candidate production、context export、記憶體同 allocation shape，
  全部以同機 RIME 證據做參照。
- **公開 demo：** `yune-web` 部署喺 <https://yune-web.pages.dev>。佢係 Yune 引擎
  demo，唔代表 browser 層嘅性能已經解決。
- **AI 姿態：** AI layer 已經存在，但喺 web harness 入面預設熄咗、只行本地，而且
  唔會走入 classic deterministic input path。

詳情睇 [docs/roadmap.md](docs/roadmap.md)。

## 兼容性

Yune 嘅兼容性係 target-driven，而唔係 checklist-driven。

**參照引擎**（定義正確行為嘅 "oracle"）：

- 預設 core oracle：上游 `rime/librime 1.17.0`
  (`33e78140250125871856cdc5b42ddc6a5fcd3cd4`)。
- TypeDuck profile oracle：TypeDuck-HK/librime `v1.1.2`
  (`74cb52b78fb2411137a7643f6c8bc6517acfde69`)。

**規則：**

- 保留命名目標嘅上游可觀察行為。
- 將 TypeDuck fork 嘅行為隔離喺 TypeDuck profile 接口後面。
- 得命名目標需要嗰陣先加對應嘅 librime 功能。
- 期望字節唔可以自己推出嚟：一定要由相關 oracle capture，唔可以喺 Yune 自己度推。

預設 `rime_get_api()` 保持同上游一致。TypeDuck fork-only ABI 接口淨係喺
`rime_get_typeduck_profile_api()` 後面曝露。

## 性能

Yune 將 RIME 當做行為同性能嘅參照點，但而家唔會 claim typing-speed、memory-footprint
或者 browser-speed 贏咗。Milestone M33-M37 已經拎走咗幾個真成本；M38 而家聚焦
用同機證據去收窄淨低嘅 native engine 差距。

目前報告：

- [docs/reports/yune-vs-librime-performance.md](docs/reports/yune-vs-librime-performance.md)
- [docs/reports/yune-vs-librime-root-cause-analysis.md](docs/reports/yune-vs-librime-root-cause-analysis.md)

## 快速開始

你要有：

- Rust 1.76 或以上
- Node.js 同 npm（for browser demo 同 TypeScript runtime）
- Emscripten（得本地 build WASM 先需要）

Build 同 test：

```bash
cargo build
cargo test --workspace
```

直接向核心引擎餵 key sequence：

```bash
cargo run -p yune-cli -- run "nihao "
```

對接真 RIME data，行完整 ABI 路徑：

```bash
cargo run -p yune-cli -- frontend \
  --shared-data-dir ./path/to/rime-data \
  --user-data-dir ./tmp/yune-user \
  --schema luna_pinyin \
  --sequence "nihao "
```

本地行 browser demo：

```bash
npm --prefix apps/yune-web install
npm --prefix apps/yune-web run build
npm --prefix apps/yune-web run start
```

做 browser validation 之前，睇咗
[apps/yune-web/e2e/yune-browser-smoke.md](apps/yune-web/e2e/yune-browser-smoke.md) 先。

## 品質檢查

重要改動 merge 之前請行：

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm --prefix packages/yune-web-runtime test
npm --prefix packages/yune-web-runtime run build
```

Browser 層嘅聲明需要 Playwright 或者等價嘅真 browser 驗證。

## 倉庫結構

| 路徑 | 係乜 |
| --- | --- |
| `crates/yune-core` | 核心引擎：字典查詢、拼音規則、候選排序、filter、user dictionary、AI staging。 |
| `crates/yune-rime-api` | C ABI 適配層：將引擎打包成可以替換 RIME shared library 嘅格式。 |
| `crates/yune-cli` | 開發者 command-line：餵 key sequence，出 JSON，用嚟測試同 debug。 |
| `packages/yune-web-runtime` | WASM build 嘅 TypeScript 封裝。 |
| `apps/yune-web` | Browser demo app——個 project 嘅對外面。 |
| `docs` | Roadmap、架構決策、規範、報告。 |
| `fixtures` | 確定性測試 fixture（畀定輸入嘅預期引擎輸出）。 |
| `scripts` | Build helper、benchmark、行為 capture 工具。 |

## 文檔

- [docs/conventions.md](docs/conventions.md) — 架構、技術棧、coding rules、test
  conventions、ABI rules、integrations、current risks。
- [docs/roadmap.md](docs/roadmap.md) — 活躍 roadmap 同 milestone 關卡。
- [docs/decisions.md](docs/decisions.md) — decision log 同長期原則。
- [docs/requirements.md](docs/requirements.md) — requirement IDs 同 status。
- [docs/ledgers/fork-parity-ledger.md](docs/ledgers/fork-parity-ledger.md) —
  Cantoboard 同 TypeDuck fork 相對上游嘅差異。
- [docs/plans/](docs/plans/) — 活躍、參考同已完成嘅 execution records。

## 非目標

同目標一樣咁重要——以下係 Yune 刻意唔做嘅：

- 逐 byte 一樣嘅 librime internals 或者完整 C++ plugin ABI parity。
- 冇命名目標嘅寬泛 librime 功能清單。
- 為 TypeDuck-only 行為擴闊預設 upstream `RimeApi`。
- 將 cloud inference 當做硬依賴。
- 冇明確 privacy 同 product gate 嘅 remote AI provider。
- 用 native engine evidence 去 claim application/browser 性能贏咗。

## 參與貢獻

歡迎提交 bug report、功能提案同 pull request。任何涉及行為兼容性嘅改動，請附上
oracle capture 證據（一樣輸入下真 RIME 嘅輸出——expected values 唔可以喺 Yune 自己
度推出嚟）。參與前請先睇 [docs/conventions.md](docs/conventions.md) 了解架構同
coding rules。

## 許可證

原創 code 用 [MIT 許可證](LICENSE)。第三方輸入方案、字典、fixtures、generated data
同 provenance materials 保留各自上游許可證——詳情睇
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。
