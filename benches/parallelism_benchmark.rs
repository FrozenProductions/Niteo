use std::fs;
use std::path::{Path, PathBuf};

use criterion::{Criterion, black_box};
use tempfile::TempDir;

use niteo::rules::check_files_for_benchmark;

fn write_project(dir: &Path, count: usize) -> Vec<PathBuf> {
    fs::create_dir_all(dir.join("src")).unwrap();

    let niteo_config = r#"
[root]
allow = ["src"]
[rules]
"#;
    fs::write(dir.join("niteo.toml"), niteo_config).unwrap();

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
        fs::write(&path, content).unwrap();
        files.push(path);
    }

    files.sort();
    files
}

fn bench_parallelism(c: &mut Criterion) {
    let mut group = c.benchmark_group("check_files_parallelism");
    group.sample_size(10);

    for count in [100usize, 500, 1000, 2000] {
        let temp_dir = TempDir::new().unwrap();
        let files = write_project(temp_dir.path(), count);

        group.bench_function(format!("single_threaded_{count}"), |b| {
            b.iter(|| {
                let result = check_files_for_benchmark(temp_dir.path(), &files, false).unwrap();
                black_box(result);
            });
        });

        group.bench_function(format!("multi_threaded_{count}"), |b| {
            b.iter(|| {
                let result = check_files_for_benchmark(temp_dir.path(), &files, true).unwrap();
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
