import { describe, it, expect, beforeAll } from 'vitest';

const BACKEND_URL = 'http://localhost:8088';

describe('Deep Content Revisions, Snapshots & Rollback Suite', () => {
  let authToken = '';
  let docId = '';

  beforeAll(async () => {
    const loginRes = await fetch(`${BACKEND_URL}/admin/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ email: 'matin@pixelstack.io', password: 'Password123!' }),
    });
    const loginData = await loginRes.json();
    authToken = loginData.data.token;
  });

  describe('1. Sequential Version Tracking', () => {
    it('creates product entry and records initial Revision 1', async () => {
      const res = await fetch(`${BACKEND_URL}/content-manager/collection-types/api::product.product`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${authToken}`,
        },
        body: JSON.stringify({
          name: 'Cloud Compute Instance V1',
          price: 50,
          description: 'Basic 2 vCPU 4GB RAM',
        }),
      });

      expect([200, 201]).toContain(res.status);
      const data = await res.json();
      docId = data.data.documentId;
      expect(docId).toBeDefined();

      const revsRes = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/api::product.product/${docId}/revisions`,
        { headers: { Authorization: `Bearer ${authToken}` } }
      );
      expect(revsRes.status).toBe(200);
      const revsData = await revsRes.json();
      expect(revsData.data.length).toBeGreaterThanOrEqual(1);
      expect(revsData.data[0].snapshot.name).toBe('Cloud Compute Instance V1');
    });

    it('updates entry to V2 and records Revision 2', async () => {
      const res = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/api::product.product/${docId}`,
        {
          method: 'PUT',
          headers: {
            'Content-Type': 'application/json',
            Authorization: `Bearer ${authToken}`,
          },
          body: JSON.stringify({
            name: 'Cloud Compute Instance V2',
            price: 90,
            description: 'Upgraded to 4 vCPU 8GB RAM',
          }),
        }
      );
      expect(res.status).toBe(200);

      const revsRes = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/api::product.product/${docId}/revisions`,
        { headers: { Authorization: `Bearer ${authToken}` } }
      );
      const revsData = await revsRes.json();
      expect(revsData.data.length).toBeGreaterThanOrEqual(2);
    });

    it('updates entry to V3 and records Revision 3', async () => {
      const res = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/api::product.product/${docId}`,
        {
          method: 'PUT',
          headers: {
            'Content-Type': 'application/json',
            Authorization: `Bearer ${authToken}`,
          },
          body: JSON.stringify({
            name: 'Cloud Compute Instance V3 Enterprise',
            price: 180,
            description: 'Enterprise 8 vCPU 16GB RAM NVMe',
          }),
        }
      );
      expect(res.status).toBe(200);

      const revsRes = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/api::product.product/${docId}/revisions`,
        { headers: { Authorization: `Bearer ${authToken}` } }
      );
      const revsData = await revsRes.json();
      expect(revsData.data.length).toBeGreaterThanOrEqual(3);
    });
  });

  describe('2. Historical Snapshot Consistency & Rollback Operations', () => {
    it('verifies revisions list returns snapshots ordered by newest first', async () => {
      const revsRes = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/api::product.product/${docId}/revisions`,
        { headers: { Authorization: `Bearer ${authToken}` } }
      );
      expect(revsRes.status).toBe(200);
      const revsData = await revsRes.json();
      const versions = revsData.data.map((r: any) => r.version);
      for (let i = 1; i < versions.length; i++) {
        expect(versions[i - 1]).toBeGreaterThanOrEqual(versions[i]);
      }
    });

    it('restores entry state to Revision 1 (V1)', async () => {
      const revsRes = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/api::product.product/${docId}/revisions`,
        { headers: { Authorization: `Bearer ${authToken}` } }
      );
      const revsData = await revsRes.json();
      const v1Rev = revsData.data.find((r: any) => r.snapshot?.name === 'Cloud Compute Instance V1');
      expect(v1Rev).toBeDefined();

      const restoreRes = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/api::product.product/${docId}/actions/restore/${v1Rev.id}`,
        {
          method: 'POST',
          headers: { Authorization: `Bearer ${authToken}` },
        }
      );
      expect(restoreRes.status).toBe(200);
      const restored = await restoreRes.json();
      expect(restored.data.name).toBe('Cloud Compute Instance V1');
      expect(restored.data.price).toBe(50);
      expect(restored.data.description).toBe('Basic 2 vCPU 4GB RAM');

      // Verify via direct GET
      const getRes = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/api::product.product/${docId}`,
        { headers: { Authorization: `Bearer ${authToken}` } }
      );
      const getData = await getRes.json();
      expect(getData.data.name).toBe('Cloud Compute Instance V1');
    });

    it('restoring a revision appends a new revision snapshot marking the rollback', async () => {
      const revsRes = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/api::product.product/${docId}/revisions`,
        { headers: { Authorization: `Bearer ${authToken}` } }
      );
      const revsData = await revsRes.json();
      // Total revisions should have increased by 1
      expect(revsData.data.length).toBeGreaterThanOrEqual(4);
    });

    it('returns 404 when attempting to restore non-existent revision ID', async () => {
      const res = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/api::product.product/${docId}/actions/restore/99999999`,
        {
          method: 'POST',
          headers: { Authorization: `Bearer ${authToken}` },
        }
      );
      expect(res.status).toBe(404);
    });
  });
});
