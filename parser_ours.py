import sys
from typing import Dict
import networkx as nx
import numpy as np

class OurParser:
    def __init__(self, graph_path):
        self.iidx_to_qidx = dict()
        self.qidx_to_name = dict()
        self.graph = nx.DiGraph()
        self.__parse_into_graph(graph_path)

    def __parse_into_graph(self, graph_path):
        lines = open(graph_path, "r").read().split("\n")
        if lines[-1] == "":
            lines = lines[:-1]
        line_num = 0
        assert lines[line_num] == "Z3 4.13.0"
        line_num += 1

        cur = None

        while lines[line_num][0] != "#":
            line = lines[line_num]
            if line[0] == "\t":
                items = line[1:].split(" ")
                assert len(items) == 2
                reason, count = items[0], int(items[1])
                # assert reason[0] == "i" or reason[0] == "e"
                if reason[0] == "i":
                    self.graph.add_edge(reason, cur, weight=count)
                else:
                    assert reason[0] == "e"
            else:
                assert line[0] == "i"
                cur = line
                self.graph.add_node(cur)
            line_num += 1
        
        assert lines[line_num] == "# iidx_2_qidx"
        line_num += 1
        
        while lines[line_num][0] != "#":
            line = lines[line_num]
            line = line.split(" -> ")
            assert len(line) == 2
            iidx, qidx = line[0], line[1]
            assert iidx[0] == "i"
            assert qidx[0] == "q"
            assert iidx not in self.iidx_to_qidx
            self.iidx_to_qidx[iidx] = qidx
            line_num += 1

        assert lines[line_num] == "# qidx_2_qid"
        line_num += 1

        while lines[line_num][0] != "#":
            line = lines[line_num]
            line = line.split(" -> ")
            assert len(line) == 2
            qidx, name = line[0], line[1]
            assert qidx[0] == "q"
            assert qidx not in self.qidx_to_name
            self.qidx_to_name[qidx] = name
            line_num += 1
            
        assert lines[line_num] == "# theory_iidxs"
        line_num += 1

class ReportAnalysis:
    def __init__(self, parser: OurParser):
        self.inst_graph = parser.graph
        self.iidx_to_qidx = parser.iidx_to_qidx
        self.qidx_to_name = parser.qidx_to_name

        self.name_to_qidxs = dict()
        for qidx, name in self.qidx_to_name.items():
            if name not in self.name_to_qidxs:
                self.name_to_qidxs[name] = set()
            self.name_to_qidxs[name].add(qidx)

        self.name_to_iidxs = dict()
        for iidx, qidx in self.iidx_to_qidx.items():
            name = self.qidx_to_name[qidx]
            if name not in self.name_to_iidxs:
                self.name_to_iidxs[name] = set()
            self.name_to_iidxs[name].add(iidx)

    def get_direct_theory_impact(self, name: str):
        iidxs = self.name_to_iidxs[name]
        succ_iidxs = set()

        for iidx in iidxs:
            succ_iidxs.update(a.inst_graph.successors(iidx))
        
        theory_count = 0
        for iidx in succ_iidxs:
            if iidx not in a.iidx_to_qidx:
                theory_count += 1

        return theory_count
        
if __name__ == "__main__":
    p = OurParser(sys.argv[1])
    a = ReportAnalysis(p)
    # print(a.name_to_iidxs["prelude_eucmod"])
    pred_iidxs = set()

    # for iidx in a.name_to_iidxs["prelude_eucmod"]:
    #     pred_iidxs.update(a.graph.predecessors(iidx))

    for iidx in pred_iidxs:
        print(a.qidx_to_name[a.iidx_to_qidx[iidx]])
        # print(a.iidx_to_qidx[iidx])
        # print(a.qidx_to_name[a.iidx_to_qidx[iidx]])
        # print(a.graph[a.iidx_to_qidx[iidx]])

    theory_counts = dict()

    for name in a.name_to_iidxs:
        count = a.get_direct_theory_impact(name)
        theory_counts[name] = count

    for k, v in sorted(theory_counts.items(), key=lambda x: x[1], reverse=True):
        print(f"{k}: {v}")

    # for iidx in a.name_to_iidxs["prelude_eucmod"]:
    #     succ_iidxs.update(a.graph.successors(iidx))
    
    # for iidx in succ_iidxs:
    #     if iidx not in a.iidx_to_qidx:
    #         print("theory")
    #         continue
    #     print(a.qidx_to_name[a.iidx_to_qidx[iidx]])