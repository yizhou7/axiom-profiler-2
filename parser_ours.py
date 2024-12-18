import sys
from typing import Dict, Set
import networkx as nx
import numpy as np

class OurParser:
    def __init__(self, graph_path):
        self.qunat_iidxs: Dict[str: (str, str)] = dict()
        self.theory_iidxs: Dict[str: str] = dict()

        self.graph = nx.DiGraph()
        self.__parse_into_graph(graph_path)
    
        assert set(self.qunat_iidxs.keys()).isdisjoint(
            set(self.theory_iidxs.keys()))
        assert set(self.graph.nodes()) == set(
            self.qunat_iidxs.keys() | self.theory_iidxs.keys())

    def __parse_into_graph(self, graph_path):
        lines = open(graph_path, "r").read().split("\n")
        if lines[-1] == "":
            lines = lines[:-1]
        line_num = 0
        assert lines[line_num] == "Z3 4.13.0"
        line_num += 1

        cur = None

        # parsing the inst graph
        while lines[line_num][0] != "#":
            line = lines[line_num]
            if line[0] == "\t":
                items = line[1:].split(" ")
                assert len(items) == 2
                reason, count = items[0], int(items[1])
                if reason[0] == "i":
                    self.graph.add_edge(reason, cur, weight=count)
                else:
                    assert reason[0] == "e"
            else:
                assert line[0] == "i"
                cur = line
                self.graph.add_node(cur)
            line_num += 1
        
        # parsing the iidx to qidx mapping
        assert lines[line_num] == "# iidx_2_qidx"
        line_num += 1
        
        qunat_iidxs = dict()

        while lines[line_num][0] != "#":
            line = lines[line_num]
            line = line.split(" -> ")
            assert len(line) == 2
            iidx, qidx = line[0], line[1]
            assert iidx[0] == "i"
            assert qidx[0] == "q"
            assert iidx not in qunat_iidxs
            qunat_iidxs[iidx] = qidx
            line_num += 1

        assert lines[line_num] == "# qidx_2_qid"
        line_num += 1

        # parsing the qidx to name mapping
        qidx_to_name = dict()

        while lines[line_num][0] != "#":
            line = lines[line_num]
            line = line.split(" -> ")
            assert len(line) == 2
            qidx, name = line[0], line[1]
            assert qidx[0] == "q"
            assert qidx not in qidx_to_name
            qidx_to_name[qidx] = name
            line_num += 1

        for iidx, qidx in qunat_iidxs.items():
            assert iidx not in self.qunat_iidxs
            self.qunat_iidxs[iidx] = (qidx, qidx_to_name[qidx])

        assert lines[line_num] == "# theory_iidxs"
        line_num += 1
        
        while line_num < len(lines):
            line = lines[line_num]
            line = line.split(" -> ")
            assert len(line) == 2
            iidx, theory = line[0], line[1]
            assert iidx[0] == "i"
            self.theory_iidxs[iidx] = theory
            line_num += 1

class ReportAnalysis:
    def __init__(self, parser: OurParser):
        self.inst_graph = parser.graph
        self.qunat_iidxs: Dict[str: (str, str)] = parser.qunat_iidxs
        self.theory_iidxs: Dict[str: str] = parser.theory_iidxs

        self.iidxs_by_name: Dict[str: Set[str]] = dict()

        for iidx, (_, name) in self.qunat_iidxs.items():
            if name not in self.iidxs_by_name:
                self.iidxs_by_name[name] = set()
            self.iidxs_by_name[name].add(iidx)

        self.iidxs_by_theory: Dict[str: Set[str]] = dict()

        for iidx, theory in self.theory_iidxs.items():
            if theory not in self.iidxs_by_theory:
                self.iidxs_by_theory[theory] = set()
            self.iidxs_by_theory[theory].add(iidx)

    def get_direct_theory_impact(self, name: str):
        iidxs = self.iidxs_by_name[name]
        succ_iidxs = set()

        for iidx in iidxs:
            succ_iidxs.update(self.inst_graph.successors(iidx))

        theory_count = 0

        for iidx in succ_iidxs:
            if iidx not in self.iidx_to_qidx:
                theory_count += 1

        return theory_count
        
if __name__ == "__main__":
    p = OurParser(sys.argv[1])
    a = ReportAnalysis(p)
