use std::collections::BTreeMap;

pub fn normalize_project_local_paths(text: &str, project_path: &str) -> String {
    if text.is_empty() || project_path.is_empty() {
        return text.to_string();
    }

    let mut roots = Vec::new();
    let raw_root = project_path.trim_end_matches(std::path::MAIN_SEPARATOR);
    if !raw_root.is_empty() {
        roots.push(raw_root.to_string());
    }

    if let Ok(canonical_root) = std::fs::canonicalize(project_path) {
        if let Some(canonical) = canonical_root.to_str() {
            let canonical = canonical.trim_end_matches(std::path::MAIN_SEPARATOR);
            if !canonical.is_empty() && !roots.iter().any(|root| root == canonical) {
                roots.push(canonical.to_string());
            }
        }
    }

    let mut normalized = text.to_string();
    for root in roots {
        let root_with_sep = format!("{root}{}", std::path::MAIN_SEPARATOR);
        normalized = normalized.replace(&root_with_sep, "");
        normalized = normalized.replace(&root, ".");
    }

    normalized
}

/// Estimate the optimal max_turns for a task based on prompt complexity.
/// Returns a number between 3 and 25.
pub fn estimate_max_turns(description: &str, tag: Option<&str>) -> i32 {
    let mut score: f32 = 0.0;
    let desc_lower = description.to_lowercase();
    let char_count = description.len();

    // 1. Prompt length signal
    score += match char_count {
        0..=80 => 0.0,
        81..=200 => 0.5,
        201..=500 => 1.5,
        501..=1000 => 2.5,
        _ => 3.5,
    };

    // 2. Tag signal (complexity grade)
    score += match tag {
        Some("docs") => 0.0,
        Some("bug") => 1.0,
        Some("update") => 1.5,
        Some("misc") => 0.5,
        Some("refactor") => 2.0,
        Some("feature") => 2.5,
        None => 0.0,
        _ => 1.0,
    };

    // 3. Complexity keywords
    let simple_keywords = [
        "fix typo",
        "rename",
        "update text",
        "change name",
        "remove unused",
        "add comment",
        "update comment",
        "fix import",
        "fix lint",
    ];
    let medium_keywords = [
        "fix bug",
        "add validation",
        "update",
        "modify",
        "adjust",
        "handle error",
        "add test",
        "write test",
    ];
    let complex_keywords = [
        "implement",
        "create",
        "build",
        "design",
        "refactor",
        "migrate",
        "rewrite",
        "integrate",
        "add feature",
        "new feature",
        "overhaul",
        "architect",
    ];

    for kw in &simple_keywords {
        if desc_lower.contains(kw) {
            score -= 1.0;
            break;
        }
    }
    for kw in &complex_keywords {
        if desc_lower.contains(kw) {
            score += 2.0;
            break;
        }
    }
    for kw in &medium_keywords {
        if desc_lower.contains(kw) {
            score += 0.5;
            break;
        }
    }

    // 4. File/path mentions (more files = more complex)
    let file_extensions = [
        ".rs", ".ts", ".vue", ".js", ".tsx", ".jsx", ".css", ".py", ".go",
    ];
    let file_mentions: usize = file_extensions
        .iter()
        .map(|ext| description.matches(ext).count())
        .sum();
    score += (file_mentions as f32).min(3.0) * 0.5;

    // 5. Multiple action words suggest multi-step work
    let action_words = ["and", "then", "also", "additionally", "plus", "as well"];
    let action_count: usize = action_words
        .iter()
        .map(|w| desc_lower.matches(w).count())
        .sum();
    score += (action_count as f32).min(3.0) * 0.5;

    // Map score to turns — tighter defaults to save tokens
    match score.round() as i32 {
        i32::MIN..=0 => 3,
        1 => 3,
        2 => 5,
        3 => 8,
        4 => 10,
        5 => 15,
        6 => 20,
        _ => 25,
    }
}

pub fn generate_codebase_map(project_path: &str) -> String {
    use std::io::Read;

    const MAX_FILES: usize = 2000;
    let base = std::path::Path::new(project_path);

    let files: Vec<String> = {
        let output = std::process::Command::new("git")
            .args(["ls-files"])
            .current_dir(project_path)
            .output();
        match output {
            Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
                .lines()
                .take(MAX_FILES)
                .map(|s| s.to_string())
                .collect(),
            _ => {
                let skip_dirs = [
                    ".git",
                    "node_modules",
                    "target",
                    "dist",
                    "build",
                    ".next",
                    "__pycache__",
                    ".venv",
                    "venv",
                ];
                let mut found = Vec::new();
                let mut stack = vec![base.to_path_buf()];
                while let Some(dir) = stack.pop() {
                    if found.len() >= MAX_FILES {
                        break;
                    }
                    let entries = match std::fs::read_dir(&dir) {
                        Ok(e) => e,
                        Err(_) => continue,
                    };
                    for entry in entries.flatten() {
                        if found.len() >= MAX_FILES {
                            break;
                        }
                        let path = entry.path();
                        let name = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("")
                            .to_string();
                        if path.is_dir() {
                            if !skip_dirs.contains(&name.as_str()) {
                                stack.push(path);
                            }
                        } else {
                            let rel = path.strip_prefix(base).unwrap_or(&path);
                            found.push(rel.to_string_lossy().to_string());
                        }
                    }
                }
                found
            }
        }
    };

    let priority_exts = [".rs", ".ts", ".vue", ".py", ".go", ".js", ".jsx", ".tsx"];
    let ext_priority = |f: &str| -> usize {
        for (i, ext) in priority_exts.iter().enumerate() {
            if f.ends_with(ext) {
                return i;
            }
        }
        priority_exts.len()
    };

    let mut sorted_files = files;
    sorted_files.sort_by_key(|f| ext_priority(f));

    // BTreeMap for deterministic ordering — keeps system prompt identical across tasks for cache hits
    let mut grouped: BTreeMap<String, Vec<(String, usize)>> = BTreeMap::new();
    for rel_path in &sorted_files {
        let full_path = base.join(rel_path);

        // Read only the first 8KB to detect binary files, then count lines
        let line_count = (|| {
            let mut file = std::fs::File::open(&full_path).ok()?;
            let mut buf = [0u8; 8192];
            let n = file.read(&mut buf).ok()?;
            if buf[..n].iter().any(|&b| b == 0) {
                return None; // binary file
            }
            // For line count, read metadata size as approximation for large files
            let metadata = std::fs::metadata(&full_path).ok()?;
            let size = metadata.len() as usize;
            if size <= 8192 {
                Some(buf[..n].iter().filter(|&&b| b == b'\n').count())
            } else {
                // Read full file for accurate line count (it's text, not binary)
                let content = std::fs::read(&full_path).ok()?;
                Some(content.iter().filter(|&&b| b == b'\n').count())
            }
        })()
        .unwrap_or(0);

        let p = std::path::Path::new(rel_path);
        let dir = p
            .parent()
            .and_then(|d| d.to_str())
            .filter(|s| !s.is_empty())
            .map(|s| format!("{}/", s))
            .unwrap_or_default();
        let filename = p
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(rel_path)
            .to_string();
        grouped.entry(dir).or_default().push((filename, line_count));
    }

    let mut output = String::from("## Codebase Map\nFile structure with line counts:\n\n");
    const MAX_CHARS: usize = 12000;

    'outer: for (dir, files) in &grouped {
        if !dir.is_empty() {
            let line = format!("{}\n", dir);
            if output.len() + line.len() > MAX_CHARS {
                break;
            }
            output.push_str(&line);
        }
        for (name, lines) in files {
            let indent = if dir.is_empty() { "" } else { "  " };
            let entry = format!("{}{} ({} lines)\n", indent, name, lines);
            if output.len() + entry.len() > MAX_CHARS {
                break 'outer;
            }
            output.push_str(&entry);
        }
    }

    output
}

pub fn auto_select_model(description: &str, tag: Option<&str>, provider: &str) -> String {
    let desc_lower = description.to_lowercase();
    let mut complexity: f32 = 0.0;

    // Length signal
    complexity += match description.len() {
        0..=200 => 0.0,
        201..=500 => 0.5,
        501..=1000 => 1.0,
        _ => 1.5,
    };

    // Tag signal
    complexity += match tag {
        Some("docs") => 0.0,
        Some("misc") => 0.0,
        Some("bug") => 0.5,
        Some("update") => 0.5,
        Some("refactor") => 1.5,
        Some("feature") => 1.0,
        _ => 0.0,
    };

    // Simple task indicators (reduce complexity)
    let simple = [
        "fix typo",
        "rename",
        "update text",
        "change name",
        "remove unused",
        "add comment",
        "update comment",
        "fix import",
        "fix lint",
        "formatting",
        "add docstring",
        "update readme",
        "config change",
    ];
    for kw in &simple {
        if desc_lower.contains(kw) {
            complexity -= 1.0;
            break;
        }
    }

    // Complex task indicators (increase complexity)
    let complex = [
        "implement",
        "architect",
        "design",
        "migrate",
        "rewrite",
        "integrate",
        "overhaul",
        "multi-file",
        "across files",
        "full feature",
        "from scratch",
        "complex",
        "security",
    ];
    for kw in &complex {
        if desc_lower.contains(kw) {
            complexity += 1.5;
            break;
        }
    }

    // File count signal — multiple file mentions suggest complexity
    let file_extensions = [
        ".rs", ".ts", ".vue", ".js", ".tsx", ".jsx", ".css", ".py", ".go",
    ];
    let file_mentions: usize = file_extensions
        .iter()
        .map(|ext| description.matches(ext).count())
        .sum();
    if file_mentions >= 3 {
        complexity += 1.0;
    }

    // Threshold: only use expensive model for genuinely complex tasks
    let use_expensive = complexity >= 3.0;

    match provider {
        "codex" => {
            if use_expensive {
                "gpt-5.4".to_string()
            } else {
                "gpt-5.4-mini".to_string()
            }
        }
        _ => {
            if use_expensive {
                "claude-opus-4-6".to_string()
            } else {
                "claude-sonnet-4-6".to_string()
            }
        }
    }
}
