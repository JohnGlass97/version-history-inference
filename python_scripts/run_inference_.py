import os
import subprocess
from pathlib import Path

DEFENDER_DISABLED = True
MULTITHREADING = True
START_AT_I = 0
TAG_WITH_VERSION_COUNT = True

# ROOT OF PROJECT
os.chdir("..")

TEST_REPOS = Path("./test_repos")

while DEFENDER_DISABLED and input("Is Defender disabled? y/n: ") != "y":
    pass

for i, test_repo in enumerate(os.listdir(TEST_REPOS)):
    test_dir = TEST_REPOS / test_repo
    if not os.path.isdir(test_dir) or i < START_AT_I:
        continue

    no_versions = sum(os.path.isdir(test_dir / d)
                      for d in os.listdir(test_dir))

    if no_versions < 2:
        continue

    print("NOW STARTING", i, test_repo, f"({no_versions} versions)")

    defender = "_no_defender" if DEFENDER_DISABLED else ""
    mt = "" if MULTITHREADING else "_no_mt"
    tag = f"_{no_versions}_versions" if TAG_WITH_VERSION_COUNT else ""

    trace_name = f"perf_trace{mt}{defender}{tag}.json"

    mt_flag = "" if MULTITHREADING else "--no-multithreading"
    command = f".\\target\\release\\vhi.exe infer -d {mt_flag} -p {trace_name} {test_dir}"
    subprocess.run(command, shell=True, check=True)

print("DONE")
