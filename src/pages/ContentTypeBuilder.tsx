import React, { useState } from 'react';
import { 
  Plus, 
  Trash2, 
  Layers, 
  Sparkles, 
  Save, 
  ArrowLeft, 
  Check, 
  AlertCircle,
  FileCode,
  Type,
  AlignLeft,
  Hash,
  ToggleLeft,
  Calendar,
  Image,
  Link,
  Code2
} from 'lucide-react';
import { ContentTypeSchema, AttributeDefinition, FieldType } from '../types/schema';
import { cmsService } from '../services/api';

interface ContentTypeBuilderProps {
  contentTypes: Record<string, ContentTypeSchema>;
  onRefresh: () => void;
}

export const ContentTypeBuilder: React.FC<ContentTypeBuilderProps> = ({ contentTypes, onRefresh }) => {
  const [selectedUid, setSelectedUid] = useState<string>(Object.keys(contentTypes)[0] || '');
  const [isCreating, setIsCreating] = useState(false);
  const [newTypeName, setNewTypeName] = useState('');
  const [isSaving, setIsSaving] = useState(false);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);

  // Field modal state
  const [isFieldModalOpen, setIsFieldModalOpen] = useState(false);
  const [fieldName, setFieldName] = useState('');
  const [fieldType, setFieldType] = useState<FieldType>('string');
  const [fieldRequired, setFieldRequired] = useState(false);

  const selectedType = contentTypes[selectedUid];

  const handleCreateType = async () => {
    if (!newTypeName.trim()) return;
    setIsSaving(true);
    const singular = newTypeName.toLowerCase().replace(/\s+/g, '-');
    const uid = `api::${singular}.${singular}`;

    const newSchema: ContentTypeSchema = {
      uid,
      kind: 'collectionType',
      info: {
        singularName: singular,
        pluralName: `${singular}s`,
        displayName: newTypeName,
      },
      options: {
        draftAndPublish: true,
      },
      attributes: {
        title: { type: 'string', required: true },
      },
    };

    try {
      await cmsService.createContentType(newSchema);
      setSuccessMessage(`Content type "${newTypeName}" created and saved to disk instantly!`);
      setIsCreating(false);
      setNewTypeName('');
      onRefresh();
      setSelectedUid(uid);
      setTimeout(() => setSuccessMessage(null), 3000);
    } catch (e: any) {
      alert(`Error creating content type: ${e.message}`);
    } finally {
      setIsSaving(false);
    }
  };

  const handleAddField = async () => {
    if (!fieldName.trim() || !selectedType) return;
    setIsSaving(true);

    const updatedAttributes = {
      ...selectedType.attributes,
      [fieldName.trim()]: {
        type: fieldType,
        required: fieldRequired,
      } as AttributeDefinition,
    };

    const updatedSchema: ContentTypeSchema = {
      ...selectedType,
      attributes: updatedAttributes,
    };

    try {
      await cmsService.createContentType(updatedSchema);
      setSuccessMessage(`Field "${fieldName}" added in 2ms. No build, zero restart!`);
      setIsFieldModalOpen(false);
      setFieldName('');
      setFieldRequired(false);
      onRefresh();
      setTimeout(() => setSuccessMessage(null), 3000);
    } catch (e: any) {
      alert(`Error updating schema: ${e.message}`);
    } finally {
      setIsSaving(false);
    }
  };

  const handleDeleteField = async (attrKey: string) => {
    if (!selectedType || !confirm(`Delete field "${attrKey}"?`)) return;
    setIsSaving(true);

    const { [attrKey]: _, ...restAttributes } = selectedType.attributes;
    const updatedSchema: ContentTypeSchema = {
      ...selectedType,
      attributes: restAttributes,
    };

    try {
      await cmsService.createContentType(updatedSchema);
      setSuccessMessage(`Field "${attrKey}" removed.`);
      onRefresh();
      setTimeout(() => setSuccessMessage(null), 3000);
    } catch (e: any) {
      alert(`Error: ${e.message}`);
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div className="flex-1 flex overflow-hidden bg-[#f6f6f9]">
      {/* Types List Column */}
      <div className="w-80 border-r border-[#eaeaef] bg-white flex flex-col">
        <div className="p-4 border-b border-[#eaeaef] flex items-center justify-between">
          <h2 className="font-bold text-sm text-[#32324d] flex items-center gap-2">
            <Layers className="w-4 h-4 text-[#4945ff]" />
            Content Types
          </h2>
          <button
            onClick={() => setIsCreating(true)}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded bg-[#4945ff] text-white text-xs font-medium hover:bg-[#2f27fd] transition shadow-sm"
          >
            <Plus className="w-3.5 h-3.5" />
            Create
          </button>
        </div>

        <div className="flex-1 overflow-y-auto p-3 space-y-1">
          {Object.values(contentTypes).map((ct) => (
            <div
              key={ct.uid}
              onClick={() => {
                setSelectedUid(ct.uid);
                setIsCreating(false);
              }}
              className={`p-3 rounded-lg cursor-pointer border transition ${
                selectedUid === ct.uid && !isCreating
                  ? 'border-[#4945ff] bg-[#f0f0ff]'
                  : 'border-transparent hover:bg-[#f6f6f9]'
              }`}
            >
              <div className="flex items-center justify-between">
                <span className="font-semibold text-xs text-[#32324d]">{ct.info.displayName}</span>
                <span className="text-[10px] bg-slate-100 text-slate-600 px-1.5 py-0.5 rounded font-mono">
                  {Object.keys(ct.attributes).length} fields
                </span>
              </div>
              <p className="text-[11px] text-[#666687] font-mono truncate mt-0.5">{ct.uid}</p>
            </div>
          ))}
        </div>
      </div>

      {/* Main Content / Schema Editor */}
      <div className="flex-1 flex flex-col overflow-y-auto">
        {/* Banner notification */}
        {successMessage && (
          <div className="m-4 mb-0 p-3 bg-emerald-50 border border-emerald-200 rounded-lg flex items-center gap-2 text-emerald-800 text-xs shadow-sm animate-fade-in">
            <Check className="w-4 h-4 text-emerald-600" />
            <span>{successMessage}</span>
          </div>
        )}

        {isCreating ? (
          <div className="p-8 max-w-xl">
            <h2 className="text-xl font-bold text-[#32324d] mb-4">Create New Content Type</h2>
            <div className="space-y-4 bg-white p-6 rounded-xl border border-[#eaeaef] shadow-sm">
              <div>
                <label className="block text-xs font-semibold text-[#32324d] mb-1">Display Name</label>
                <input
                  type="text"
                  placeholder="e.g. Article, Hero Banner, Product"
                  value={newTypeName}
                  onChange={(e) => setNewTypeName(e.target.value)}
                  className="w-full px-3 py-2 text-sm border border-[#dcdce4] rounded-lg focus:outline-none focus:ring-2 focus:ring-[#4945ff]"
                />
              </div>

              <div className="flex gap-3 pt-2">
                <button
                  onClick={handleCreateType}
                  disabled={isSaving || !newTypeName.trim()}
                  className="px-4 py-2 bg-[#4945ff] text-white text-xs font-semibold rounded-lg hover:bg-[#2f27fd] transition disabled:opacity-50 flex items-center gap-2"
                >
                  <Save className="w-3.5 h-3.5" />
                  Save & Compile to Disk (Instant)
                </button>
                <button
                  onClick={() => setIsCreating(false)}
                  className="px-4 py-2 bg-slate-100 text-slate-700 text-xs font-semibold rounded-lg hover:bg-slate-200 transition"
                >
                  Cancel
                </button>
              </div>
            </div>
          </div>
        ) : selectedType ? (
          <div className="p-8 max-w-4xl">
            {/* Header info */}
            <div className="flex items-center justify-between mb-6 pb-4 border-b border-[#eaeaef]">
              <div>
                <h2 className="text-2xl font-bold text-[#32324d]">{selectedType.info.displayName}</h2>
                <p className="text-xs text-[#666687] font-mono mt-1">UID: {selectedType.uid}</p>
              </div>

              <button
                onClick={() => setIsFieldModalOpen(true)}
                className="px-4 py-2 bg-[#4945ff] text-white text-xs font-semibold rounded-lg hover:bg-[#2f27fd] transition flex items-center gap-2 shadow-sm"
              >
                <Plus className="w-4 h-4" />
                Add Another Field
              </button>
            </div>

            {/* Fields table */}
            <div className="bg-white rounded-xl border border-[#eaeaef] shadow-sm overflow-hidden">
              <table className="w-full text-left text-xs">
                <thead className="bg-[#f6f6f9] border-b border-[#eaeaef] text-[#666687] font-semibold uppercase tracking-wider text-[10px]">
                  <tr>
                    <th className="py-3 px-4">Field Name</th>
                    <th className="py-3 px-4">Type</th>
                    <th className="py-3 px-4">Required</th>
                    <th className="py-3 px-4 text-right">Actions</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-[#eaeaef]">
                  {Object.entries(selectedType.attributes).map(([attrName, attrDef]) => (
                    <tr key={attrName} className="hover:bg-slate-50/70 transition">
                      <td className="py-3.5 px-4 font-semibold text-[#32324d] flex items-center gap-2">
                        <span className="font-mono">{attrName}</span>
                      </td>
                      <td className="py-3.5 px-4">
                        <span className="px-2 py-0.5 rounded-full bg-indigo-50 text-indigo-700 border border-indigo-200 font-mono text-[11px]">
                          {attrDef.type}
                        </span>
                      </td>
                      <td className="py-3.5 px-4 text-[#666687]">
                        {attrDef.required ? (
                          <span className="text-amber-600 font-medium">Yes</span>
                        ) : (
                          'No'
                        )}
                      </td>
                      <td className="py-3.5 px-4 text-right">
                        <button
                          onClick={() => handleDeleteField(attrName)}
                          className="text-slate-400 hover:text-red-600 p-1 rounded transition"
                          title="Delete field"
                        >
                          <Trash2 className="w-3.5 h-3.5" />
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        ) : (
          <div className="flex-1 flex items-center justify-center text-xs text-[#666687]">
            Select or create a content type to begin.
          </div>
        )}
      </div>

      {/* Add Field Modal */}
      {isFieldModalOpen && (
        <div className="fixed inset-0 bg-black/40 backdrop-blur-sm z-50 flex items-center justify-center p-4">
          <div className="bg-white rounded-2xl shadow-2xl border border-[#eaeaef] max-w-lg w-full overflow-hidden animate-scale-up">
            <div className="p-6 border-b border-[#eaeaef]">
              <h3 className="text-base font-bold text-[#32324d]">Add New Field to Schema</h3>
              <p className="text-xs text-[#666687]">
                Changes persist instantly into the JSON file and hot-reloads the database schema.
              </p>
            </div>

            <div className="p-6 space-y-4">
              <div>
                <label className="block text-xs font-semibold text-[#32324d] mb-1">Field Name</label>
                <input
                  type="text"
                  placeholder="e.g. subtitle, price, description"
                  value={fieldName}
                  onChange={(e) => setFieldName(e.target.value)}
                  className="w-full px-3 py-2 text-sm border border-[#dcdce4] rounded-lg focus:outline-none focus:ring-2 focus:ring-[#4945ff]"
                />
              </div>

              <div>
                <label className="block text-xs font-semibold text-[#32324d] mb-1">Field Type</label>
                <div className="grid grid-cols-2 gap-2 max-h-48 overflow-y-auto p-1">
                  {[
                    { type: 'string', label: 'Short Text', desc: 'Titles, names' },
                    { type: 'text', label: 'Long Text', desc: 'Descriptions, articles' },
                    { type: 'richtext', label: 'Rich Text (Markdown/HTML)', desc: 'Formatted content' },
                    { type: 'integer', label: 'Number (Integer)', desc: 'Quantities, counters' },
                    { type: 'float', label: 'Number (Decimal/Float)', desc: 'Prices, coordinates' },
                    { type: 'boolean', label: 'Boolean (Switch)', desc: 'True/False toggles' },
                    { type: 'json', label: 'JSON Block', desc: 'Complex structured data' },
                    { type: 'media', label: 'Media (Image/Video)', desc: 'Files from MinIO/S3' },
                    { type: 'datetime', label: 'Date & Time', desc: 'Events, deadlines' },
                  ].map((f) => (
                    <div
                      key={f.type}
                      onClick={() => setFieldType(f.type as FieldType)}
                      className={`p-2.5 rounded-lg border text-left cursor-pointer transition ${
                        fieldType === f.type
                          ? 'border-[#4945ff] bg-[#f0f0ff]'
                          : 'border-[#eaeaef] hover:bg-slate-50'
                      }`}
                    >
                      <p className="font-semibold text-xs text-[#32324d]">{f.label}</p>
                      <p className="text-[10px] text-[#666687]">{f.desc}</p>
                    </div>
                  ))}
                </div>
              </div>

              <div className="flex items-center gap-2 pt-2">
                <input
                  type="checkbox"
                  id="requiredCheck"
                  checked={fieldRequired}
                  onChange={(e) => setFieldRequired(e.target.checked)}
                  className="rounded text-[#4945ff] focus:ring-[#4945ff]"
                />
                <label htmlFor="requiredCheck" className="text-xs text-[#32324d] font-medium cursor-pointer">
                  Required field
                </label>
              </div>
            </div>

            <div className="p-4 bg-[#f6f6f9] border-t border-[#eaeaef] flex justify-end gap-2">
              <button
                onClick={() => setIsFieldModalOpen(false)}
                className="px-4 py-2 bg-white border border-[#dcdce4] text-slate-700 text-xs font-semibold rounded-lg hover:bg-slate-50 transition"
              >
                Cancel
              </button>
              <button
                onClick={handleAddField}
                disabled={isSaving || !fieldName.trim()}
                className="px-4 py-2 bg-[#4945ff] text-white text-xs font-semibold rounded-lg hover:bg-[#2f27fd] transition disabled:opacity-50"
              >
                {isSaving ? 'Updating...' : 'Finish & Hot Reload'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
