import json
import os
import pandas as pd
from pathlib import Path

# ROOT OF PROJECT
os.chdir("..")

TEST_REPOS = Path("./test_repos")

STAGES = ["load_versions", "infer"]

results_df = pd.DataFrame(
    columns=["name", "stage"] + [str(i) for i in range(2, 21, 2)])

for test_repo in os.listdir(TEST_REPOS):
    test_dir = TEST_REPOS / test_repo
    if not os.path.isdir(test_dir):
        continue

    new_rows = {s: {"name": test_repo, "stage": s} for s in STAGES}

    for file in os.listdir(test_dir):
        if os.path.isdir(test_dir / file):
            continue

        if not file.startswith("perf_trace_no_defender_") or not file.endswith("_versions.json"):
            continue

        data = json.load(open(test_dir / file))

        for stage in STAGES:
            new_rows[stage][data["no_versions"]] = data[f"{stage}_rt"]

    results_df = pd.concat(
        [results_df, pd.DataFrame(new_rows.values())], ignore_index=True)

results_df.to_csv("./test_repos/time_vs_versions.csv")
