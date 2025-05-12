import os
import subprocess
from pathlib import Path

DEFENDER_DISABLED = True
MULTITHREADING = True
TAG_WITH_VERSION_COUNT = True

TEST_REPOS = Path("./test_repos")

RUNS = 5


def set_num_versions(versions_dir: Path, num_versions):
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


def run_inference_on(test_repos_dir: Path, test_repo: str, version_counts):
    for num_versions in version_counts:
        curr_dir = test_repos_dir / test_repo

        # Check if enough versions
        versions_set = set_num_versions(curr_dir, num_versions)
        if not versions_set:
            # This check should un-`ingore_` all versions
            break

        for run in range(1, RUNS + 1):
            print(f"NOW STARTING {test_repo} ({num_versions} versions) run {run}")

            # Bypass cache
            new_dir = test_repos_dir / f"{test_repo}_{run}"
            os.rename(curr_dir, new_dir)
            curr_dir = new_dir

            defender = "_no_defender" if DEFENDER_DISABLED else ""
            mt = "" if MULTITHREADING else "_no_mt"
            v_tag = f"_{num_versions}_versions" if TAG_WITH_VERSION_COUNT else ""

            trace_name = f"perf_trace{mt}{defender}{v_tag}_{run}.json"

            mt_flag = "" if MULTITHREADING else "--no-multithreading"
            command = f".\\target\\release\\vhi.exe infer -d {mt_flag} -p {trace_name} {curr_dir}"
            subprocess.run(command, shell=True, check=True)

        os.rename(curr_dir, test_repos_dir / test_repo)


def run_inference_all():
    for test_repo in os.listdir(TEST_REPOS):
        if not os.path.isdir(TEST_REPOS / test_repo):
            continue

        run_inference_on(TEST_REPOS, test_repo, list(range(1, RUNS + 1)))


if __name__ == "__main__":
    while DEFENDER_DISABLED and input("Is Defender disabled? y/n: ") == "n":
        pass

    os.chdir("..")
    print("Current working directory:", os.getcwd())
    run_inference_all()
