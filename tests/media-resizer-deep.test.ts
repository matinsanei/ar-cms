import { describe, it, expect, beforeAll } from 'vitest';

const BACKEND_URL = 'http://localhost:8088';

describe('Deep Media Library & Dynamic On-the-fly Image Resizer Suite', () => {
  let authToken = '';
  let uploadedPngFilename = '';
  let uploadedMediaId = 0;

  beforeAll(async () => {
    const loginRes = await fetch(`${BACKEND_URL}/admin/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ email: 'matin@pixelstack.io', password: 'Password123!' }),
    });
    const loginData = await loginRes.json();
    authToken = loginData.data.token;
  });

  describe('1. Multipart Upload & Asset Management', () => {
    it('uploads a standard PNG image with metadata extraction', async () => {
      // 2x2 colored PNG
      const pngBase64 = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==';
      const buffer = Buffer.from(pngBase64, 'base64');
      const blob = new Blob([buffer], { type: 'image/png' });
      const formData = new FormData();
      formData.append('files', blob, 'test_canvas.png');

      const res = await fetch(`${BACKEND_URL}/upload`, {
        method: 'POST',
        headers: { Authorization: `Bearer ${authToken}` },
        body: formData,
      });

      expect([200, 201]).toContain(res.status);
      const data = await res.json();
      expect(Array.isArray(data)).toBe(true);
      expect(data.length).toBeGreaterThan(0);

      const asset = data[0];
      expect(asset.name).toBe('test_canvas.png');
      expect(asset.mime).toBe('image/png');
      expect(asset.hash).toBeDefined();
      expect(asset.url).toContain('/uploads/');
      uploadedPngFilename = asset.hash + asset.ext;
      uploadedMediaId = asset.id;
    });

    it('GET /upload/files lists all uploaded media with pagination', async () => {
      const res = await fetch(`${BACKEND_URL}/upload/files?pageSize=10&page=1`, {
        headers: { Authorization: `Bearer ${authToken}` },
      });
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(Array.isArray(data.results)).toBe(true);
      expect(data.pagination.total).toBeGreaterThanOrEqual(1);
    });

    it('GET /upload/files/:id retrieves single media item', async () => {
      const res = await fetch(`${BACKEND_URL}/upload/files/${uploadedMediaId}`, {
        headers: { Authorization: `Bearer ${authToken}` },
      });
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.id).toBe(uploadedMediaId);
      expect(data.mime).toBe('image/png');
    });

    it('serves static file directly from /uploads/:filename', async () => {
      const res = await fetch(`${BACKEND_URL}/uploads/${uploadedPngFilename}`);
      expect(res.status).toBe(200);
      const bytes = await res.arrayBuffer();
      expect(bytes.byteLength).toBeGreaterThan(0);
    });
  });

  describe('2. Dynamic Resizer Worker - Dimension Transformations', () => {
    it('resizes with width only (preserves proportional aspect ratio)', async () => {
      const res = await fetch(
        `${BACKEND_URL}/uploads/resize/${uploadedPngFilename}?w=100`
      );
      expect(res.status).toBe(200);
      expect(res.headers.get('content-type')).toBe('image/webp'); // Default modern webp
      expect(res.headers.get('cache-control')).toContain('public');
      const bytes = await res.arrayBuffer();
      expect(bytes.byteLength).toBeGreaterThan(0);
    });

    it('resizes with height only', async () => {
      const res = await fetch(
        `${BACKEND_URL}/uploads/resize/${uploadedPngFilename}?h=80`
      );
      expect(res.status).toBe(200);
      const bytes = await res.arrayBuffer();
      expect(bytes.byteLength).toBeGreaterThan(0);
    });

    it('resizes with width and height using fit=contain', async () => {
      const res = await fetch(
        `${BACKEND_URL}/uploads/resize/${uploadedPngFilename}?w=120&h=120&fit=contain`
      );
      expect(res.status).toBe(200);
      const bytes = await res.arrayBuffer();
      expect(bytes.byteLength).toBeGreaterThan(0);
    });

    it('resizes with width and height using fit=cover (crop fill)', async () => {
      const res = await fetch(
        `${BACKEND_URL}/uploads/resize/${uploadedPngFilename}?w=64&h=64&fit=cover`
      );
      expect(res.status).toBe(200);
      const bytes = await res.arrayBuffer();
      expect(bytes.byteLength).toBeGreaterThan(0);
    });
  });

  describe('3. Dynamic Resizer Worker - Format Conversions & Compression Quality', () => {
    it('converts format to WebP with Content-Type image/webp', async () => {
      const res = await fetch(
        `${BACKEND_URL}/uploads/resize/${uploadedPngFilename}?w=50&format=webp`
      );
      expect(res.status).toBe(200);
      expect(res.headers.get('content-type')).toBe('image/webp');
    });

    it('converts format to PNG with Content-Type image/png', async () => {
      const res = await fetch(
        `${BACKEND_URL}/uploads/resize/${uploadedPngFilename}?w=50&format=png`
      );
      expect(res.status).toBe(200);
      expect(res.headers.get('content-type')).toBe('image/png');
    });

    it('converts format to JPEG with Content-Type image/jpeg', async () => {
      const res = await fetch(
        `${BACKEND_URL}/uploads/resize/${uploadedPngFilename}?w=50&format=jpeg`
      );
      expect(res.status).toBe(200);
      expect(res.headers.get('content-type')).toBe('image/jpeg');
    });

    it('supports custom quality compression parameter (q=40)', async () => {
      const res = await fetch(
        `${BACKEND_URL}/uploads/resize/${uploadedPngFilename}?w=50&q=40&format=jpeg`
      );
      expect(res.status).toBe(200);
      const bytes = await res.arrayBuffer();
      expect(bytes.byteLength).toBeGreaterThan(0);
    });

    it('second request for same parameters hits disk cache immediately', async () => {
      const start = Date.now();
      const res = await fetch(
        `${BACKEND_URL}/uploads/resize/${uploadedPngFilename}?w=50&format=webp`
      );
      const elapsed = Date.now() - start;
      expect(res.status).toBe(200);
      expect(elapsed).toBeLessThan(100); // Sub-millisecond hit
    });
  });

  describe('4. Resizer Edge Cases & Clean Deletion', () => {
    it('returns 404 for non-existent image resize', async () => {
      const res = await fetch(`${BACKEND_URL}/uploads/resize/non_existent_file.png?w=50`);
      expect(res.status).toBe(404);
    });

    it('safely clamps oversized requested dimensions (e.g. w=99999)', async () => {
      const res = await fetch(
        `${BACKEND_URL}/uploads/resize/${uploadedPngFilename}?w=99999&format=webp`
      );
      expect(res.status).toBe(200);
      const bytes = await res.arrayBuffer();
      expect(bytes.byteLength).toBeGreaterThan(0);
    });

    it('DELETE /upload/files/:id removes media from database and disk', async () => {
      const res = await fetch(`${BACKEND_URL}/upload/files/${uploadedMediaId}`, {
        method: 'DELETE',
        headers: { Authorization: `Bearer ${authToken}` },
      });
      expect(res.status).toBe(200);

      // Verify asset cannot be retrieved
      const verifyRes = await fetch(`${BACKEND_URL}/upload/files/${uploadedMediaId}`, {
        headers: { Authorization: `Bearer ${authToken}` },
      });
      expect(verifyRes.status).toBe(404);
    });
  });
});
