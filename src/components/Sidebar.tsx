import React from 'react';
import { NavLink } from 'react-router-dom';
import { 
  Layers, 
  FolderGit2, 
  Settings, 
  Database, 
  FileText, 
  PlusCircle, 
  Sparkles,
  ChevronRight,
  HardDrive
} from 'lucide-react';
import { ContentTypeSchema } from '../types/schema';

interface SidebarProps {
  contentTypes: Record<string, ContentTypeSchema>;
  onRefresh?: () => void;
}

export const Sidebar: React.FC<SidebarProps> = ({ contentTypes }) => {
  const collectionTypes = Object.values(contentTypes).filter(ct => ct.kind === 'collectionType');
  const singleTypes = Object.values(contentTypes).filter(ct => ct.kind === 'singleType');

  return (
    <aside className="w-64 bg-[#212134] text-white flex flex-col h-screen border-r border-[#181826] select-none flex-shrink-0">
      {/* Brand Header */}
      <div className="h-16 flex items-center px-6 gap-3 border-b border-[#2d2d44]">
        <div className="w-8 h-8 rounded-lg bg-gradient-to-tr from-[#4945ff] to-[#7b79ff] flex items-center justify-center shadow-lg shadow-indigo-500/30">
          <Sparkles className="w-5 h-5 text-white" />
        </div>
        <div>
          <h1 className="font-bold text-sm tracking-wide text-white">ZENITH CMS</h1>
          <p className="text-[10px] text-indigo-300 font-mono tracking-wider">UNIFIED RUNTIME</p>
        </div>
      </div>

      {/* Navigation Sections */}
      <div className="flex-1 overflow-y-auto py-4 px-3 space-y-6">
        {/* Core Plugins */}
        <div>
          <p className="px-3 text-[11px] font-semibold text-[#8e8ea9] uppercase tracking-wider mb-2">
            General
          </p>
          <nav className="space-y-1">
            <NavLink
              to="/content-manager"
              className={({ isActive }) =>
                `flex items-center gap-3 px-3 py-2 rounded-md text-sm font-medium transition-colors ${
                  isActive 
                    ? 'bg-[#4945ff] text-white shadow-sm' 
                    : 'text-[#c0c0cf] hover:bg-[#2d2d44] hover:text-white'
                }`
              }
            >
              <Layers className="w-4 h-4" />
              <span>Content Manager</span>
            </NavLink>

            <NavLink
              to="/content-type-builder"
              className={({ isActive }) =>
                `flex items-center gap-3 px-3 py-2 rounded-md text-sm font-medium transition-colors ${
                  isActive 
                    ? 'bg-[#4945ff] text-white shadow-sm' 
                    : 'text-[#c0c0cf] hover:bg-[#2d2d44] hover:text-white'
                }`
              }
            >
              <FolderGit2 className="w-4 h-4" />
              <span>Content-Type Builder</span>
            </NavLink>

            <NavLink
              to="/media-library"
              className={({ isActive }) =>
                `flex items-center gap-3 px-3 py-2 rounded-md text-sm font-medium transition-colors ${
                  isActive 
                    ? 'bg-[#4945ff] text-white shadow-sm' 
                    : 'text-[#c0c0cf] hover:bg-[#2d2d44] hover:text-white'
                }`
              }
            >
              <HardDrive className="w-4 h-4" />
              <span>Media Library</span>
            </NavLink>
          </nav>
        </div>

        {/* Collection Types Quick Jump */}
        <div>
          <div className="flex items-center justify-between px-3 mb-2">
            <p className="text-[11px] font-semibold text-[#8e8ea9] uppercase tracking-wider">
              Collection Types ({collectionTypes.length})
            </p>
            <NavLink 
              to="/content-type-builder/create" 
              className="text-[#8e8ea9] hover:text-white transition-colors"
              title="Add Collection Type"
            >
              <PlusCircle className="w-3.5 h-3.5" />
            </NavLink>
          </div>

          <nav className="space-y-1">
            {collectionTypes.map((ct) => (
              <NavLink
                key={ct.uid}
                to={`/content-manager/collection-types/${ct.uid}`}
                className={({ isActive }) =>
                  `flex items-center justify-between px-3 py-1.5 rounded-md text-xs font-medium transition-colors ${
                    isActive 
                      ? 'bg-[#2d2d44] text-[#4945ff] font-semibold' 
                      : 'text-[#a5a5ba] hover:bg-[#2a2a3e] hover:text-white'
                  }`
                }
              >
                <div className="flex items-center gap-2.5 truncate">
                  <div className="w-1.5 h-1.5 rounded-full bg-[#4945ff]" />
                  <span className="truncate">{ct.info.displayName}</span>
                </div>
                <ChevronRight className="w-3 h-3 text-[#666687]" />
              </NavLink>
            ))}

            {collectionTypes.length === 0 && (
              <p className="px-3 text-xs text-[#666687] italic">No collection types yet</p>
            )}
          </nav>
        </div>

        {/* Single Types Quick Jump */}
        {singleTypes.length > 0 && (
          <div>
            <p className="px-3 text-[11px] font-semibold text-[#8e8ea9] uppercase tracking-wider mb-2">
              Single Types ({singleTypes.length})
            </p>
            <nav className="space-y-1">
              {singleTypes.map((st) => (
                <NavLink
                  key={st.uid}
                  to={`/content-manager/single-types/${st.uid}`}
                  className={({ isActive }) =>
                    `flex items-center justify-between px-3 py-1.5 rounded-md text-xs font-medium transition-colors ${
                      isActive 
                        ? 'bg-[#2d2d44] text-[#4945ff] font-semibold' 
                        : 'text-[#a5a5ba] hover:bg-[#2a2a3e] hover:text-white'
                    }`
                  }
                >
                  <div className="flex items-center gap-2.5 truncate">
                    <FileText className="w-3.5 h-3.5 text-indigo-400" />
                    <span className="truncate">{st.info.displayName}</span>
                  </div>
                </NavLink>
              ))}
            </nav>
          </div>
        )}
      </div>

      {/* Footer Info */}
      <div className="p-4 border-t border-[#2d2d44] bg-[#181826] flex items-center justify-between text-xs text-[#8e8ea9]">
        <div className="flex items-center gap-2">
          <Database className="w-3.5 h-3.5 text-emerald-400" />
          <span className="font-mono text-[11px]">Rust Engine: 8088</span>
        </div>
        <span className="px-1.5 py-0.5 rounded text-[10px] bg-emerald-950 text-emerald-400 border border-emerald-800">
          ONLINE
        </span>
      </div>
    </aside>
  );
};
