//! Latency measurements for the provider-oriented VCS seam.

use crate::plugin::load_prototype_plugin;
use miette::{IntoDiagnostic, miette};
use moon_pdk_api::*;
use serde::Serialize;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const DEFAULT_ITERATIONS: usize = 20;

#[derive(Debug, Serialize)]
struct BenchmarkReport {
    iterations: usize,
    passed: bool,
    metrics: Vec<Metric>,
}

#[derive(Debug, Serialize)]
struct GitComparisonReport {
    iterations: usize,
    max_regression_percent: f64,
    max_regression_ms: f64,
    master_binary: PathBuf,
    current_binary: PathBuf,
    passed: bool,
    workloads: Vec<GitComparisonMetric>,
}

#[derive(Debug, Serialize)]
struct GitComparisonMetric {
    name: &'static str,
    master: Metric,
    current: Metric,
    current_to_master_p95_ratio: f64,
    max_current_p95_ms: f64,
    passed: bool,
}

#[derive(Debug, Serialize)]
struct Metric {
    name: &'static str,
    samples: usize,
    min_ms: f64,
    mean_ms: f64,
    p50_ms: f64,
    p95_ms: f64,
    max_ms: f64,
}

impl Metric {
    fn new(name: &'static str, mut samples: Vec<Duration>) -> Self {
        samples.sort_unstable();
        let milliseconds = |duration: Duration| duration.as_secs_f64() * 1_000.0;
        let total = samples.iter().sum::<Duration>();
        let p50 = samples[(samples.len() - 1) / 2];
        let p95 = samples[((samples.len() * 95).div_ceil(100) - 1).min(samples.len() - 1)];

        Self {
            name,
            samples: samples.len(),
            min_ms: milliseconds(samples[0]),
            mean_ms: milliseconds(total) / samples.len() as f64,
            p50_ms: milliseconds(p50),
            p95_ms: milliseconds(p95),
            max_ms: milliseconds(*samples.last().expect("samples cannot be empty")),
        }
    }
}

pub async fn run(moon_root: &Path, enforce_budgets: bool) -> miette::Result<()> {
    let iterations = read_env_number("MOON_VCS_BENCH_ITERATIONS", DEFAULT_ITERATIONS)?;

    if iterations == 0 {
        return Err(miette!("benchmark iterations must be greater than zero"));
    }

    let started = Instant::now();
    let plugin = load_prototype_plugin(moon_root, moon_root).await?;
    let cold_load = started.elapsed();
    let context = MoonContext {
        working_dir: plugin.to_virtual_path(moon_root),
        workspace_root: plugin.to_virtual_path(moon_root),
    };
    let observation = plugin
        .observe(ObserveVcsInput {
            baseline: Some("master".into()),
            remote_candidates: vec![],
            consistency: VcsConsistency::ExistingObservation,
            context: context.clone(),
        })
        .await?;
    let impact_input = GetVcsImpactsInput {
        context: context.clone(),
        observation_id: observation.id,
        intent: VcsImpactIntent::Workspace,
    };
    let mut metrics = vec![Metric::new("provider_load_cold", vec![cold_load])];
    metrics.push(Metric::new(
        "provider_load_subsequent",
        sample_async(iterations, || async {
            load_prototype_plugin(moon_root, moon_root)
                .await
                .map(|_| ())
        })
        .await?,
    ));
    metrics.push(Metric::new(
        "provider_detect",
        sample_async(iterations, || async {
            plugin
                .detect(DetectVcsInput {
                    context: context.clone(),
                })
                .await
                .map(|_| ())
        })
        .await?,
    ));
    metrics.push(Metric::new(
        "provider_observe_existing",
        sample_async(iterations, || async {
            plugin
                .observe(ObserveVcsInput {
                    baseline: Some("master".into()),
                    remote_candidates: vec![],
                    consistency: VcsConsistency::ExistingObservation,
                    context: context.clone(),
                })
                .await
                .map(|_| ())
        })
        .await?,
    ));
    metrics.push(Metric::new(
        "provider_impact_raw",
        sample_async(iterations, || async {
            let _: GetVcsImpactsOutput = plugin
                .call_func_with("get_vcs_impacts", impact_input.clone())
                .await?;
            Ok(())
        })
        .await?,
    ));
    plugin.get_impacts(impact_input.clone()).await?;
    metrics.push(Metric::new(
        "provider_impact_cached",
        sample_async(iterations, || async {
            plugin.get_impacts(impact_input.clone()).await.map(|_| ())
        })
        .await?,
    ));

    let passed = metrics.iter().all(|metric| match metric.name {
        "provider_load_subsequent" => metric.p95_ms <= 100.0,
        "provider_impact_cached" => metric.p95_ms <= 1.0,
        _ => true,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&BenchmarkReport {
            iterations,
            passed,
            metrics,
        })
        .into_diagnostic()?
    );

    if enforce_budgets && !passed {
        Err(miette!(
            "source-control provider benchmark exceeded its budget"
        ))
    } else {
        Ok(())
    }
}

pub fn run_git_comparison(
    master_binary: &Path,
    current_binary: &Path,
    master_fixture: &Path,
    current_fixture: &Path,
) -> miette::Result<()> {
    let iterations = read_env_number("MOON_VCS_BENCH_ITERATIONS", DEFAULT_ITERATIONS)?;
    let max_regression_percent = read_env_number("MOON_VCS_GIT_MAX_REGRESSION_PERCENT", 5.0)?;
    let max_regression_ms = read_env_number("MOON_VCS_GIT_MAX_REGRESSION_MS", 2.0)?;

    if iterations == 0 || max_regression_percent < 0.0 || max_regression_ms < 0.0 {
        return Err(miette!("benchmark settings must not be negative or zero"));
    }

    let workloads: [(&str, &[&str]); 3] = [
        ("process_startup", &["--version"]),
        ("git_working_copy", &["query", "changed-files", "--local"]),
        (
            "git_between_revisions",
            &["query", "changed-files", "--base", "base", "--head", "HEAD"],
        ),
    ];
    let mut comparison = vec![];

    for (name, args) in workloads {
        if name != "process_startup" {
            let master_output = run_moon_capture(master_binary, master_fixture, args)?;
            let current_output = run_moon_capture(current_binary, current_fixture, args)?;

            if master_output != current_output {
                return Err(miette!(
                    "master and current `{name}` outputs differ; refusing to compare unlike work\nmaster:\n{}\ncurrent:\n{}",
                    serde_json::to_string_pretty(&master_output).into_diagnostic()?,
                    serde_json::to_string_pretty(&current_output).into_diagnostic()?,
                ));
            }
        }

        for _ in 0..3 {
            run_moon(master_binary, master_fixture, args)?;
            run_moon(current_binary, current_fixture, args)?;
        }

        let (master_samples, current_samples) = sample_moon_pair(
            iterations,
            master_binary,
            current_binary,
            master_fixture,
            current_fixture,
            args,
        )?;
        let master = Metric::new(name, master_samples);
        let current = Metric::new(name, current_samples);
        let max_current_p95_ms = (master.p95_ms * (1.0 + max_regression_percent / 100.0))
            .max(master.p95_ms + max_regression_ms);
        let passed = current.p95_ms <= max_current_p95_ms;
        let current_to_master_p95_ratio = current.p95_ms / master.p95_ms;

        comparison.push(GitComparisonMetric {
            name,
            master,
            current,
            current_to_master_p95_ratio,
            max_current_p95_ms,
            passed,
        });
    }

    let passed = comparison.iter().all(|metric| metric.passed);
    println!(
        "{}",
        serde_json::to_string_pretty(&GitComparisonReport {
            iterations,
            max_regression_percent,
            max_regression_ms,
            master_binary: master_binary.to_owned(),
            current_binary: current_binary.to_owned(),
            passed,
            workloads: comparison,
        })
        .into_diagnostic()?
    );

    if passed {
        Ok(())
    } else {
        Err(miette!(
            "current Moon exceeded the Git performance regression threshold"
        ))
    }
}

async fn sample_async<F, Fut>(iterations: usize, mut operation: F) -> miette::Result<Vec<Duration>>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = miette::Result<()>>,
{
    let mut samples = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let started = Instant::now();
        operation().await?;
        samples.push(started.elapsed());
    }

    Ok(samples)
}

fn sample_moon_pair(
    iterations: usize,
    master_binary: &Path,
    current_binary: &Path,
    master_fixture: &Path,
    current_fixture: &Path,
    args: &[&str],
) -> miette::Result<(Vec<Duration>, Vec<Duration>)> {
    let mut master_samples = Vec::with_capacity(iterations);
    let mut current_samples = Vec::with_capacity(iterations);

    for index in 0..iterations {
        let sample = |binary, fixture, samples: &mut Vec<Duration>| {
            let started = Instant::now();
            run_moon(binary, fixture, args)?;
            samples.push(started.elapsed());
            Ok::<_, miette::Report>(())
        };

        if index % 2 == 0 {
            sample(master_binary, master_fixture, &mut master_samples)?;
            sample(current_binary, current_fixture, &mut current_samples)?;
        } else {
            sample(current_binary, current_fixture, &mut current_samples)?;
            sample(master_binary, master_fixture, &mut master_samples)?;
        }
    }

    Ok((master_samples, current_samples))
}

fn run_moon(binary: &Path, fixture: &Path, args: &[&str]) -> miette::Result<()> {
    let status = moon_command(binary, fixture, args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .into_diagnostic()?;

    if status.success() {
        Ok(())
    } else {
        Err(miette!("Moon benchmark command failed: {status}"))
    }
}

fn run_moon_capture(
    binary: &Path,
    fixture: &Path,
    args: &[&str],
) -> miette::Result<serde_json::Value> {
    let output = moon_command(binary, fixture, args)
        .output()
        .into_diagnostic()?;

    if output.status.success() {
        let mut value =
            serde_json::from_slice::<serde_json::Value>(&output.stdout).into_diagnostic()?;

        if let Some(files) = value
            .get_mut("files")
            .and_then(|files| files.as_array_mut())
        {
            files.sort_by(|left, right| left.as_str().cmp(&right.as_str()));
        }

        Ok(value)
    } else {
        Err(miette!(
            "Moon benchmark command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn moon_command(binary: &Path, fixture: &Path, args: &[&str]) -> Command {
    let fixture_name = fixture
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("fixture");
    let moon_home = fixture.with_file_name(format!(".{fixture_name}-moon-home"));
    let mut command = Command::new(binary);
    command
        .args(args)
        .current_dir(fixture)
        .env("MOON_HOME", moon_home)
        .env("NO_COLOR", "1");
    command
}

fn read_env_number<T>(name: &str, default: T) -> miette::Result<T>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    std::env::var(name)
        .ok()
        .map(|value| value.parse::<T>().into_diagnostic())
        .transpose()
        .map(|value| value.unwrap_or(default))
}
