import React, { useState, useEffect } from 'react';
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { Sidebar } from './components/Sidebar';
import { ContentTypeBuilder } from './pages/ContentTypeBuilder';
import { ContentManager } from './pages/ContentManager';
import { ContentTypeSchema } from './types/schema';
import { cmsService } from './services/api';

export const App: React.FC = () => {
  const [contentTypes, setContentTypes] = useState<Record<string, ContentTypeSchema>>({});
  const [loading, setLoading] = useState(true);

  const refreshSchema = async () => {
    try {
      const res = await cmsService.getSchema();
      setContentTypes(res.contentTypes || {});
    } catch (e) {
      console.error('Failed to load CMS schema from backend:', e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    refreshSchema();
  }, []);

  if (loading) {
    return (
      <div className="h-screen w-screen flex items-center justify-center bg-[#212134] text-white">
        <div className="text-center space-y-3">
          <div className="w-8 h-8 border-2 border-indigo-500 border-t-transparent rounded-full animate-spin mx-auto" />
          <p className="text-xs font-mono tracking-wider text-indigo-300">ZENITH ENGINE BOOTSTRAP...</p>
        </div>
      </div>
    );
  }

  return (
    <BrowserRouter>
      <div className="flex h-screen w-screen overflow-hidden font-sans">
        <Sidebar contentTypes={contentTypes} onRefresh={refreshSchema} />

        <Routes>
          <Route path="/" element={<Navigate to="/content-manager" replace />} />
          
          <Route 
            path="/content-type-builder/*" 
            element={<ContentTypeBuilder contentTypes={contentTypes} onRefresh={refreshSchema} />} 
          />

          <Route 
            path="/content-manager" 
            element={<ContentManager contentTypes={contentTypes} />} 
          />
          <Route 
            path="/content-manager/collection-types/:uid" 
            element={<ContentManager contentTypes={contentTypes} />} 
          />

          <Route 
            path="/media-library" 
            element={
              <div className="flex-1 p-8 bg-[#f6f6f9] flex items-center justify-center text-xs text-[#666687]">
                Media Library connected to MinIO bucket (Assets & Uploads).
              </div>
            } 
          />
        </Routes>
      </div>
    </BrowserRouter>
  );
};

export default App;
