import json
import os
import pandas as pd

# ROOT OF PROJECT
os.chdir("..")

columns = ["name", "version_count", "avg_files_per_version"]
time_metrics = ["load_s", "infer_s", "save_s", "total_s"]
prefixes = ["no_mt_", "base_", "no_defender_"]

for prefix in prefixes:
    columns += [prefix + x for x in time_metrics]

results_df = pd.DataFrame(columns=columns)

for test_repo in os.listdir("./test_repos"):
    test_dir = "./test_repos/" + test_repo
    if not os.path.isdir(test_dir):
        continue

    row = {}

    no_mt = json.load(open(test_dir + "/perf_trace_no_multithreading.json"))
    base = json.load(open(test_dir + "/perf_trace.json"))
    no_defender = json.load(open(test_dir + "/perf_trace_no_defender.json"))

    row["name"] = test_repo
    row["version_count"] = base["no_versions"]
    row["avg_files_per_version"] = base["avg_files_per_version"]

    for prefix, data in zip(prefixes, [no_mt, base, no_defender]):
        row[prefix + "load_s"] = data["load_versions_rt"]
        row[prefix + "infer_s"] = data["infer_rt"]
        row[prefix + "save_s"] = data["saving_rt"]
        row[prefix + "total_s"] = data["total_rt"]

    results_df = pd.concat(
        [results_df, pd.DataFrame([row])], ignore_index=True)

results_df.to_csv("./test_repos/perf_under_cond.csv")
