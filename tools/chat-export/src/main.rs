// ============================================================
//  chat-export —— 把 Ollama / LM Studio 的聊天记录导出成 txt
//
//  零依赖：SQLite 已经编译进 exe，用户不需要装任何东西。
//
//  用法：
//    双击                       → 导出到桌面的「聊天记录导出」文件夹
//    chat-export.exe <目录>     → 导出到你指定的目录
//    把文件夹拖到 exe 上         → 同上（用那个文件夹）
//
//  输出格式：ChatML（<|im_start|>role ... <|im_end|>）
//            思考过程包在 <think>...</think> 里，方便单独取出来
// ============================================================

use rusqlite::OpenFlags;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

// ---------- 小工具 ----------

fn line(s: &str) {
    println!("{}", s);
}

fn desktop_dir() -> PathBuf {
    for key in ["USERPROFILE", "HOME"] {
        if let Ok(home) = std::env::var(key) {
            let h = PathBuf::from(home);
            for name in ["Desktop", "OneDrive\\Desktop", "桌面"] {
                let p = h.join(name);
                if p.is_dir() {
                    return p;
                }
            }
            return h;
        }
    }
    PathBuf::from(".")
}

/// 文件名里不能有的字符换成下划线，并限制长度
fn safe_name(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        if "\\/:*?\"<>|\r\n\t".contains(ch) {
            out.push('_');
        } else {
            out.push(ch);
        }
    }
    let t = out.trim().trim_matches('.').trim();
    let mut t = t.to_string();
    if t.chars().count() > 60 {
        t = t.chars().take(60).collect();
    }
    if t.is_empty() {
        t = "未命名".to_string();
    }
    t
}

/// 一条消息：role / 正文 / 思考
struct Msg {
    role: String,
    content: String,
    reasoning: String,
}

fn to_chatml(msgs: &[Msg]) -> String {
    let mut s = String::new();
    for m in msgs {
        // 防止内容里混进了分隔符，破坏格式
        let c = m.content.replace("<|im_start|>", "").replace("<|im_end|>", "");
        let r = m.reasoning.replace("<|im_start|>", "").replace("<|im_end|>", "");
        s.push_str(&format!("<|im_start|>{}\n", m.role));
        if !r.trim().is_empty() {
            s.push_str("<think>\n");
            s.push_str(r.trim());
            s.push_str("\n</think>\n");
        }
        s.push_str(c.trim());
        s.push_str("<|im_end|>\n");
    }
    s
}

// ---------- Ollama ----------

fn ollama_db_path() -> Option<PathBuf> {
    let mut c: Vec<PathBuf> = Vec::new();
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        c.push(Path::new(&local).join("Ollama").join("db.sqlite"));
    }
    if let Ok(home) = std::env::var("USERPROFILE") {
        c.push(Path::new(&home).join(".ollama").join("db.sqlite"));
    }
    if let Ok(home) = std::env::var("HOME") {
        c.push(Path::new(&home).join(".local").join("share").join("ollama").join("db.sqlite"));
        c.push(
            Path::new(&home)
                .join("Library")
                .join("Application Support")
                .join("Ollama")
                .join("db.sqlite"),
        );
        c.push(Path::new(&home).join(".ollama").join("db.sqlite"));
    }
    c.into_iter().find(|p| p.is_file())
}

/// 返回 (会话标题, 消息列表)
fn read_ollama(db: &Path) -> Result<Vec<(String, Vec<Msg>)>, String> {
    let conn = rusqlite::Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("打不开 Ollama 的数据库: {e}"))?;

    let mut stmt = conn
        .prepare(
            "SELECT m.chat_id, COALESCE(c.title, ''), m.role, m.content, COALESCE(m.thinking, '') \
             FROM messages m LEFT JOIN chats c ON c.id = m.chat_id \
             ORDER BY m.chat_id, m.id",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut order: Vec<String> = Vec::new();
    let mut map: std::collections::HashMap<String, (String, Vec<Msg>)> =
        std::collections::HashMap::new();

    for row in rows {
        let (chat_id, title, role, content, thinking) = row.map_err(|e| e.to_string())?;
        if !map.contains_key(&chat_id) {
            order.push(chat_id.clone());
            map.insert(chat_id.clone(), (String::new(), Vec::new()));
        }
        let e = map.get_mut(&chat_id).unwrap();
        if e.0.is_empty() && !title.trim().is_empty() {
            e.0 = title.trim().to_string();
        }
        let role = role.to_lowercase();
        if role != "user" && role != "assistant" {
            continue;
        }
        let content = content.trim().to_string();
        let reasoning = thinking.trim().to_string();
        if content.is_empty() && reasoning.is_empty() {
            continue;
        }
        e.1.push(Msg { role, content, reasoning });
    }

    let mut out = Vec::new();
    for id in order {
        if let Some((title, msgs)) = map.remove(&id) {
            if msgs.is_empty() {
                continue;
            }
            // Ollama 不存标题，用首条用户消息兜底
            let title = if title.is_empty() {
                msgs.iter()
                    .find(|m| m.role == "user" && !m.content.trim().is_empty())
                    .map(|m| m.content.trim().replace('\n', " ").chars().take(24).collect::<String>())
                    .unwrap_or_else(|| "Ollama 会话".to_string())
            } else {
                title
            };
            out.push((title, msgs));
        }
    }
    Ok(out)
}

// ---------- LM Studio ----------

fn dir_has_conversations(dir: &Path) -> bool {
    let mut seen = 0usize;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(cur) = stack.pop() {
        let entries = match std::fs::read_dir(&cur) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for ent in entries.flatten() {
            seen += 1;
            if seen > 4000 {
                return false;
            }
            let p = ent.path();
            if p.is_dir() {
                stack.push(p);
            } else if p
                .file_name()
                .map(|n| n.to_string_lossy().ends_with(".conversation.json"))
                .unwrap_or(false)
            {
                return true;
            }
        }
    }
    false
}

fn lmstudio_dir() -> Option<PathBuf> {
    let mut c: Vec<PathBuf> = Vec::new();
    if let Ok(home) = std::env::var("USERPROFILE") {
        c.push(Path::new(&home).join(".lmstudio").join("conversations"));
    }
    if let Ok(home) = std::env::var("HOME") {
        c.push(Path::new(&home).join(".lmstudio").join("conversations"));
        c.push(Path::new(&home).join(".cache").join("lm-studio").join("conversations"));
        c.push(
            Path::new(&home)
                .join("Library")
                .join("Application Support")
                .join("LM Studio")
                .join("conversations"),
        );
    }
    // 常见盘符
    for d in ["C:", "D:", "E:", "F:", "G:"] {
        let root_str = format!("{d}\\");
        let root = Path::new(&root_str);
        if !root.is_dir() {
            continue;
        }
        c.push(root.join(".lmstudio").join("conversations"));
        if let Ok(entries) = std::fs::read_dir(root) {
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().to_lowercase();
                if !name.contains("lmstudio") && !name.contains("lm studio") && !name.contains(".lmstudio") {
                    continue;
                }
                c.push(e.path().join("conversations"));
                c.push(e.path().join(".lmstudio").join("conversations"));
            }
        }
    }
    c.into_iter().find(|p| p.is_dir() && dir_has_conversations(p))
}

fn collect_conversations(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    let mut seen = 0usize;
    while let Some(cur) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(&cur) {
            for e in entries.flatten() {
                seen += 1;
                if seen > 20000 {
                    break;
                }
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p
                    .file_name()
                    .map(|n| n.to_string_lossy().ends_with(".conversation.json"))
                    .unwrap_or(false)
                {
                    out.push(p);
                }
            }
        }
    }
    out.sort();
    out
}

fn text_of(content: &serde_json::Value) -> String {
    content
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter(|c| c.get("type").and_then(|t| t.as_str()) == Some("text"))
                .filter_map(|c| c.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default()
}

fn read_lmstudio(root: &Path) -> Vec<(String, Vec<Msg>)> {
    let mut out = Vec::new();
    for f in collect_conversations(root) {
        let fallback = f
            .file_stem()
            .map(|s| s.to_string_lossy().replace(".conversation", ""))
            .unwrap_or_else(|| "LM Studio 会话".to_string());
        let raw = match std::fs::read_to_string(&f) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let raw = raw.trim_start_matches('\u{feff}');
        let j: serde_json::Value = match serde_json::from_str(raw) {
            Ok(j) => j,
            Err(_) => continue,
        };

        let title = j
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let wraps = match j.get("messages").and_then(|m| m.as_array()) {
            Some(w) => w,
            None => continue,
        };

        let mut msgs: Vec<Msg> = Vec::new();
        for wrap in wraps {
            let versions = match wrap.get("versions").and_then(|v| v.as_array()) {
                Some(v) => v,
                None => continue,
            };
            if versions.is_empty() {
                continue;
            }
            let idx = wrap.get("currentlySelected").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let v = match versions.get(idx).or_else(|| versions.first()) {
                Some(v) => v,
                None => continue,
            };
            let role = v.get("role").and_then(|r| r.as_str()).unwrap_or("");
            match role {
                "user" => {
                    let t = text_of(&v["content"]).trim().to_string();
                    if !t.is_empty() {
                        msgs.push(Msg { role: "user".into(), content: t, reasoning: String::new() });
                    }
                }
                "assistant" => {
                    let mut answer = String::new();
                    let mut think = String::new();
                    if let Some(steps) = v.get("steps").and_then(|s| s.as_array()) {
                        for step in steps {
                            if step.get("type").and_then(|t| t.as_str()) != Some("contentBlock") {
                                continue;
                            }
                            let t = text_of(&step["content"]);
                            if t.trim().is_empty() {
                                continue;
                            }
                            // LM Studio 用 style.type = "thinking" 标记思考
                            let is_think = step
                                .get("style")
                                .and_then(|s| s.get("type"))
                                .and_then(|t| t.as_str())
                                == Some("thinking");
                            if is_think {
                                think.push_str(&t);
                            } else {
                                answer.push_str(&t);
                            }
                        }
                    }
                    let answer = answer.trim().to_string();
                    let think = think.trim().to_string();
                    if answer.is_empty() && think.is_empty() {
                        continue;
                    }
                    msgs.push(Msg { role: "assistant".into(), content: answer, reasoning: think });
                }
                _ => {}
            }
        }

        if msgs.is_empty() {
            continue;
        }
        let title = if title.is_empty() { fallback } else { title };
        out.push((title, msgs));
    }
    out
}

// ---------- 写文件 ----------

fn write_all(out_dir: &Path, tag: &str, list: Vec<(String, Vec<Msg>)>, quiet: bool) -> (usize, usize) {
    let mut ok = 0usize;
    let mut msgs = 0usize;
    let mut used: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (title, m) in list {
        let base = format!("{}-{}", tag, safe_name(&title));
        let mut path = out_dir.join(format!("{base}.txt"));
        let mut n = 2;
        while used.contains(&path.to_string_lossy().to_lowercase()) || path.exists() {
            path = out_dir.join(format!("{base} ({n}).txt"));
            n += 1;
        }
        used.insert(path.to_string_lossy().to_lowercase());

        let text = to_chatml(&m);
        if fs::write(&path, text).is_err() {
            continue;
        }
        msgs += m.len();
        ok += 1;
        if !quiet {
            line(&format!(
                "    [OK] {}  ({} 条消息)",
                path.file_name().unwrap_or_default().to_string_lossy(),
                m.len()
            ));
        }
    }
    (ok, msgs)
}

// ---------- main ----------

fn main() {
    line("");
    line("  ============================================================");
    line("   聊天记录导出工具   Ollama / LM Studio  ->  txt");
    line("  ============================================================");
    line("");

    let args: Vec<String> = std::env::args().collect();
    // -q / --quiet: 不打印每个文件名和路径（录屏、分享截图时用）
    let quiet = args.iter().any(|a| a == "-q" || a == "--quiet");
    let interactive = args.len() <= 1;
    let out_dir = args
        .iter()
        .skip(1)
        .find(|a| !a.starts_with('-'))
        .map(PathBuf::from)
        .unwrap_or_else(|| desktop_dir().join("聊天记录导出"));

    if let Err(e) = fs::create_dir_all(&out_dir) {
        line(&format!("  [x] 建不了输出目录: {e}"));
        line(&format!("      {}", out_dir.display()));
        pause(interactive);
        return;
    }
    line(&format!("  输出到: {}", out_dir.display()));
    line("");

    let mut total_files = 0usize;
    let mut total_msgs = 0usize;
    let mut found_any = false;

    // ---- Ollama ----
    line("  --- Ollama ---");
    match ollama_db_path() {
        Some(db) => {
            if !quiet { line(&format!("  找到数据库: {}", db.display())); }
            match read_ollama(&db) {
                Ok(list) => {
                    let n = list.len();
                    let (f, m) = write_all(&out_dir, "Ollama", list, quiet);
                    line(&format!("  ✅ {n} 个会话，导出 {f} 个文件、{m} 条消息"));
                    total_files += f;
                    total_msgs += m;
                    if f > 0 {
                        found_any = true;
                    }
                }
                Err(e) => line(&format!("  ⚠️ {e}")),
            }
        }
        None => line("  （没找到 Ollama 的聊天库，跳过）"),
    }
    line("");

    // ---- LM Studio ----
    line("  --- LM Studio ---");
    match lmstudio_dir() {
        Some(dir) => {
            if !quiet { line(&format!("  找到目录: {}", dir.display())); }
            let list = read_lmstudio(&dir);
            let n = list.len();
            let (f, m) = write_all(&out_dir, "LMStudio", list, quiet);
            line(&format!("  ✅ {n} 个会话，导出 {f} 个文件、{m} 条消息"));
            total_files += f;
            total_msgs += m;
            if f > 0 {
                found_any = true;
            }
        }
        None => line("  （没找到 LM Studio 的聊天记录，跳过）"),
    }

    line("");
    line("  ============================================================");
    if found_any {
        line(&format!("   完成：共 {total_files} 个 txt、{total_msgs} 条消息"));
        line(&format!("   位置：{}", out_dir.display()));
    } else {
        line("   没有找到任何聊天记录。");
        line("   如果你确实用过 Ollama 或 LM Studio，请把它们打开过一次再试。");
    }
    line("  ============================================================");
    line("");
    line("  提示：思考过程包在 <think>...</think> 里，用任何文本编辑器都能看。");
    line("        也可以直接导入 GGUF Studio（对话列表的「导入」）。");
    line("");

    pause(interactive);
}

fn pause(interactive: bool) {
    if !interactive {
        return;
    }
    line("  按回车键关闭…");
    let _ = std::io::stdout().flush();
    let mut s = String::new();
    let _ = std::io::stdin().read_line(&mut s);
}
