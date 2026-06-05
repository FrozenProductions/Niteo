use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn write_react_project(dir: &Path, count: usize) {
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::create_dir_all(dir.join("src/components")).unwrap();
    fs::create_dir_all(dir.join("src/utils")).unwrap();
    fs::create_dir_all(dir.join("src/hooks")).unwrap();

    let niteo_config = r#"
[root]
allow = ["src", "scripts"]
[rules]
"#;
    fs::write(dir.join("niteo.toml"), niteo_config).unwrap();

    let barrel = (1..=count)
        .map(|i| format!("export * from './component{}';", i))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(dir.join("src/components/index.ts"), barrel).unwrap();

    let utils_barrel = (1..=count / 2)
        .map(|i| format!("export * from './util{}';", i))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(dir.join("src/utils/index.ts"), utils_barrel).unwrap();

    for i in 1..=count {
        let content = format!(
            r#"import {{ useMemo }} from 'react';
import {{ helper{} }} from '../utils/util{}';

export interface Props{} {{
  title: string;
  count: number;
}}

export function Component{}({{ title, count }}: Props{}) {{
  const doubled = useMemo(() => count * 2, [count]);
  const value = helper{}(doubled);
  return <div>{{title}}: {{value}}</div>;
}}
"#,
            i % (count / 2).max(1) + 1,
            i % (count / 2).max(1) + 1,
            i,
            i,
            i,
            i % (count / 2).max(1) + 1,
        );
        fs::write(
            dir.join(format!("src/components/component{}.tsx", i)),
            content,
        )
        .unwrap();
    }

    for i in 1..=count / 2 {
        let content = format!(
            r#"export function helper{}(value: number): number {{
  return value * {};
}}
"#,
            i, i
        );
        fs::write(dir.join(format!("src/utils/util{}.ts", i)), content).unwrap();
    }

    for i in 1..=count / 4 {
        let content = format!(
            r#"import {{ useState, useEffect }} from 'react';

interface UseData{}Options {{
  id: string;
}}

export function useData{}({{ id }}: UseData{}Options) {{
  const [data, setData] = useState<string | null>(null);
  useEffect(() => {{
    setData(`loaded-${{id}}`);
  }}, [id]);
  return data;
}}
"#,
            i, i, i
        );
        fs::write(dir.join(format!("src/hooks/useData{}.ts", i)), content).unwrap();
    }

    let app_content = format!(
        r#"import React from 'react';
import {{ Component1 }} from './components/component1';
import {{ Component2 }} from './components/component2';
import {{ Component3 }} from './components/component3';
import {{ useData1 }} from './hooks/useData1';

export function App() {{
  const data = useData1({{ id: 'main' }});
  return (
    <div>
      <Component1 title="First" count={{1}} />
      <Component2 title="Second" count={{2}} />
      <Component3 title="Third" count={{3}} />
      <p>{{data}}</p>
    </div>
  );
}}
"#
    );
    fs::write(dir.join("src/App.tsx"), app_content).unwrap();
}

fn write_import_heavy_project(dir: &Path, count: usize) {
    fs::create_dir_all(dir.join("src")).unwrap();

    let config = r#"[root]
allow = ["src"]
[rules]
"#;
    fs::write(dir.join("niteo.toml"), config).unwrap();

    for i in 1..=count {
        let import_count = 5 + (i % 11);
        let mut imports = Vec::new();
        for j in 0..import_count {
            let target = ((i + j * 7) % count) + 1;
            imports.push(format!(
                "import {{ util{} }} from './util{}';",
                target, target
            ));
        }
        let content = format!(
            r#"{}

export function util{}(value: number): number {{
    return value * {};
}}
"#,
            imports.join("\n"),
            i,
            i
        );
        fs::write(dir.join(format!("src/util{}.ts", i)), content).unwrap();
    }

    let barrel = (1..=count)
        .map(|i| format!("export * from './util{}';", i))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(dir.join("src/index.ts"), barrel).unwrap();
}

fn bench_cli(c: &mut Criterion) {
    let mut group = c.benchmark_group("cli");
    group.sample_size(10);

    for count in [25usize, 100, 250, 1000] {
        let bench_name = format!("cli_lint_react_{}_files_json", count);
        group.bench_function(bench_name, |b| {
            let temp_dir = TempDir::new().unwrap();
            write_react_project(temp_dir.path(), count);

            b.iter(|| {
                let status = std::process::Command::new(env!("CARGO_BIN_EXE_niteo"))
                    .args(["lint", "--format", "json"])
                    .current_dir(temp_dir.path())
                    .output()
                    .unwrap();

                black_box(status);
            });
        });
    }

    let formats: [(&str, &[&str]); 2] = [("text", &[]), ("sarif", &["--format", "sarif"])];

    for (format_name, format_args) in formats {
        let bench_name = format!("cli_lint_react_100_files_{}", format_name);
        group.bench_function(bench_name, |b| {
            let temp_dir = TempDir::new().unwrap();
            write_react_project(temp_dir.path(), 100);

            b.iter(|| {
                let status = std::process::Command::new(env!("CARGO_BIN_EXE_niteo"))
                    .args(["lint"])
                    .args(format_args)
                    .current_dir(temp_dir.path())
                    .output()
                    .unwrap();

                black_box(status);
            });
        });
    }

    for count in [50usize, 100, 200] {
        let bench_name = format!("cli_lint_import_heavy_{}_files_json", count);
        group.bench_function(bench_name, |b| {
            let temp_dir = TempDir::new().unwrap();
            write_import_heavy_project(temp_dir.path(), count);

            b.iter(|| {
                let status = std::process::Command::new(env!("CARGO_BIN_EXE_niteo"))
                    .args(["lint", "--format", "json"])
                    .current_dir(temp_dir.path())
                    .output()
                    .unwrap();

                black_box(status);
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_cli);
criterion_main!(benches);
