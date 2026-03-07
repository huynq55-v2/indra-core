import { useEffect, useState, useRef } from 'react';
import ForceGraph2D from 'react-force-graph-2d';
import axios from 'axios';

interface NeoNode {
  id: string;
  label: string;
  name: string;
}

interface NeoLink {
  source: string;
  target: string;
  label: string;
}

interface GraphData {
  nodes: NeoNode[];
  links: NeoLink[];
}

export default function GraphViewer({ userId }: { userId: string }) {
  const [data, setData] = useState<GraphData>({ nodes: [], links: [] });
  const [activeFilters, setActiveFilters] = useState<Record<string, boolean>>({});
  const graphRef = useRef<any>(null);

  useEffect(() => {
    const fetchGraph = async () => {
      try {
        const res = await axios.get(`http://localhost:3000/api/graph/user/${userId}`);
        setData(res.data);
      } catch (err) {
        console.error("Lỗi tải đồ thị:", err);
      }
    };
    if (userId) {
      fetchGraph();
    }
  }, [userId]);

  // Lọc Edge theo filter
  const filteredLinks = data.links.filter(link => activeFilters[link.label] !== false);
  
  // Lọc bỏ đi những Node mồ côi bị cô lập sau khi filter (Trừ node của chính User)
  const connectedNodeIds = new Set<string>([userId]);
  filteredLinks.forEach(l => {
    connectedNodeIds.add(typeof l.source === 'object' ? (l.source as any).id : l.source);
    connectedNodeIds.add(typeof l.target === 'object' ? (l.target as any).id : l.target);
  });
  
  const filteredNodes = data.nodes.filter(n => connectedNodeIds.has(n.id));

  const toggleFilter = (type: string) => {
    setActiveFilters(prev => ({...prev, [type]: prev[type] === false ? true : false}));
  };

  return (
    <div className="relative w-full h-full min-h-[500px] bg-slate-950 rounded-lg overflow-hidden">
      {/* Overlay Filter Panel */}
      <div className="absolute top-4 left-4 z-10 flex flex-wrap gap-2 max-w-sm">
        {Array.from(new Set(data.links.map(l => l.label))).map(fType => (
          <button 
            key={fType}
            onClick={() => toggleFilter(fType)}
            className={`text-[10px] font-bold px-2 py-1 rounded border transition-colors cursor-pointer ${
              activeFilters[fType] !== false
                ? 'bg-slate-800 border-slate-600 text-slate-200' 
                : 'bg-slate-900/50 border-slate-800 text-slate-600'
            }`}
          >
            {fType}
          </button>
        ))}
      </div>

      <ForceGraph2D
        ref={graphRef}
        graphData={{ nodes: filteredNodes, links: filteredLinks }}
        width={800}
        height={500}
        backgroundColor="#020617" // slate-950
        nodeRelSize={6}
        linkDirectionalArrowLength={3.5}
        linkDirectionalArrowRelPos={1}
        nodeCanvasObject={(node: any, ctx, globalScale) => {
          const label = node.name || node.id;
          const fontSize = 12 / globalScale;
          ctx.font = `${fontSize}px Sans-Serif`;

          // Thay đổi màu sắc theo loại Node
          if (node.id === userId) ctx.fillStyle = '#06b6d4'; // Cyan-500: Chính mình
          else if (node.label === 'System') ctx.fillStyle = '#eab308'; // Vàng: Core
          else if (node.label === 'InviteCode') ctx.fillStyle = '#10b981'; // Xanh lá: Mã mời
          else ctx.fillStyle = '#3b82f6'; // Blue: User khác

          ctx.beginPath();
          ctx.arc(node.x, node.y, 5, 0, 2 * Math.PI, false);
          ctx.fill();

          ctx.textAlign = 'center';
          ctx.textBaseline = 'middle';
          ctx.fillStyle = '#cbd5e1'; // text-slate-300
          ctx.fillText(label, node.x, node.y + 8);
        }}
        linkColor={(link: any) => {
          if (link.label === 'BECOMES_GENESIS') return '#eab308';
          if (link.label === 'INVITED_BY') return '#ec4899'; // Pink
          if (link.label === 'GENERATED') return '#10b981'; // Emerald
          if (link.label === 'USED_CODE') return '#6366f1'; // Indigo
          return '#475569';
        }}
        linkWidth={(link: any) => (link.label === 'BECOMES_GENESIS' || link.label === 'INVITED_BY') ? 2 : 1}
        linkLabel={(link: any) => link.label}
      />
    </div>
  );
}
