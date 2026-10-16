  LM Studio 聊天记录 → GGUF Studio 可导入的 ChatML txt
#
#  零依赖：Windows 自带的 PowerShell 就能运行，不需要装 Node.js
#
#  用法：
#    直接双击「一键转换LM聊天记录.bat」
#    或命令行： powershell -ExecutionPolicy Bypass -File 一键转换LM聊天记录.ps1
#
#  查找顺序：
#    上次记住的路径 → 默认位置 → LM Studio 设置里的路径 → 扫常见盘符 → 询问
# ============================================================

$ErrorActionPreference = 'Stop'
try { [Console]::OutputEncoding = [Text.Encoding]::UTF8 } catch {}

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$SavedPathFile = Join-Path $ScriptDir 'lm-import-path.txt'
$Utf8NoBom = New-Object System.Text.UTF8Encoding $false

# ---------- 输出目录：桌面 ----------
function Get-DesktopDir {
  $home2 = $env:USERPROFILE
  foreach ($n in @('Desktop', 'OneDrive\Desktop', '桌面')) {
    $p = Join-Path $home2 $n
    if (Test-Path $p) { return $p }
  }
  return $home2
}

# ---------- 目录里有没有会话文件（带上限，避免卡死）----------
function Test-HasConversations {
  param([string]$Dir, [int]$Limit = 4000)
  $seen = 0
  $stack = New-Object System.Collections.Stack
  $stack.Push($Dir)
  while ($stack.Count -gt 0) {
    $cur = $stack.Pop()
    try { $entries = Get-ChildItem -LiteralPath $cur -Force -ErrorAction Stop } catch { continue }
    foreach ($e in $entries) {
      $seen++
      if ($seen -gt $Limit) { return $false }
      if ($e.PSIsContainer) { $stack.Push($e.FullName) }
      elseif ($e.Name -like '*.conversation.json') { return $true }
    }
  }
  return $false
}

# ---------- 自动查找 ----------
function Find-Conversations {
  $home2 = $env:USERPROFILE
  $tried = New-Object System.Collections.ArrayList

  function Consider {
    param([string]$Path, [string]$Why)
    if (-not $Path) { return $null }
    [void]$tried.Add([pscustomobject]@{ Path = $Path; Why = $Why })
    if (-not (Test-Path -LiteralPath $Path)) { return $null }
    if (-not (Test-HasConversations -Dir $Path)) { return $null }
    return $Path
  }

  # ① 上次记住的
  if (Test-Path -LiteralPath $SavedPathFile) {
    $saved = (Get-Content -LiteralPath $SavedPathFile -Raw -Encoding UTF8).Trim()
    $hit = Consider -Path $saved -Why '上次记住的路径'
    if ($hit) { return [pscustomobject]@{ Dir = $hit; Tried = $tried } }
  }

  # ② 默认位置
  $def = Join-Path $home2 '.lmstudio\conversations'
  $hit = Consider -Path $def -Why '默认位置 .lmstudio\conversations'
  if ($hit) { return [pscustomobject]@{ Dir = $hit; Tried = $tried } }

  # ③ 从 LM Studio 的 settings.json 里读 downloadsFolder，猜附近
  try {
    $sp = Join-Path $home2 '.lmstudio\settings.json'
    if (Test-Path -LiteralPath $sp) {
      $j = [IO.File]::ReadAllText($sp, [Text.Encoding]::UTF8) | ConvertFrom-Json
      if ($j.downloadsFolder) {
        $hit = Consider -Path (Join-Path $j.downloadsFolder 'conversations') -Why '设置里 downloadsFolder 下面'
        if (-not $hit) {
          $parent = Split-Path -Parent $j.downloadsFolder
          $hit = Consider -Path (Join-Path $parent 'conversations') -Why '设置里 downloadsFolder 旁边'
        }
        if ($hit) { return [pscustomobject]@{ Dir = $hit; Tried = $tried } }
      }
    }
  } catch { }

  # ④ 扫常见盘符
  foreach ($drive in @('C:', 'D:', 'E:', 'F:', 'G:')) {
    $root = "$drive\"
    if (-not (Test-Path -LiteralPath $root)) { continue }
    $hit = Consider -Path (Join-Path $root '.lmstudio\conversations') -Why "$drive 根目录 .lmstudio"
    if ($hit) { return [pscustomobject]@{ Dir = $hit; Tried = $tried } }
    try { $entries = Get-ChildItem -LiteralPath $root -Directory -Force -ErrorAction Stop } catch { continue }
    foreach ($e in $entries) {
      if ($e.Name -notmatch 'lm.?studio') { continue }
      $hit = Consider -Path (Join-Path $e.FullName 'conversations') -Why "$drive\$($e.Name)\conversations"
      if (-not $hit) { $hit = Consider -Path (Join-Path $e.FullName '.lmstudio\conversations') -Why "$drive\$($e.Name)\.lmstudio" }
      if ($hit) { return [pscustomobject]@{ Dir = $hit; Tried = $tried } }
    }
  }

  return [pscustomobject]@{ Dir = $null; Tried = $tried }
}

# ---------- 工具 ----------
function Get-SafeName {
  param([string]$Name)
  if (-not $Name) { return '未命名会话' }
  $s = $Name -replace '[\\/:*?"<>|]', '_'
  $s = ($s -replace '\s+', ' ').Trim()
  if ($s.Length -gt 80) { $s = $s.Substring(0, 80) }
  if (-not $s) { return '未命名会话' }
  return $s
}

function Get-TextOf {
  param($Content)
  if ($null -eq $Content) { return '' }
  if ($Content -is [string]) { return $Content }
  $sb = New-Object System.Text.StringBuilder
  foreach ($c in $Content) {
    if ($c -and $c.type -eq 'text' -and $c.text) { [void]$sb.Append($c.text) }
  }
  return $sb.ToString()
}

function Get-ChosenVersion {
  param($Message)
  $vs = $Message.versions
  if (-not $vs) { return $null }
  $idx = 0
  if ($null -ne $Message.currentlySelected) { $idx = [int]$Message.currentlySelected }
  $arr = @($vs)
  if ($idx -ge $arr.Count) { $idx = 0 }
  return $arr[$idx]
}

# ---------- 单个会话 → ChatML ----------
function Convert-Conversation {
  param($Conv)
  $blocks = New-Object System.Collections.ArrayList
  $stat = @{ User = 0; Assistant = 0; Files = 0; Tools = 0 }

  if ($Conv.systemPrompt -and $Conv.systemPrompt.Trim()) {
    [void]$blocks.Add("<|im_start|>system`n$($Conv.systemPrompt.Trim())<|im_end|>")
  }

  foreach ($wrap in @($Conv.messages)) {
    if (-not $wrap) { continue }
    $v = Get-ChosenVersion -Message $wrap
    if (-not $v -or -not $v.role) { continue }

    if ($v.role -eq 'user') {
      foreach ($c in @($v.content)) {
        if ($c -and $c.type -eq 'file') { $stat.Files++ }
      }
      $text = (Get-TextOf -Content $v.content).Trim()
      if (-not $text) { continue }
      [void]$blocks.Add("<|im_start|>user`n$text<|im_end|>")
      $stat.User++
    }
    elseif ($v.role -eq 'assistant') {
      $parts = New-Object System.Collections.ArrayList
      $think = New-Object System.Collections.ArrayList
      foreach ($step in @($v.steps)) {
        if (-not $step) { continue }
        if ($step.type -ne 'contentBlock') {
          if ($step.type -eq 'toolStatus') { $stat.Tools++ }
          continue
        }
        $t = Get-TextOf -Content $step.content
        if (-not $t) { continue }
        # LM Studio 用 step.style.type = "thinking" 标记思考过程
        if ($step.style -and $step.style.type -eq 'thinking') { [void]$think.Add($t) }
        else { [void]$parts.Add($t) }
      }
      $answer = ($parts -join '').Trim()
      $thinking = ($think -join '').Trim()
      $text = $(if ($thinking) { "<think>`n$thinking`n</think>`n$answer" } else { $answer })
      if (-not $text) { continue }
      [void]$blocks.Add("<|im_start|>assistant`n$text<|im_end|>")
      $stat.Assistant++
    }
  }

  return [pscustomobject]@{
    ChatML = ($blocks -join "`n")
    Stat   = $stat
  }
}

# ---------- 收集会话文件 ----------
function Get-ConversationFiles {
  param([string]$Path)
  if (-not (Test-Path -LiteralPath $Path)) { return @() }
  $item = Get-Item -LiteralPath $Path -Force
  if (-not $item.PSIsContainer) {
    if ($item.Name -like '*.conversation.json') { return @($item.FullName) }
    return @()
  }
  return @(Get-ChildItem -LiteralPath $Path -Recurse -File -Force -ErrorAction SilentlyContinue |
           Where-Object { $_.Name -like '*.conversation.json' } |
           ForEach-Object { $_.FullName })
}

# ============================================================
#  主流程
# ============================================================
Write-Host ''
Write-Host '=================================================='
Write-Host '  LM Studio 聊天记录  ->  GGUF Studio 可导入的 txt'
Write-Host '=================================================='
Write-Host ''

# 输入路径
$input = $null
if ($args.Count -ge 1 -and $args[0]) { $input = $args[0] }

if (-not $input) {
  Write-Host '正在查找 LM Studio 的聊天记录目录…'
  Write-Host ''
  $found = Find-Conversations
  if ($found.Dir) {
    $input = $found.Dir
    Write-Host "  找到: $input"
  } else {
    Write-Host '  没找到。试过这些地方：'
    foreach ($t in $found.Tried | Select-Object -First 12) {
      Write-Host "     - $($t.Why)"
      Write-Host "       $($t.Path)"
    }
    Write-Host ''
    Write-Host '  请把 LM Studio 的 conversations 文件夹（或它的上级 .lmstudio）'
    Write-Host '  拖到下面这个窗口里，然后回车。'
    Write-Host ''
    $ans = Read-Host '  路径'
    if (-not $ans) { Write-Host '没有输入，退出。'; exit 1 }
    $input = $ans.Trim('"').Trim()
    if (-not (Test-Path -LiteralPath $input)) { Write-Host "找不到这个路径: $input"; exit 1 }
    try { [IO.File]::WriteAllText($SavedPathFile, $input, $Utf8NoBom); Write-Host "  已记住这个路径，下次自动使用。" } catch { }
  }
}

# 输出目录
$outDir = $null
if ($args.Count -ge 2 -and $args[1]) { $outDir = $args[1] }
else { $outDir = Join-Path (Get-DesktopDir) 'LM聊天记录' }

$files = Get-ConversationFiles -Path $input
if ($files.Count -eq 0) {
  Write-Host ''
  Write-Host '这个路径里没有找到 .conversation.json 文件。'
  Write-Host '请确认你选的是 LM Studio 的 conversations 目录（或它的上级）。'
  Write-Host ''
  exit 1
}

if (-not (Test-Path -LiteralPath $outDir)) { New-Item -ItemType Directory -Path $outDir -Force | Out-Null }

Write-Host ''
Write-Host "输入: $input"
Write-Host "找到 $($files.Count) 个会话文件"
Write-Host "输出到: $outDir"
Write-Host ''

$used = @{}
$okCount = 0; $skipCount = 0; $totalMsgs = 0

foreach ($f in $files) {
  $conv = $null
  try { $conv = [IO.File]::ReadAllText($f, [Text.Encoding]::UTF8) | ConvertFrom-Json }
  catch { Write-Host "  [跳过] JSON 解析失败: $(Split-Path -Leaf $f)"; $skipCount++; continue }

  $res = Convert-Conversation -Conv $conv
  $msgs = $res.Stat.User + $res.Stat.Assistant
  if ($msgs -eq 0) {
    $nm = if ($conv.name) { $conv.name } else { Split-Path -Leaf $f }
    Write-Host "  [跳过] 没有可导入的消息: $nm"
    $skipCount++
    continue
  }

  $base = Get-SafeName -Name $conv.name
  $out = Join-Path $outDir "$base.txt"
  $n = 2
  while ($used.ContainsKey($out.ToLower()) -or (Test-Path -LiteralPath $out)) {
    $out = Join-Path $outDir "$base ($n).txt"
    $n++
  }
  $used[$out.ToLower()] = $true

  [IO.File]::WriteAllText($out, $res.ChatML, $Utf8NoBom)
  $totalMsgs += $msgs
  $okCount++

  $extra = ''
  if ($res.Stat.Files -gt 0 -or $res.Stat.Tools -gt 0) {
    $bits = @()
    if ($res.Stat.Files -gt 0) { $bits += "附件 $($res.Stat.Files)" }
    if ($res.Stat.Tools -gt 0) { $bits += "工具调用 $($res.Stat.Tools)" }
    $extra = "  (跳过: $($bits -join '、'))"
  }
  Write-Host "  [OK] $(Split-Path -Leaf $out)   $($res.Stat.User) 用户 / $($res.Stat.Assistant) 助手$extra"
}

Write-Host ''
Write-Host "完成: $okCount 个会话已转换（共 $totalMsgs 条消息），跳过 $skipCount 个"
Write-Host ''
Write-Host '下一步：打开 GGUF Studio -> 对话列表的「导入」-> 选中上面那个目录里的 txt'
Write-Host '(可以多选；也可以一次只导一个，避免镜头里出现太多文件名)'
Write-Host ''
