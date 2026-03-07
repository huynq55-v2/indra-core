import { useState } from 'react';
import ForceGraph2D from 'react-force-graph-2d';

// Mock data array components (we will connect this to a real API endpoint later!)
const GraphViewer = () => {
  const [data] = useState({
    nodes: [
      { id: 'indra_core_genesis_node', name: 'System Core (IndraCore)', color: '#eab308', val: 5 }, // Yellow (System)
      { id: 'user_1', name: 'You', color: '#06b6d4', val: 3 }, // Cyan (User)
    ],
    links: [
      { source: 'user_1', target: 'indra_core_genesis_node', name: 'BECOMES_GENESIS' }
    ]
  });

  return (
    <div className="border border-slate-800 rounded-lg overflow-hidden bg-slate-900 w-full flex justify-center items-center relative" style={{ height: '500px' }}>
      <ForceGraph2D
        graphData={data}
        // Force graph will auto-fit if we don't fix the width
        nodeLabel="name"
        nodeColor={(node: any) => node.color}
        linkColor={() => 'rgba(255,255,255,0.2)'}
        linkDirectionalArrowLength={3.5}
        linkDirectionalArrowRelPos={1}
        backgroundColor="#0f172a" // match slate-900
      />
      <div className="absolute bottom-4 right-4 bg-slate-950/80 px-3 py-2 rounded border border-slate-800 text-xs text-slate-400">
        You can drag nodes to interact with them
      </div>
    </div>
  );
};

export default GraphViewer;
