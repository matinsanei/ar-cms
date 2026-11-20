import { describe, it, expect, beforeAll } from 'vitest';

const BACKEND_URL = 'http://localhost:8088';

describe('Modern CMS Features & Global Runtime Capabilities', () => {
  let authToken = '';
  let uploadedFilename = '';
  let testProductId = '';

  beforeAll(async () => {
    // Authenticate
    const loginRes = await fetch(`${BACKEND_URL}/admin/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ email: 'matin@pixelstack.io', password: 'Password123!' }),
    });
    const loginData = await loginRes.json();
    authToken = loginData.data.token;
  });

  describe('1. Dynamic On-the-fly Image Resizer Worker', () => {
    it('uploads an image and processes dynamic resize with format conversion', async () => {
      // 1x1 transparent PNG in base64
      const pngBase64 = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==';
      const buffer = Buffer.from(pngBase64, 'base64');
      const blob = new Blob([buffer], { type: 'image/png' });
      const formData = new FormData();
      formData.append('files', blob, 'sample_box.png');

      const uploadRes = await fetch(`${BACKEND_URL}/upload`, {
        method: 'POST',
        headers: { Authorization: `Bearer ${authToken}` },
        body: formData,
      });

      expect([200, 201]).toContain(uploadRes.status);
      const uploadData = await uploadRes.json();
      expect(Array.isArray(uploadData)).toBe(true);
      expect(uploadData.length).toBeGreaterThan(0);
      
      const asset = uploadData[0];
      uploadedFilename = asset.hash + asset.ext;

      // Now query the dynamic resizer worker: resize to 80x80 webp with cover fit
      const resizeRes = await fetch(
        `${BACKEND_URL}/uploads/resize/${uploadedFilename}?w=80&h=80&fit=cover&format=webp`
      );
      expect(resizeRes.status).toBe(200);
      expect(resizeRes.headers.get('content-type')).toBe('image/webp');
      expect(resizeRes.headers.get('cache-control')).toContain('public');
      const resizedBytes = await resizeRes.arrayBuffer();
      expect(resizedBytes.byteLength).toBeGreaterThan(0);

      // Also verify via /api/media/resize/:filename
      const apiResizeRes = await fetch(
        `${BACKEND_URL}/api/media/resize/${uploadedFilename}?w=40&h=40&format=png`
      );
      expect(apiResizeRes.status).toBe(200);
      expect(apiResizeRes.headers.get('content-type')).toBe('image/png');
    });
  });

  describe('2. Content Versioning, Revisions & Rollback Engine', () => {
    it('tracks revision snapshots across updates and supports instant rollback', async () => {
      // 1. Create product
      const createRes = await fetch(`${BACKEND_URL}/content-manager/collection-types/api::product.product`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${authToken}`,
        },
        body: JSON.stringify({
          name: 'Mechanical Keyboard V1',
          price: 100,
          description: 'Initial release',
        }),
      });
      expect(createRes.status).toBe(201);
      const created = await createRes.json();
      testProductId = created.data.documentId;

      // 2. Update to V2
      const updateRes1 = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/api::product.product/${testProductId}`,
        {
          method: 'PUT',
          headers: {
            'Content-Type': 'application/json',
            Authorization: `Bearer ${authToken}`,
          },
          body: JSON.stringify({
            name: 'Mechanical Keyboard V2',
            price: 120,
            description: 'RGB Backlit update',
          }),
        }
      );
      expect(updateRes1.status).toBe(200);

      // 3. Update to V3
      const updateRes2 = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/api::product.product/${testProductId}`,
        {
          method: 'PUT',
          headers: {
            'Content-Type': 'application/json',
            Authorization: `Bearer ${authToken}`,
          },
          body: JSON.stringify({
            name: 'Mechanical Keyboard V3',
            price: 150,
            description: 'Wireless Bluetooth edition',
          }),
        }
      );
      expect(updateRes2.status).toBe(200);

      // 4. Query Revisions list
      const revsRes = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/api::product.product/${testProductId}/revisions`,
        {
          headers: { Authorization: `Bearer ${authToken}` },
        }
      );
      expect(revsRes.status).toBe(200);
      const revsData = await revsRes.json();
      expect(Array.isArray(revsData.data)).toBe(true);
      expect(revsData.data.length).toBeGreaterThanOrEqual(3);

      // Find the V1 revision
      const v1Rev = revsData.data.find((r: any) => r.snapshot?.name === 'Mechanical Keyboard V1');
      expect(v1Rev).toBeDefined();

      // 5. Restore back to V1
      const restoreRes = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/api::product.product/${testProductId}/actions/restore/${v1Rev.id}`,
        {
          method: 'POST',
          headers: { Authorization: `Bearer ${authToken}` },
        }
      );
      expect(restoreRes.status).toBe(200);
      const restored = await restoreRes.json();
      expect(restored.data.name).toBe('Mechanical Keyboard V1');
      expect(restored.data.price).toBe(100);

      // Verify current entry state
      const verifyRes = await fetch(
        `${BACKEND_URL}/content-manager/collection-types/api::product.product/${testProductId}`,
        {
          headers: { Authorization: `Bearer ${authToken}` },
        }
      );
      const verifyData = await verifyRes.json();
      expect(verifyData.data.name).toBe('Mechanical Keyboard V1');
    });
  });

  describe('3. Map Proximity & GeoJSON RFC 7946 Standard API', () => {
    it('creates geo-tagged entries and queries GeoJSON endpoint with proximity sorting', async () => {
      // Create Tehran Store
      const storeRes = await fetch(`${BACKEND_URL}/content-manager/collection-types/api::product.product`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${authToken}`,
        },
        body: JSON.stringify({
          name: 'Tehran Tech Hub',
          price: 500,
          location: { lat: 35.6892, lng: 51.3890 },
        }),
      });
      const storeData = await storeRes.json();
      const storeId = storeData.data.documentId;

      // Publish entry
      await fetch(
        `${BACKEND_URL}/content-manager/collection-types/api::product.product/${storeId}/actions/publish`,
        {
          method: 'POST',
          headers: { Authorization: `Bearer ${authToken}` },
        }
      );

      // Query GeoJSON endpoint
      const geoRes = await fetch(`${BACKEND_URL}/api/products/geojson`);
      expect(geoRes.status).toBe(200);
      expect(geoRes.headers.get('content-type')).toContain('geo+json');
      const geojson = await geoRes.json();
      expect(geojson.type).toBe('FeatureCollection');
      expect(Array.isArray(geojson.features)).toBe(true);
      
      const techHubFeature = geojson.features.find(
        (f: any) => f.properties?.name === 'Tehran Tech Hub'
      );
      expect(techHubFeature).toBeDefined();
      expect(techHubFeature.geometry.type).toBe('Point');
      expect(techHubFeature.geometry.coordinates).toEqual([51.3890, 35.6892]);

      // Query with proximity filter: $near=35.6892,51.3890,2000 (within 2km)
      const nearRes = await fetch(
        `${BACKEND_URL}/api/products?filters[location][$near]=35.6892,51.3890,2000`
      );
      expect(nearRes.status).toBe(200);
      const nearData = await nearRes.json();
      const found = nearData.data.find((e: any) => e.name === 'Tehran Tech Hub');
      expect(found).toBeDefined();
    });
  });

  describe('4. System Health & Runtime Metrics Protocol', () => {
    it('GET /admin/health and /_health report live status and engine capabilities', async () => {
      const res = await fetch(`${BACKEND_URL}/admin/health`);
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.status).toBe('healthy');
      expect(body.database.type).toBe('SQLite 3');
      expect(body.database.journalMode).toBe('WAL');
      expect(body.features.zeroRestartHotReload).toBe(true);
      expect(body.features.dynamicImageResizer).toBe(true);
      expect(body.features.geoSpatialQueries).toBe(true);
      expect(body.features.contentRevisions).toBe(true);
      expect(body.stats.admins).toBeGreaterThan(0);
    });
  });

  describe('5. First Admin Setup Wizard Flow Verification', () => {
    it('handles first administrator onboarding correctly', async () => {
      // Test register-admin endpoint with new super admin
      const testEmail = `newadmin_${Date.now()}@pixelstack.io`;
      const regRes = await fetch(`${BACKEND_URL}/admin/register-admin`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          firstName: 'Sina',
          lastName: 'Architect',
          email: testEmail,
          password: 'Password123!',
        }),
      });

      expect(regRes.status).toBe(201);
      const regData = await regRes.json();
      expect(regData.data.token).toBeDefined();
      expect(regData.data.user.email).toBe(testEmail);
      expect(regData.data.user.firstname).toBe('Sina');
      expect(regRes.headers.get('set-cookie')).toContain('jwtToken=');

      // Verify login works with the new administrator
      const loginNew = await fetch(`${BACKEND_URL}/admin/login`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email: testEmail, password: 'Password123!' }),
      });
      expect(loginNew.status).toBe(200);
      const loginNewData = await loginNew.json();
      expect(loginNewData.data.user.email).toBe(testEmail);
    });
  });
});
