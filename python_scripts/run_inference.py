import json
import os
import pandas as pd
import subprocess

from pathlib import Path

DEFENDER_DISABLED = True
MULTITHREADING = True
TAG_WITH_VERSION_COUNT = True

TEST_REPOS = Path("./test_repos")

RUNS = 5
SAVE_TRACES = False
TEMP_NAME = "perf_trace_temp.json"
OUTPUT_CSV = True


output_columns = [
    "name",
    # "run",
    "no_versions",
    "avg_files_per_version",
    "load_versions_rt",
    "infer_rt",
    "saving_rt",
    "total_rt",
]


def set_num_versions(versions_dir: Path, num_versions: int) -> bool:
    versions: list[str] = os.listdir(versions_dir)

    included_count = 0
    for version in versions:
        curr = versions_dir / version

        if not os.path.isdir(curr):
            os.rename(curr, versions_dir / version.removeprefix("ignore_"))
            continue

        include = included_count < num_versions
        needs_changed = include == version.startswith("ignore_")

        if needs_changed:
            if include:
                os.rename(curr, versions_dir / version.removeprefix("ignore_"))
            else:
                os.rename(curr, versions_dir / f"ignore_{version}")

        if include:
            included_count += 1

    assert included_count <= num_versions

    return included_count == num_versions


def produce_trace_name(num_versions, run) -> str:
    defender = "_no_defender" if DEFENDER_DISABLED else ""
    mt = "" if MULTITHREADING else "_no_mt"
    v_tag = f"_{num_versions}_versions" if TAG_WITH_VERSION_COUNT else ""

    return f"perf_trace{mt}{defender}{v_tag}_{run}.json"


def run_inference_on(
    test_repos_dir: Path,
    test_repo: str,
    version_counts: list[int] = list(range(2, 21, 2)),
) -> pd.DataFrame:
    rows = []

    for num_versions in version_counts:
        curr_dir = test_repos_dir / test_repo

        # Check if enough versions
        versions_set = set_num_versions(curr_dir, num_versions)
        if not versions_set:
            # This check should un-`ingore_` all versions
            break

        try:
            for run in range(1, RUNS + 1):
                print(f"NOW STARTING {test_repo} ({num_versions} versions) run {run}")

                # Bypass cache
                new_dir = test_repos_dir / f"{test_repo}_{run}"
                os.rename(curr_dir, new_dir)
                curr_dir = new_dir

                if SAVE_TRACES:
                    trace_name = produce_trace_name(num_versions, run)
                else:
                    trace_name = TEMP_NAME

                mt_flag = "" if MULTITHREADING else "--no-multithreading"
                command = f".\\target\\release\\vhi.exe infer -d {mt_flag} -p {trace_name} {curr_dir}"
                subprocess.run(command, shell=True, check=True)

                row = json.load(open(curr_dir / trace_name))
                row["name"] = test_repo
                row["run"] = run
                rows.append(row)
        finally:
            if not SAVE_TRACES and os.path.exists(curr_dir / TEMP_NAME):
                os.remove(curr_dir / TEMP_NAME)

            os.rename(curr_dir, test_repos_dir / test_repo)

    results_df = pd.DataFrame(rows, columns=output_columns)

    return results_df


def run_inference_all():
    results_df = pd.DataFrame(columns=output_columns)
    for test_repo in os.listdir(TEST_REPOS):
        if not os.path.isdir(TEST_REPOS / test_repo):
            continue

        # Save on completion of every run
        df = run_inference_on(TEST_REPOS, test_repo)
        results_df = pd.concat([results_df, df], ignore_index=True)
        if OUTPUT_CSV:
            results_df.to_csv(TEST_REPOS / "time_vs_versions.csv")


if __name__ == "__main__":
    while DEFENDER_DISABLED and input("Is Defender disabled? y/n: ") == "n":
        pass

    os.chdir("..")
    subprocess.run("cargo build -r --bin vhi", shell=True, check=True)

    print("Current working directory:", os.getcwd())
    run_inference_on(TEST_REPOS, "redis-forks").to_csv(
        TEST_REPOS / "time_vs_versions_redis.csv"
    )
