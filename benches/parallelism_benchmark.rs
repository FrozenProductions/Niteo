use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use criterion::{Criterion, black_box};
use tempfile::TempDir;

use niteo::rules::check_files_for_benchmark;

fn write_project(dir: &Path, count: usize) -> Result<Vec<PathBuf>> {
    fs::create_dir_all(dir.join("src"))?;

    let niteo_config = r#"
[root]
allow = ["src"]
[rules]
"#;
    fs::write(dir.join("niteo.toml"), niteo_config)?;

    let mut files = Vec::new();
    for i in 1..=count {
        let import_a = ((i + 2) % count) + 1;
        let import_b = ((i + 5) % count) + 1;
        let content = format!(
            r#"import {{ helper{import_a} }} from './helper{import_a}';
import {{ helper{import_b} }} from './helper{import_b}';

export interface Props{i} {{
  title: string;
  count: number;
}}

export function helper{i}(value: number): number {{
  const result = value * {i};
  return result;
}}

export default function component{i}({{ title, count }}: Props{i}): string {{
  return `${{title}}: ${{helper{i}(count)}}`;
}}
"#
        );
        let path = dir.join(format!("src/helper{i}.ts"));
        fs::write(&path, content)?;
        files.push(path);
    }

    files.sort();
    Ok(files)
}

fn bench_parallelism(c: &mut Criterion) {
    let mut group = c.benchmark_group("check_files_parallelism");
    group.sample_size(10);

    for count in [100usize, 500, 1000, 2000] {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let files = write_project(temp_dir.path(), count).expect("failed to write project files");

        group.bench_function(format!("single_threaded_{count}"), |b| {
            b.iter(|| {
                let result = check_files_for_benchmark(temp_dir.path(), &files, false)
                    .expect("benchmark check failed");
                black_box(result);
            });
        });

        group.bench_function(format!("multi_threaded_{count}"), |b| {
            b.iter(|| {
                let result = check_files_for_benchmark(temp_dir.path(), &files, true)
                    .expect("benchmark check failed");
                black_box(result);
            });
        });
    }

    group.finish();
}

fn main() {
    let mut criterion = Criterion::default().configure_from_args();
    bench_parallelism(&mut criterion);
    criterion.final_summary();
}
