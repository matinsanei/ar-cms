import axios from 'axios';
import { ContentTypeSchema, ContentEntry } from '../types/schema';

export const api = axios.create({
  baseURL: '',
  headers: {
    'Content-Type': 'application/json',
  },
});

export const cmsService = {
  // Schema endpoints
  async getSchema(): Promise<{ contentTypes: Record<string, ContentTypeSchema> }> {
    const res = await api.get('/content-type-builder/schema');
    return res.data;
  },

  async updateSchema(data: { contentTypes: ContentTypeSchema[] }): Promise<{ success: boolean }> {
    const res = await api.post('/content-type-builder/update-schema', { data });
    return res.data;
  },

  async createContentType(schema: ContentTypeSchema): Promise<ContentTypeSchema> {
    const res = await api.post('/content-type-builder/content-types', { contentType: schema });
    return res.data.data;
  },

  async deleteContentType(uid: string): Promise<void> {
    await api.delete(`/content-type-builder/content-types/${uid}`);
  },

  // Content entries endpoints
  async getEntries(uid: string, params?: any): Promise<{ data: ContentEntry[]; meta: any }> {
    const res = await api.get(`/content-manager/collection-types/${uid}`, { params });
    return res.data;
  },

  async getEntry(uid: string, id: string | number): Promise<{ data: ContentEntry }> {
    const res = await api.get(`/content-manager/collection-types/${uid}/${id}`);
    return res.data;
  },

  async createEntry(uid: string, data: any): Promise<{ data: ContentEntry }> {
    const res = await api.post(`/content-manager/collection-types/${uid}`, data);
    return res.data;
  },

  async updateEntry(uid: string, id: string | number, data: any): Promise<{ data: ContentEntry }> {
    const res = await api.put(`/content-manager/collection-types/${uid}/${id}`, data);
    return res.data;
  },

  async deleteEntry(uid: string, id: string | number): Promise<void> {
    await api.delete(`/content-manager/collection-types/${uid}/${id}`);
  },
};
