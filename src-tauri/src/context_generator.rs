use std::path::Path;

const SKIP_DIRS: &[&str] = &[
    "node_modules", "target", ".git", "dist", "build", ".next",
    "__pycache__", ".venv", "vendor", ".idea", ".vscode",
];

const SKIP_EXTENSIONS: &[&str] = &["lock"];

fn should_skip(name: &str) -> bool {
    if SKIP_DIRS.contains(&name) {
        return true;
    }
    if let Some(ext) = Path::new(name).extension().and_then(|e| e.to_str()) {
        if SKIP_EXTENSIONS.contains(&ext) {
            return true;
        }
    }
    false
}

struct StackInfo {
    project_name: Option<String>,
    frameworks: Vec<String>,
    js_deps: Vec<String>,
    rust_deps: Vec<String>,
}

fn detect_stack(project_path: &Path) -> StackInfo {
    let mut info = StackInfo {
        project_name: None,
        frameworks: Vec::new(),
        js_deps: Vec::new(),
        rust_deps: Vec::new(),
    };

    // package.json
    let pkg_path = project_path.join("package.json");
    if pkg_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&pkg_path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(name) = json.get("name").and_then(|v| v.as_str()) {
                    info.project_name = Some(name.to_string());
                }

                let mut all_deps: Vec<String> = Vec::new();
                for key in &["dependencies", "devDependencies"] {
                    if let Some(deps) = json.get(key).and_then(|v| v.as_object()) {
                        for dep_name in deps.keys() {
                            all_deps.push(dep_name.clone());
                        }
                    }
                }

                // Detect frameworks
                let dep_set: std::collections::HashSet<&str> = all_deps.iter().map(|s| s.as_str()).collect();
                if dep_set.contains("next") || dep_set.contains("next-themes") {
                    info.frameworks.push("Next.js".to_string());
                } else if dep_set.contains("vue") {
                    let version = json.pointer("/dependencies/vue")
                        .or_else(|| json.pointer("/devDependencies/vue"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if version.contains('3') || version.starts_with("^3") || version.starts_with('3') {
                        info.frameworks.push("Vue 3".to_string());
                    } else {
                        info.frameworks.push("Vue".to_string());
                    }
                } else if dep_set.contains("react") {
                    info.frameworks.push("React".to_string());
                } else if dep_set.contains("svelte") {
                    info.frameworks.push("Svelte".to_string());
                }

                if dep_set.contains("typescript") || dep_set.contains("@types/node") {
                    info.frameworks.push("TypeScript".to_string());
                }

                // Check config files for vite and tailwind
                if project_path.join("vite.config.ts").exists()
                    || project_path.join("vite.config.js").exists()
                    || dep_set.contains("vite")
                {
                    info.frameworks.push("Vite".to_string());
                }

                if project_path.join("tailwind.config.ts").exists()
                    || project_path.join("tailwind.config.js").exists()
                    || project_path.join("tailwind.config.cjs").exists()
                    || dep_set.contains("tailwindcss")
                {
                    info.frameworks.push("Tailwind".to_string());
                }

                // Collect meaningful js deps (filter out type packages and framework itself)
                let skip_deps: std::collections::HashSet<&str> = [
                    "typescript", "@types/node", "vite", "tailwindcss", "postcss", "autoprefixer",
                    "eslint", "prettier", "@vitejs/plugin-vue", "@vitejs/plugin-react",
                    "vite-plugin-svelte", "ts-node", "@typescript-eslint/eslint-plugin",
                    "@typescript-eslint/parser",
                ].iter().copied().collect();

                for dep in &all_deps {
                    if (!dep.starts_with('@') || dep.starts_with("@tauri-apps") || dep.starts_with("@pinia"))
                        && !skip_deps.contains(dep.as_str())
                        && dep != "vue" && dep != "react" && dep != "svelte" && dep != "next"
                    {
                        info.js_deps.push(dep.clone());
                    }
                }
                info.js_deps.dedup();
            }
        }
    }

    // Cargo.toml
    let cargo_path = project_path.join("Cargo.toml");
    let src_tauri_cargo = project_path.join("src-tauri").join("Cargo.toml");
    let cargo_to_check = if src_tauri_cargo.exists() { &src_tauri_cargo } else { &cargo_path };

    if cargo_to_check.exists() {
        if let Ok(content) = std::fs::read_to_string(cargo_to_check) {
            // Extract package name
            if info.project_name.is_none() {
                for line in content.lines() {
                    let line = line.trim();
                    if line.starts_with("name") && line.contains('=') {
                        if let Some(val) = line.split('=').nth(1) {
                            let name = val.trim().trim_matches('"').trim_matches('\'').to_string();
                            if !name.is_empty() {
                                info.project_name = Some(name);
                                break;
                            }
                        }
                    }
                }
            }

            // Detect tauri version
            let has_tauri = content.contains("tauri");
            let is_tauri_v2 = content.contains("tauri = \"2") || content.contains("tauri = { version = \"2");
            if is_tauri_v2 {
                info.frameworks.push("Tauri v2".to_string());
            } else if has_tauri {
                info.frameworks.push("Tauri".to_string());
            }

            info.frameworks.push("Rust".to_string());

            // Extract key deps
            let key_deps = ["tokio", "serde", "rusqlite", "sqlx", "axum", "actix-web", "uuid", "reqwest", "anyhow", "thiserror"];
            for dep in &key_deps {
                if content.contains(dep) {
                    info.rust_deps.push(dep.to_string());
                }
            }
        }
    }

    // Python
    if project_path.join("pyproject.toml").exists() || project_path.join("requirements.txt").exists() {
        info.frameworks.push("Python".to_string());
    }

    // Go
    if project_path.join("go.mod").exists() {
        info.frameworks.push("Go".to_string());
    }

    // tsconfig
    if project_path.join("tsconfig.json").exists() && !info.frameworks.contains(&"TypeScript".to_string()) {
        info.frameworks.push("TypeScript".to_string());
    }

    info.frameworks.dedup();
    info
}

struct DirEntry {
    name: String,
    file_count: usize,
    files: Vec<String>,
    children: Vec<DirEntry>,
}

fn walk_dir(path: &Path, depth: usize, max_depth: usize) -> Option<DirEntry> {
    let name = path.file_name()?.to_str()?.to_string();

    if should_skip(&name) {
        return None;
    }

    if path.is_dir() {
        if depth > max_depth {
            let count = std::fs::read_dir(path).ok()?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_file())
                .count();
            return Some(DirEntry {
                name,
                file_count: count,
                files: Vec::new(),
                children: Vec::new(),
            });
        }

        let mut children: Vec<DirEntry> = Vec::new();
        let mut file_names: Vec<String> = Vec::new();
        let mut total_files = 0;

        if let Ok(entries) = std::fs::read_dir(path) {
            let mut sorted_entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
            sorted_entries.sort_by_key(|e| {
                let is_dir = e.path().is_dir();
                let name = e.file_name().to_string_lossy().to_string();
                (!is_dir, name)
            });

            for entry in sorted_entries {
                let entry_path = entry.path();
                let entry_name = entry.file_name().to_string_lossy().to_string();
                if should_skip(&entry_name) {
                    continue;
                }
                if entry_path.is_dir() {
                    if let Some(child) = walk_dir(&entry_path, depth + 1, max_depth) {
                        total_files += child.file_count + child.files.len();
                        children.push(child);
                    }
                } else {
                    file_names.push(entry_name);
                    total_files += 1;
                }
            }
        }

        Some(DirEntry {
            name,
            file_count: total_files,
            files: file_names,
            children,
        })
    } else {
        None
    }
}

fn detect_dir_label(name: &str) -> Option<&'static str> {
    match name {
        "components" => Some("Vue components"),
        "views" => Some("Page views"),
        "pages" => Some("Pages"),
        "stores" => Some("Pinia state management"),
        "store" => Some("State management"),
        "types" => Some("TypeScript interfaces"),
        "utils" => Some("Utilities"),
        "hooks" => Some("React hooks"),
        "composables" => Some("Vue composables"),
        "api" => Some("API layer"),
        "lib" => Some("Library code"),
        "assets" => Some("Static assets"),
        "public" => Some("Public assets"),
        "styles" | "css" => Some("Styles"),
        _ => None,
    }
}

pub fn generate_context(project_path: &Path) -> String {
    let stack = detect_stack(project_path);

    let project_name = stack.project_name
        .or_else(|| project_path.file_name().and_then(|n| n.to_str()).map(|s| s.to_string()))
        .unwrap_or_else(|| "unknown".to_string());

    let mut output = String::new();

    output.push_str(&format!("Project: {}\n", project_name));

    if !stack.frameworks.is_empty() {
        output.push_str(&format!("Stack: {}\n", stack.frameworks.join(", ")));
    }

    let mut dep_parts: Vec<String> = Vec::new();
    if !stack.js_deps.is_empty() {
        dep_parts.push(stack.js_deps.join(", "));
    }
    if !stack.rust_deps.is_empty() {
        dep_parts.push(stack.rust_deps.join(", "));
    }
    if !dep_parts.is_empty() {
        output.push_str(&format!("Deps: {}\n", dep_parts.join(" | ")));
    }

    output.push('\n');

    // Build directory tree (max depth 3)
    if let Ok(entries) = std::fs::read_dir(project_path) {
        let mut top_level: Vec<DirEntry> = Vec::new();

        let mut sorted_entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        sorted_entries.sort_by_key(|e| {
            let is_dir = e.path().is_dir();
            let name = e.file_name().to_string_lossy().to_string();
            (!is_dir, name)
        });

        let mut root_files: Vec<String> = Vec::new();

        for entry in sorted_entries {
            let name = entry.file_name().to_string_lossy().to_string();
            if should_skip(&name) {
                continue;
            }
            let path = entry.path();
            if path.is_dir() {
                if let Some(dir_entry) = walk_dir(&path, 1, 3) {
                    top_level.push(dir_entry);
                }
            } else {
                root_files.push(name);
            }
        }

        for dir in &top_level {
            let total = dir.file_count + dir.files.len();
            let label = detect_dir_label(&dir.name);
            let label_str = label.map(|l| format!(" — {}", l)).unwrap_or_default();

            if dir.children.is_empty() && dir.files.len() <= 8 {
                if !dir.files.is_empty() {
                    output.push_str(&format!("{}/\n", dir.name));
                    output.push_str(&format!("  {}\n", dir.files.join(", ")));
                } else if total > 0 {
                    output.push_str(&format!("{}/  ({} files){}\n", dir.name, total, label_str));
                }
            } else {
                if total > 0 {
                    output.push_str(&format!("{}/  ({} files){}\n", dir.name, total, label_str));
                } else {
                    output.push_str(&format!("{}/\n", dir.name));
                }

                for child in &dir.children {
                    let child_total = child.file_count + child.files.len();
                    let child_label = detect_dir_label(&child.name);
                    let child_label_str = child_label.map(|l| format!(" — {}", l)).unwrap_or_default();

                    if child.children.is_empty() && child.files.len() <= 6 && !child.files.is_empty() {
                        output.push_str(&format!("  {}/\n", child.name));
                        output.push_str(&format!("    {}\n", child.files.join(", ")));
                    } else if child_total > 0 {
                        output.push_str(&format!("  {}/  ({} files){}\n", child.name, child_total, child_label_str));
                    }
                }

                if !dir.files.is_empty() && dir.files.len() <= 8 {
                    output.push_str(&format!("  {}\n", dir.files.join(", ")));
                }
            }
        }

        // Root-level entry points
        let entry_points: Vec<&str> = root_files.iter()
            .filter(|f| matches!(f.as_str(),
                "main.rs" | "lib.rs" | "main.ts" | "main.js" | "index.ts" | "index.js" |
                "App.vue" | "App.tsx" | "App.jsx" | "index.html" | "package.json" | "Cargo.toml"
            ))
            .map(|s| s.as_str())
            .collect();
        if !entry_points.is_empty() {
            output.push_str(&format!("{}\n", entry_points.join(", ")));
        }
    }

    output.trim_end().to_string()
}

/// Estimate the optimal max_turns for a task based on prompt complexity.
/// Returns a number between 3 and 30.
pub fn estimate_max_turns(description: &str, tag: Option<&str>) -> i32 {
    let mut score: f32 = 0.0;
    let desc_lower = description.to_lowercase();
    let char_count = description.len();

    // 1. Prompt length signal
    score += match char_count {
        0..=80 => 0.0,
        81..=200 => 1.0,
        201..=500 => 2.0,
        501..=1000 => 3.0,
        _ => 4.0,
    };

    // 2. Tag signal (complexity grade)
    score += match tag {
        Some("docs") => 0.0,
        Some("bug") => 1.5,
        Some("update") => 2.0,
        Some("misc") => 1.0,
        Some("refactor") => 2.5,
        Some("feature") => 3.0,
        None => 0.0,
        _ => 1.0,
    };

    // 3. Complexity keywords
    let simple_keywords = ["fix typo", "rename", "update text", "change name", "remove unused",
                           "add comment", "update comment", "fix import", "fix lint"];
    let medium_keywords = ["fix bug", "add validation", "update", "modify", "adjust",
                           "handle error", "add test", "write test"];
    let complex_keywords = ["implement", "create", "build", "design", "refactor",
                            "migrate", "rewrite", "integrate", "add feature", "new feature",
                            "overhaul", "architect"];

    for kw in &simple_keywords {
        if desc_lower.contains(kw) { score -= 1.0; break; }
    }
    for kw in &complex_keywords {
        if desc_lower.contains(kw) { score += 2.0; break; }
    }
    for kw in &medium_keywords {
        if desc_lower.contains(kw) { score += 0.5; break; }
    }

    // 4. File/path mentions (more files = more complex)
    let file_extensions = [".rs", ".ts", ".vue", ".js", ".tsx", ".jsx", ".css", ".py", ".go"];
    let file_mentions: usize = file_extensions.iter()
        .map(|ext| description.matches(ext).count())
        .sum();
    score += (file_mentions as f32).min(4.0) * 0.5;

    // 5. Multiple action words suggest multi-step work
    let action_words = ["and", "then", "also", "additionally", "plus", "as well"];
    let action_count: usize = action_words.iter()
        .map(|w| desc_lower.matches(w).count())
        .sum();
    score += (action_count as f32).min(3.0) * 0.5;

    // Map score to turns (round to avoid truncation bias)
    match score.round() as i32 {
        i32::MIN..=0 => 3,
        1 => 5,
        2 => 8,
        3 => 12,
        4 => 15,
        5 => 20,
        6 => 25,
        _ => 30,
    }
}
