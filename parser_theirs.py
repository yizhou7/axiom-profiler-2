from typing import Dict
import networkx as nx
import numpy as np
import sys

class InstBlame:
    def __init__(self, qidx, cost):
        self.qidx = qidx
        self.reasons = dict()
        self.cost = cost
        self.blamed_count = 0
        self.stat_count = None

    def add_reason(self, reason_qidx, count):
        if reason_qidx not in self.reasons:
            self.reasons[reason_qidx] = 0
        self.reasons[reason_qidx] += count
        self.blamed_count += count

def read_file_into_list(file_name):
    lines = open(file_name, "r").read().split("\n")
    if lines[-1] == "":
        lines = lines[:-1]
    return lines

class TheirParser:
    def __init__(self, graph_path):
        self.qidx_to_name = dict()
        stats_path = graph_path.replace("graph", "stats")
        blames = self.__parse_into_blames(graph_path)
        self.__deduplicate_blames(blames)
        self.__parse_stats(stats_path)

    def __parse_into_blames(self, graph_path):
        lines = read_file_into_list(graph_path)
        line_no = 0
        assert lines[line_no] == "Z3 4.13.0"
        line_no += 1

        cur = None
        
        blames = []

        while line_no < len(lines):
            line = lines[line_no]
            if line[0] == "\t":
                items = line[1:].split(" ")
                name, qidx, count = items[0], int(items[1]), int(items[2])
                self.__register_name(qidx, name)
                cur.add_reason(qidx, count)
            else:
                items = line.split(" ")
                name, qidx, cost = items[0], int(items[1]), float(items[2])
                self.__register_name(qidx, name)
                cur = InstBlame(qidx, cost)
                blames.append(cur)
            line_no += 1

        return blames

    def __register_name(self, qidx, name):
        if qidx not in self.qidx_to_name:
            self.qidx_to_name[qidx] = name
        else:
            assert self.qidx_to_name[qidx] == name

    def __deduplicate_blames(self, blames):
        grouped = dict()
        for blame in blames:
            if blame.qidx not in grouped:
                grouped[blame.qidx] = []
            grouped[blame.qidx].append(blame)

        kept = dict()

        for qidx, group in grouped.items():
            non_zero_items = [b for b in group if b.cost != 0]
            assert len(non_zero_items) <= 1
            if len(non_zero_items) == 1:
                kept[qidx] = non_zero_items[0]

        self.blames = kept
        kept = set(kept.keys())

        for qidx in set(self.qidx_to_name.keys()) - kept:
            del self.qidx_to_name[qidx]

    def __parse_stats(self, stats_path):
        lines = read_file_into_list(stats_path)
        line_no = 0
        start = False
        for line in lines:
            if line == "top-instantiations=":
                start = True
                continue
            if not start:
                continue
            items = line.split(" ")
            name, qidx, count = items[0], int(items[1]), int(items[2])
            if qidx not in self.blames:
                assert count == 0
                continue
            self.blames[qidx].stat_count = count

class TheirAnalysis:
    def __init__(self, parser: TheirParser):
        self.qidx_to_name = parser.qidx_to_name
        self.name_to_qidxs = dict()

        for qidx, name in self.qidx_to_name.items():
            if name not in self.name_to_qidxs:
                self.name_to_qidxs[name] = set()
            self.name_to_qidxs[name].add(qidx)

        self.blames = parser.blames
        self.graph = nx.DiGraph()

        for qidx, blame in self.blames.items():
            self.graph.add_node(qidx, name=self.qidx_to_name[qidx])

            for reason_qidx, count in blame.reasons.items():
                self.graph.add_edge(reason_qidx, qidx, weight=count)

    def get_qidx_checked(self, name):
        qidxs = self.name_to_qidxs[name]
        assert len(qidxs) == 1
        return list(qidxs)[0]
    
    def debug_name(self, name):
        qidx = self.get_qidx_checked(name)
        print(f"{name} [{qidx}]:")
        print(f"cost: {self.blames[qidx].cost}")
        print(f"stat_count: {self.blames[qidx].stat_count}")
        print(f"blamed_count: {self.blames[qidx].blamed_count}")
        print("predecessors:")
        for reason_qidx, count in self.blames[qidx].reasons.items():
            print(f"\t{self.qidx_to_name[reason_qidx]}: {count}")
        print("successors:")
        for child in self.graph.successors(qidx):
            print(f"\t{self.qidx_to_name[child]}: {self.graph[qidx][child]['weight']}")

if __name__ == "__main__":
    p = TheirParser(sys.argv[1])
    a = TheirAnalysis(p)
    a.debug_name("prelude_eucmod")
