<div align="center">

<img src="cover2-B.png" width="100%" alt="GGUF Studio">

# GGUF Studio

**图灵卡专属的本地大模型客户端 —— 轻量、离线、开箱即用**

Tauri + llama.cpp · 空闲内存约 80 MB · 自带图灵架构推理优化

[功能](#功能) · [下载](#下载) · [快速开始](#快速开始) · [常见问题](#常见问题)

</div>

---

## 为什么是 GGUF Studio

### ⚡ 针对图灵卡优化

内置推理后端基于自研的 [llama.cpp-SM75-Turbo](https://github.com/bailuo14/llama.cpp-SM75-Turbo) —— 只保留并调优 GTX 16 系 / RTX 20 系（sm_75）需要的内核路径。

同机同模型实测（RTX 2080 Ti 22G · Qwen3.8-27B）：

| | 速度 |
|---|---|
| LM Studio | 29.0 tok/s |
| **GGUF Studio** | **35.2 tok/s** |

> 录屏会占用 GPU 导致性能下降，以上为不受录屏影响的实测值，仅供参考。

### 🪶 轻量

使用 Tauri 框架，复用系统自带的 WebView2，不打包整个 Chromium。

同样状态（停在主界面、不加载任何模型）实测：

| | 内存占用 |
|---|---|
| LM Studio | 790 MB |
| **GGUF Studio** | **约 80 MB** |

### 📦 无痛搬家

支持导入 LM Studio，Ollama的聊天记录与 MCP 配置，不需要重开对话。

---

## 功能

### 推理与模型
- 基于 llama.cpp，内核可替换为你自己编译的版本
- **显存估算**：加载前就告诉你跑不跑得动，包含 MoE 口径与 MTP 占用
- **加载进度条**：带耗时秒数，后台实时日志
- Flash Attention / KV 量化（f16 / q8_0 / q4_0）/ mmap 可切换
- **MTP 投机解码**：可调预测 token 数与接受阈值
- **Arguments Override**：完全接管命令行，或"界面设置优先"
- **思考强度**：自动读取模型的聊天模板，只列出模板真正支持的值
- **视觉理解**：支持 mmproj，可发送图片

### 角色与记忆
- 角色卡：形象图 + 系统提示词 + 独立记忆
- **跨会话长期记忆**，可从对话中自动提取
- 系统提示词方案管理，随时切换

### 对话
- 流式输出，思考过程单独展示
- 消息**编辑 / 重新生成 / 分支（⑂）**
- 代码块一键复制 + HTML 预览
- 图片附件、消息内搜索
- 会话文件夹、拖拽排序、关键词搜索

### 语音
- **离线语音朗读**（Piper / Edge TTS）
- 语速、音色、韵律可调

### 扩展
- **MCP 工具**：stdio 与 Streamable HTTP 两种传输
- 高风险工具调用前二次确认
- 可直接导入 LM Studio 的 `mcp.json`

### 云端兜底
- 支持 OpenAI 兼容的云端 API（如 DeepSeek），本地不可用时可切换

### 运维与诊断
- **一键备份 / 恢复**：设置 + 角色卡 + 记忆 + MCP + 全部对话
- **诊断信息导出**：版本 / 显卡 / 设置 / 最近日志，云端 Key 自动脱敏
- llama-server 实时日志，一键清理残留进程

### 界面
- 深色主题，Ctrl + 滚轮缩放
- 侧栏宽度、对话区宽度、字号、输入框高度均可调，自动记住

---

## 下载

### GGUF Studio（客户端）

| 版本 | 说明 | 下载 |
|---|---|---|
最新版 | (https://github.com/bailuo14/GGUF-Studio/releases/latest)
> 完整包约 116 MB（含推理内核与语音引擎）。
> 旧版功能和稳定性都不及最新版，一般建议直接用最新版。

### 附：聊天记录导出工具

把 **Ollama** 和 **LM Studio** 的聊天记录导出成 txt，双击即用，无网络功能。（工具的第 2 版）

**下载** → https://github.com/bailuo14/GGUF-Studio/releases/download/tool-v2/chat-export.zip

> 源码在 [tools/chat-export](tools/chat-export)（约 300 行 Rust，只读本地文件，不含任何网络代码）
> 另有纯 PowerShell 版（只支持 LM Studio，不含二进制文件）：[tools/lm-import.ps1](tools/lm-import.ps1)


## 支持作者

这个项目是个人业余做的，没有广告、没有会员、源码公开。

如果它帮你省了事，可以请我喝杯咖啡 ☕（完全随意，不影响任何功能）

<p align="center">
  <img src="sponsor-wechat.png" width="190" alt="微信">
  &nbsp;&nbsp;&nbsp;&nbsp;
  <img src="sponsor-alipay.png" width="190" alt="支付宝">
</p>

> 也欢迎点个 ⭐ Star，或者去B站(https://space.bilibili.com/1587548320) 留句话，都是支持。
> 
---

## 快速开始

1. 解压到任意目录（路径尽量避免中文与空格）
2. 双击 `gguf-studio.exe`
3. 点右上角 **选择模型** → 选中你的 `.gguf` 文件
4. 左侧 **加载设置** 里确认参数（不确定就用默认），点**启动**
5. 开始对话

> 首次启动如果 Windows 弹出提示，见下面「常见问题」。

---

## 环境要求

| 项 | 要求 |
|---|---|
| 系统 | Windows 10 / 11 64 位 |
| 显卡 | **NVIDIA 图灵架构**：GTX 16 系 / RTX 20 系（sm_75） |
| 驱动 | 较新的 NVIDIA 驱动 |
| 内存 | 建议 16 GB 以上 |

> 内置推理内核**只针对图灵架构编译**。其他显卡请自行编译 llama.cpp，然后在设置里用
> 「附加模式」连接你自己的服务，或改用「云端 API」后端。

---

## 常见问题

**Q：打开时提示「Windows 已保护你的电脑」（SmartScreen）？**

因为软件没有购买代码签名证书。点「更多信息」→「仍要运行」即可。

**Q：杀毒软件报毒？**

未签名 exe 的常见误报。可以把程序目录加入白名单。

**Q：我的显卡不是图灵架构，能用吗？**

内置推理内核只针对图灵优化。你可以：
- 自行编译 llama.cpp，在设置里用「附加模式」连接它（界面功能全部可用）
- 或切换到「云端 API」后端

**Q：模型文件要放在哪里？**

任意位置都行，软件只记录路径，不会移动你的文件。

**Q：怎么把 LM Studio 的聊天记录搬过来？**

支持导入聊天记录（txt / json / md）与 `mcp.json`。

**Q：为什么显存估算和我实际看到的不一样？**

估算会把模型权重、KV Cache、视觉编码器、MTP 预测头、CUDA 上下文都算进去。
其他程序（浏览器、录屏、壁纸引擎）也会占显存，所以估算值是下限而非上限。

---

## 来源与致谢

- [llama.cpp](https://github.com/ggml-org/llama.cpp) —— 推理内核，本项目的图灵优化构建基于它二次开发
- [llama.cpp-SM75-Turbo](https://github.com/bailuo14/llama.cpp-SM75-Turbo) —— 本项目的图灵架构优化构建
- [Tauri](https://tauri.app/) —— 应用框架
- 以及所有开源依赖的作者

---

## 许可

本项目采用 GNU GPL v3.0。

- 个人使用、修改、分发 —— 自由 ✅
- 商业使用 —— 也可以，但**衍生作品必须同样以 GPL 开源** ✅

如果你的公司需要**闭源商用**（嵌入自有产品而不想开源），
可以联系我获取商业授权：<qwezxcvbjj@163.com> 
---

## 免责声明

本项目仅供个人学习与本地推理使用，**不附带任何模型文件**。
模型的版权与许可归各自作者所有，使用前请自行确认其许可条款。
