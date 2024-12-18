import sys, os

input_file = sys.argv[1]
assert input_file.startswith("debugger/")

base_name = os.path.basename(input_file)
graph_file = f"debugger/{base_name}.graph"
i_graph_file = f"debugger/{base_name}.igraph"
stats_file = f"debugger/{base_name}.stats"

BIN = "/home/yizhou7/axiom-profiler-2/target/release/smt-log-parser"

dep_cmd = f"{BIN} dependencies {input_file} > {graph_file}"
stats_cmd = f"{BIN} stats {input_file} > {stats_file}"
rep_cmd = f"{BIN} report {input_file} > {i_graph_file}"

# os.system(dep_cmd)
# os.system(stats_cmd)
os.system(rep_cmd)