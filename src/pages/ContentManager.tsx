import React, { useState, useEffect } from 'react';
import { useParams } from 'react-router-dom';
import { 
  Plus, 
  Trash2, 
  Edit3, 
  Search, 
  Filter, 
  Check, 
  Calendar, 
  Eye, 
  Save, 
  X,
  FileText,
  Clock
} from 'lucide-react';
import { ContentTypeSchema, ContentEntry } from '../types/schema';
import { cmsService } from '../services/api';

interface ContentManagerProps {
  contentTypes: Record<string, ContentTypeSchema>;
}

export const ContentManager: React.FC<ContentManagerProps> = ({ contentTypes }) => {
  const { uid } = useParams<{ uid?: string }>();
  const activeUid = uid || Object.keys(contentTypes)[0] || '';
  const currentSchema = contentTypes[activeUid];

  const [entries, setEntries] = useState<ContentEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [editingEntry, setEditingEntry] = useState<ContentEntry | null>(null);
  const [isCreatingNew, setIsCreatingNew] = useState(false);
  const [formData, setFormData] = useState<Record<string, any>>({});
  const [successMsg, setSuccessMsg] = useState<string | null>(null);

  useEffect(() => {
    if (activeUid) {
      loadEntries();
      setEditingEntry(null);
      setIsCreatingNew(false);
    }
  }, [activeUid]);

  const loadEntries = async () => {
    if (!activeUid) return;
    setLoading(true);
    try {
      const res = await cmsService.getEntries(activeUid);
      setEntries(res.data || []);
    } catch (e: any) {
      console.error(e);
      setEntries([]);
    } finally {
      setLoading(false);
    }
  };

  const handleStartCreate = () => {
    setFormData({});
    setIsCreatingNew(true);
    setEditingEntry(null);
  };

  const handleStartEdit = (entry: ContentEntry) => {
    setFormData({ ...entry });
    setEditingEntry(entry);
    setIsCreatingNew(false);
  };

  const handleSave = async () => {
    if (!activeUid) return;
    try {
      if (isCreatingNew) {
        await cmsService.createEntry(activeUid, formData);
        setSuccessMsg('Entry created successfully.');
      } else if (editingEntry) {
        await cmsService.updateEntry(activeUid, editingEntry.id, formData);
        setSuccessMsg('Entry updated successfully.');
      }
      setIsCreatingNew(false);
      setEditingEntry(null);
      loadEntries();
      setTimeout(() => setSuccessMsg(null), 3000);
    } catch (e: any) {
      alert(`Error saving entry: ${e.message}`);
    }
  };

  const handleDelete = async (id: string | number) => {
    if (!confirm('Are you sure you want to delete this entry?')) return;
    try {
      await cmsService.deleteEntry(activeUid, id);
      setSuccessMsg('Entry deleted.');
      loadEntries();
      setTimeout(() => setSuccessMsg(null), 3000);
    } catch (e: any) {
      alert(`Error: ${e.message}`);
    }
  };

  if (!currentSchema) {
    return (
      <div className="flex-1 flex items-center justify-center text-sm text-[#666687]">
        No collection types available. Create one in Content-Type Builder.
      </div>
    );
  }

  const attributeKeys = Object.keys(currentSchema.attributes);

  return (
    <div className="flex-1 flex flex-col bg-[#f6f6f9] overflow-hidden">
      {/* Top Banner / Notification */}
      {successMsg && (
        <div className="m-4 mb-0 p-3 bg-emerald-50 border border-emerald-200 rounded-lg flex items-center gap-2 text-emerald-800 text-xs">
          <Check className="w-4 h-4 text-emerald-600" />
          <span>{successMsg}</span>
        </div>
      )}

      {/* Header bar */}
      <div className="bg-white border-b border-[#eaeaef] px-8 py-5 flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold text-[#32324d] flex items-center gap-2">
            <span>{currentSchema.info.displayName}</span>
            <span className="text-xs bg-slate-100 text-slate-600 px-2 py-0.5 rounded-full font-mono font-normal">
              {entries.length} entries
            </span>
          </h1>
          <p className="text-xs text-[#666687] mt-0.5">Manage and edit your collection content.</p>
        </div>

        {!isCreatingNew && !editingEntry && (
          <button
            onClick={handleStartCreate}
            className="flex items-center gap-2 px-4 py-2 bg-[#4945ff] text-white text-xs font-semibold rounded-lg hover:bg-[#2f27fd] transition shadow-sm"
          >
            <Plus className="w-4 h-4" />
            Create New Entry
          </button>
        )}
      </div>

      {/* Body: Either Table View OR Edit Form */}
      <div className="flex-1 overflow-y-auto p-8">
        {isCreatingNew || editingEntry ? (
          /* Dynamic Form View */
          <div className="max-w-3xl bg-white rounded-xl border border-[#eaeaef] shadow-sm p-6">
            <div className="flex items-center justify-between pb-4 border-b border-[#eaeaef] mb-6">
              <h2 className="text-base font-bold text-[#32324d]">
                {isCreatingNew ? 'Create an entry' : `Edit entry #${editingEntry?.id}`}
              </h2>
              <button
                onClick={() => {
                  setIsCreatingNew(false);
                  setEditingEntry(null);
                }}
                className="text-slate-400 hover:text-slate-600 p-1"
              >
                <X className="w-5 h-5" />
              </button>
            </div>

            <div className="space-y-5">
              {attributeKeys.map((key) => {
                const attr = currentSchema.attributes[key];
                return (
                  <div key={key}>
                    <label className="block text-xs font-semibold text-[#32324d] mb-1.5 flex items-center gap-1">
                      <span>{key}</span>
                      {attr.required && <span className="text-red-500">*</span>}
                      <span className="text-[10px] text-slate-400 font-normal">({attr.type})</span>
                    </label>

                    {attr.type === 'text' || attr.type === 'richtext' ? (
                      <textarea
                        rows={4}
                        value={formData[key] || ''}
                        onChange={(e) => setFormData({ ...formData, [key]: e.target.value })}
                        className="w-full px-3 py-2 text-xs border border-[#dcdce4] rounded-lg focus:outline-none focus:ring-2 focus:ring-[#4945ff]"
                        placeholder={`Enter ${key}...`}
                      />
                    ) : attr.type === 'boolean' ? (
                      <input
                        type="checkbox"
                        checked={!!formData[key]}
                        onChange={(e) => setFormData({ ...formData, [key]: e.target.checked })}
                        className="h-4 w-4 text-[#4945ff] rounded focus:ring-[#4945ff]"
                      />
                    ) : attr.type === 'integer' || attr.type === 'float' ? (
                      <input
                        type="number"
                        value={formData[key] ?? ''}
                        onChange={(e) => setFormData({ ...formData, [key]: Number(e.target.value) })}
                        className="w-full px-3 py-2 text-xs border border-[#dcdce4] rounded-lg focus:outline-none focus:ring-2 focus:ring-[#4945ff]"
                      />
                    ) : (
                      <input
                        type="text"
                        value={formData[key] || ''}
                        onChange={(e) => setFormData({ ...formData, [key]: e.target.value })}
                        className="w-full px-3 py-2 text-xs border border-[#dcdce4] rounded-lg focus:outline-none focus:ring-2 focus:ring-[#4945ff]"
                        placeholder={`Enter ${key}...`}
                      />
                    )}
                  </div>
                );
              })}
            </div>

            <div className="flex justify-end gap-3 pt-6 mt-6 border-t border-[#eaeaef]">
              <button
                onClick={() => {
                  setIsCreatingNew(false);
                  setEditingEntry(null);
                }}
                className="px-4 py-2 bg-slate-100 text-slate-700 text-xs font-semibold rounded-lg hover:bg-slate-200 transition"
              >
                Cancel
              </button>
              <button
                onClick={handleSave}
                className="px-5 py-2 bg-[#4945ff] text-white text-xs font-semibold rounded-lg hover:bg-[#2f27fd] transition flex items-center gap-2 shadow-sm"
              >
                <Save className="w-3.5 h-3.5" />
                Save & Publish
              </button>
            </div>
          </div>
        ) : (
          /* Table List View */
          <div className="bg-white rounded-xl border border-[#eaeaef] shadow-sm overflow-hidden">
            {loading ? (
              <div className="p-12 text-center text-xs text-[#666687] flex items-center justify-center gap-2">
                <Clock className="w-4 h-4 animate-spin text-[#4945ff]" />
                Loading entries from Rust engine...
              </div>
            ) : entries.length === 0 ? (
              <div className="p-12 text-center text-xs text-[#666687]">
                <FileText className="w-8 h-8 text-slate-300 mx-auto mb-2" />
                No content entries found. Click "Create New Entry" to add one.
              </div>
            ) : (
              <table className="w-full text-left text-xs">
                <thead className="bg-[#f6f6f9] border-b border-[#eaeaef] text-[#666687] font-semibold uppercase tracking-wider text-[10px]">
                  <tr>
                    <th className="py-3 px-4 w-16">ID</th>
                    {attributeKeys.slice(0, 4).map((k) => (
                      <th key={k} className="py-3 px-4">
                        {k}
                      </th>
                    ))}
                    <th className="py-3 px-4">Updated At</th>
                    <th className="py-3 px-4 text-right">Actions</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-[#eaeaef]">
                  {entries.map((entry) => (
                    <tr key={entry.id} className="hover:bg-slate-50/70 transition">
                      <td className="py-3.5 px-4 font-mono text-[#666687]">#{entry.id}</td>
                      {attributeKeys.slice(0, 4).map((k) => (
                        <td key={k} className="py-3.5 px-4 font-medium text-[#32324d] max-w-xs truncate">
                          {typeof entry[k] === 'object' ? JSON.stringify(entry[k]) : String(entry[k] || '-')}
                        </td>
                      ))}
                      <td className="py-3.5 px-4 text-[#666687] text-[11px] font-mono">
                        {new Date(entry.updatedAt).toLocaleDateString()}
                      </td>
                      <td className="py-3.5 px-4 text-right space-x-2">
                        <button
                          onClick={() => handleStartEdit(entry)}
                          className="text-slate-400 hover:text-[#4945ff] p-1 rounded transition"
                          title="Edit"
                        >
                          <Edit3 className="w-3.5 h-3.5" />
                        </button>
                        <button
                          onClick={() => handleDelete(entry.id)}
                          className="text-slate-400 hover:text-red-600 p-1 rounded transition"
                          title="Delete"
                        >
                          <Trash2 className="w-3.5 h-3.5" />
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        )}
      </div>
    </div>
  );
};
