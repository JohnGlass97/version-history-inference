import json
import os
import pandas as pd
from pathlib import Path

TEST_REPOS = Path("./test_repos")
STAGES = ["load_versions", "infer"]


def get_time_vs_versions(trace_prefix: str = "perf_trace_no_defender_") -> pd.DataFrame:
    results_df = pd.DataFrame(
        columns=["name", "stage"] + [str(i) for i in range(2, 21, 2)])

    for test_repo in os.listdir(TEST_REPOS):
        test_dir = TEST_REPOS / test_repo
        if not os.path.isdir(test_dir):
            continue

        times = {k: {} for k in STAGES}

        for file in os.listdir(test_dir):
            if os.path.isdir(test_dir / file):
                continue

            if not file.startswith(trace_prefix) or not file.endswith("_versions.json"):
                continue

            data = json.load(open(test_dir / file))

            for stage in STAGES:
                v_no = data["no_versions"]
                if v_no not in times[stage]:
                    times[stage][v_no] = []

                times[stage][v_no].append(float(data[f"{stage}_rt"]))

        new_rows = []
        for stage, t_dict in times.items():
            row = {v_no: str(sum(t_list) / len(t_list))
                   for v_no, t_list in t_dict.items()}
            row["name"] = test_repo
            row["stage"] = stage
            new_rows.append(row)

        results_df = pd.concat(
            [results_df, pd.DataFrame(new_rows)], ignore_index=True)

    return results_df


if __name__ == "__main__":
    os.chdir("..")
    results_df = get_time_vs_versions()
    results_df.to_csv("./test_repos/time_vs_versions.csv")
