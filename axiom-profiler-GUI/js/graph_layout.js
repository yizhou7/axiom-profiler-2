export function get_node_positions(dagDataJson) {
    // create appropriate structure from petgraph passed as JSON
    const dagData = JSON.parse(dagDataJson); 
    const data = convert(dagData);
    const build = d3.graphStratify();
    const graph = build(data);
    // layout graph using sugiyama style (layered graph drawing)
    const layout = d3.sugiyama();
    // return information to display graph 
    const { width, height } = layout(graph);
    const nodeArray = graph.nodes();
    const edgeArray = graph.links();
    return {
        width: width,
        height: height,
        nodeArray: nodeArray,
        edgeArray: edgeArray,
    };
}

function convert(input) {
    const edges = input.edges;
    const parentsOfNode = {};
    
    edges.forEach(edge => {
      const fromNode = edge[0].toString();
      const toNode = edge[1].toString();
  
      if (!parentsOfNode[fromNode]) {
        parentsOfNode[fromNode] = {
          id: fromNode,
          parentIds: []
        };
      }
  
      if (!parentsOfNode[toNode]) {
        parentsOfNode[toNode] = {
          id: toNode,
          parentIds: [fromNode]
        };
      } else {
        parentsOfNode[toNode].parentIds.push(fromNode);
      }
    });
    const result = Object.values(parentsOfNode);
    return result;
  }